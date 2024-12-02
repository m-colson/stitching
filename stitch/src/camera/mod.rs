use std::future::Future;

use serde::{Deserialize, Serialize};

#[cfg(feature = "live")]
pub mod live;

use crate::{
    buf::FrameSize,
    loader::{Loader, OwnedWriteBuffer},
};

#[derive(Clone, Debug)]
pub struct Camera<T> {
    pub view: ViewParams,
    pub data: T,
}

impl<T> Camera<T> {
    #[inline]
    pub const fn new(view: ViewParams, data: T) -> Self {
        Self { view, data }
    }

    #[inline]
    pub fn with_map<N>(&self, f: impl FnOnce(&T) -> N) -> Camera<N> {
        Camera {
            view: self.view,
            data: f(&self.data),
        }
    }

    #[allow(clippy::future_not_send)]
    #[inline]
    pub async fn with_map_fut<'a, N, Fut: Future<Output = N>>(
        &'a self,
        f: impl FnOnce(&'a T) -> Fut,
    ) -> Camera<N> {
        Camera {
            view: self.view,
            data: f(&self.data).await,
        }
    }
}

impl<T: FrameSize> Camera<T> {
    #[inline]
    pub fn width(&self) -> usize {
        self.data.width()
    }
    #[inline]
    pub fn height(&self) -> usize {
        self.data.height()
    }
    #[inline]
    pub fn chans(&self) -> usize {
        self.data.chans()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Config<K> {
    #[serde(flatten)]
    pub view: ViewParams,
    #[serde(flatten)]
    pub meta: K,
}

impl<K> Config<K> {
    #[must_use]
    pub fn with_dims(mut self, w: f32, h: f32) -> Self {
        self.view = self.view.with_dims(w, h);
        self
    }

    pub fn with_buffer<B>(self, buf: B) -> Camera<B> {
        Camera::new(self.view, buf)
    }
}

#[cfg(feature = "live")]
impl<T> Config<T> {
    /// # Errors
    /// conversion to loader fails
    pub fn load<B: OwnedWriteBuffer + 'static>(
        self,
    ) -> std::result::Result<Camera<Loader<B>>, T::Error>
    where
        T: TryInto<Loader<B>>,
    {
        let buf: Loader<_> = self.meta.try_into()?;
        let (w, h, _) = buf.frame_size();

        Ok(Camera::new(self.view.with_dims(w as f32, h as f32), buf))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ViewParams {
    pub pos: [f32; 3],
    #[serde(with = "conv_deg_rad")]
    pub pitch: f32,
    #[serde(with = "conv_deg_rad")]
    pub azimuth: f32,
    #[serde(default, with = "conv_deg_rad")]
    pub roll: f32,
    pub sensor: SensorParams,
    #[serde(default)]
    pub lens: LensKind,
}

mod conv_deg_rad {
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(v: &f32, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_f32(v.to_degrees())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<f32, D::Error>
    where
        D: Deserializer<'de>,
    {
        f32::deserialize(d).map(f32::to_radians)
    }
}

impl ViewParams {
    #[inline]
    pub fn set_dims(&mut self, w: f32, h: f32) {
        self.sensor.fov = self.sensor.fov.with_dims(self.lens, w, h);
    }

    #[must_use]
    #[inline]
    pub fn with_dims(mut self, w: f32, h: f32) -> Self {
        self.set_dims(w, h);
        self
    }

    #[must_use]
    #[inline]
    pub fn focal_dist(&self, width: f32, height: f32) -> f32 {
        self.sensor.fov.focal_dist(self.lens, width, height)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SensorParams {
    #[serde(default)]
    pub img_off: [f32; 2],
    pub fov: Fov,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Fov {
    W(f32),
    H(f32),
    D(f32),
    FocalDist(f32),
}

impl Fov {
    #[must_use]
    #[inline]
    pub fn with_dims(self, lens: LensKind, width: f32, height: f32) -> Self {
        Self::FocalDist(self.focal_dist(lens, width, height))
    }

    #[must_use]
    #[inline]
    pub fn focal_dist(self, lens: LensKind, width: f32, height: f32) -> f32 {
        let (r, ang) = match self {
            Self::W(f) => (width / width.hypot(height), f.to_radians() / 2.),
            Self::H(f) => (height / width.hypot(height), f.to_radians() / 2.),
            Self::D(f) => (1., f.to_radians() / 2.),
            Self::FocalDist(d) => return d,
        };

        lens.focal_from_rad_ang(r, ang)
    }

    #[must_use]
    #[inline]
    pub const fn assume_focal_dist(self) -> Option<f32> {
        if let Self::FocalDist(d) = self {
            Some(d)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum LensKind {
    #[default]
    Rectilinear = 0,
    Equidistant = 1,
    Equisolid = 2,
}

impl LensKind {
    #[must_use]
    #[inline]
    pub fn focal_from_rad_ang(self, r: f32, ang: f32) -> f32 {
        match self {
            Self::Rectilinear => r / ang.tan(),
            Self::Equidistant => r / ang,
            Self::Equisolid => r / (2. * (ang / 2.).sin()),
        }
    }
}
