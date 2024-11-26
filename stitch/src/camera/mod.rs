use std::{f32::consts::PI, future::Future};

use serde::{Deserialize, Serialize};

mod img;
pub use img::ImageSpec;

#[cfg(feature = "live")]
pub mod live;
#[cfg(feature = "live")]
pub use live::{live_camera_loader, LiveSpec};

mod group;
pub use group::{CameraGroup, CameraGroupAsync};

use crate::frame::{FrameBuffer, ToFrameBuffer, ToFrameBufferAsync};

#[derive(Clone, Debug)]
pub struct Camera<T, K = ()> {
    pub spec: CameraSpec,
    pub meta: K,
    pub buf: T,
}

impl<T, K> Camera<T, K> {
    #[inline]
    pub fn new(spec: CameraSpec, meta: K, buf: T) -> Self {
        Self { spec, meta, buf }
    }

    #[inline]
    pub fn map<N>(self, f: impl FnOnce(T) -> N) -> Camera<N> {
        Camera {
            spec: self.spec,
            meta: (),
            buf: f(self.buf),
        }
    }

    #[inline]
    pub fn map_with_meta<N>(self, f: impl FnOnce(T) -> N) -> Camera<N, K>
    where
        K: Clone,
    {
        Camera {
            spec: self.spec,
            meta: self.meta.clone(),
            buf: f(self.buf),
        }
    }

    #[inline]
    pub fn with_map<N>(&self, f: impl FnOnce(&T) -> N) -> Camera<N> {
        Camera {
            spec: self.spec,
            meta: (),
            buf: f(&self.buf),
        }
    }

    #[inline]
    pub async fn with_map_fut<'a, N, Fut: Future<Output = N>>(
        &'a self,
        f: impl FnOnce(&'a T) -> Fut,
    ) -> Camera<N> {
        Camera {
            spec: self.spec,
            meta: (),
            buf: f(&self.buf).await,
        }
    }
}

impl<T: Default, K> Camera<T, K> {
    #[inline]
    pub fn new_default(spec: CameraSpec, meta: K) -> Self {
        Self {
            spec,
            meta,
            buf: T::default(),
        }
    }
}

impl<T: FrameBuffer + Default, K> From<crate::config::CameraConfig<K>> for Camera<T, K> {
    #[inline]
    fn from(value: crate::config::CameraConfig<K>) -> Self {
        Self::new_default(value.spec, value.meta)
    }
}

impl<T: FrameBuffer, K> Camera<T, K> {
    #[inline]
    pub fn at(&self, x: f32, y: f32) -> Option<&[u8]> {
        let x = x + 0.5;
        let y = y + 0.5;
        if !(0.0..1.).contains(&x) || !(0.0..1.).contains(&y) {
            return None;
        }

        let width = self.width();
        let height = self.height();
        let chans = self.chans();

        let sx = (x * width as f32) as usize;
        let sy = (y * height as f32) as usize;

        // if let Some(mask) = mask {
        //     if mask.get_pixel(sx, sy).0[0] == 0 {
        //         return None;
        //     }
        // }

        Some(&self.buf.as_bytes()[(sx + (sy * width)) * chans..][..chans])
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.buf.width()
    }
    #[inline]
    pub fn height(&self) -> usize {
        self.buf.height()
    }
    #[inline]
    pub fn chans(&self) -> usize {
        self.buf.chans()
    }
}

impl<'a, T: ToFrameBuffer<'a>, K> Camera<T, K> {
    #[inline]
    pub fn to_frame_buf(&'a self) -> Camera<T::Output, ()> {
        Camera {
            spec: self.spec,
            meta: (),
            buf: self.buf.to_frame_buf(),
        }
    }
}

impl<'a, T: ToFrameBufferAsync<'a>, K> Camera<T, K> {
    pub async fn to_frame_async(&'a self) -> Camera<T::Output> {
        Camera {
            spec: self.spec,
            meta: (),
            buf: self.buf.to_frame_async().await,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CameraSpec {
    pub pos: [f32; 3],
    #[serde(with = "conv_deg_rad")]
    pub pitch: f32,
    #[serde(with = "conv_deg_rad")]
    pub azimuth: f32,
    #[serde(default, with = "conv_deg_rad")]
    pub roll: f32,
    #[serde(default)]
    pub img_off: [f32; 2],
    pub fov: CameraFov,
    #[serde(default)]
    pub lens: CameraLens,
}

mod conv_deg_rad {
    use serde::{Deserialize, Deserializer, Serializer};

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

impl CameraSpec {
    #[inline]
    pub fn set_dims(&mut self, w: f32, h: f32) {
        self.fov = self.fov.with_aspect(w, h);
    }

    #[inline]
    pub fn with_dims(mut self, w: f32, h: f32) -> Self {
        self.set_dims(w, h);
        self
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CameraFov {
    W(f32),
    H(f32),
    D(f32),
    WHRadians(f32, f32),
    Full,
}

impl CameraFov {
    #[inline]
    pub fn with_aspect(self, width: f32, height: f32) -> Self {
        match self {
            CameraFov::W(fw) => {
                CameraFov::WHRadians(fw.to_radians(), (fw * height / width).to_radians())
            }
            CameraFov::H(fh) => {
                CameraFov::WHRadians((fh * width / height).to_radians(), fh.to_radians())
            }
            CameraFov::D(fd) => {
                let fw = (fd.powi(2) / (1. + (height / width).powi(2))).sqrt();
                CameraFov::WHRadians(fw.to_radians(), (fw * height / width).to_radians())
            }
            CameraFov::WHRadians(_, _) => self,
            CameraFov::Full => CameraFov::WHRadians(2. * PI, PI / 2.),
        }
    }

    #[inline]
    pub fn radians(self) -> (f32, f32) {
        let Self::WHRadians(x, y) = self else {
            panic!("can't get radians of {self:?}");
        };
        (x, y)
    }

    #[inline]
    pub fn diag_radians(self) -> f32 {
        let (fx, fy) = self.radians();
        (fx * fx + fy * fy).sqrt()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CameraLens {
    #[default]
    Rectilinear,
    Equidistant,
}
