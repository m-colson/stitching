use std::f32::consts::PI;

use serde::{Deserialize, Serialize};

mod img;
pub use img::ImageSpec;

mod proj;
pub use proj::{ProjSpec, ProjStyle};

#[cfg(feature = "live")]
pub mod live;
#[cfg(feature = "live")]
pub use live::{LiveBuffer, LiveSpec};

mod group;
pub use group::CameraGroupAsync;

mod util;

use crate::frame::{FrameBuffer, FrameBufferable, ToFrameBufferAsync};

#[derive(Clone, Debug)]
pub struct Camera<T, K> {
    pub spec: CameraSpec,
    pub ty: K,
    pub buf: T,
}

impl<T: FrameBufferable, K> Camera<T, K> {
    pub fn new(spec: CameraSpec, ty: K, buf: T) -> Self {
        Self { spec, ty, buf }
    }
}

impl<T: FrameBufferable + Default, K> Camera<T, K> {
    pub fn new_default(spec: CameraSpec, ty: K) -> Self {
        Self {
            spec,
            ty,
            buf: T::default(),
        }
    }
}

impl<T: FrameBuffer + Default, K> From<crate::config::CameraConfig<K>> for Camera<T, K> {
    fn from(value: crate::config::CameraConfig<K>) -> Self {
        Self::new_default(value.spec, value.ty)
    }
}

impl<T: FrameBuffer, K> Camera<T, K> {
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

impl<'a, T: ToFrameBufferAsync<'a>, K: Clone> Camera<T, K> {
    pub async fn to_frame_async(&'a self) -> Camera<T::Output, K> {
        Camera {
            spec: self.spec,
            ty: self.ty.clone(),
            buf: self.buf.to_frame_async().await,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CameraSpec {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    #[serde(with = "util::deg_rad")]
    pub pitch: f32,
    #[serde(with = "util::deg_rad")]
    pub azimuth: f32,
    #[serde(default, with = "util::deg_rad")]
    pub roll: f32,
    pub fov: CameraFov,
}

impl CameraSpec {
    pub fn set_dims(&mut self, w: f32, h: f32) {
        self.fov = self.fov.with_aspect(w, h);
    }

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

    pub fn radians(self) -> (f32, f32) {
        let Self::WHRadians(x, y) = self else {
            panic!("can't get radians of {self:?}");
        };
        (x, y)
    }
}
