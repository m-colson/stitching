use serde::{Deserialize, Serialize};

use crate::util::conv_deg_rad;

#[cfg(feature = "gpu")]
mod render_gpu;
#[cfg(feature = "gpu")]
pub use render_gpu::{GpuDirectBufferWrite, GpuProjector};

use crate::camera;
#[cfg(feature = "live")]
use crate::camera::live;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config<C> {
    pub style: ProjectionStyle,
    pub view: ViewStyle,
    pub cameras: Vec<camera::Config<C>>,
}

#[cfg(feature = "live")]
impl Config<live::Config> {
    /// # Errors
    /// path can't be read or decoded
    #[cfg(feature = "toml-cfg")]
    pub fn open(p: impl AsRef<std::path::Path>) -> crate::Result<Self> {
        toml::from_str::<Self>(
            &std::fs::read_to_string(&p)
                .map_err(crate::Error::io_ctx(format!("reading {:?}", p.as_ref())))?,
        )
        .map_err(From::from)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStyle {
    RawCamera(u8),
    Flat,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewStyle {
    Orthographic {
        pos: [f32; 3],
        radius: f32,
    },
    Perspective {
        pos: [f32; 3],
        #[serde(default)]
        look_at: [f32; 3],
        #[serde(with = "conv_deg_rad")]
        fov_y: f32,
    },
    Orbit {
        dist: f32,
        z: f32,
        #[serde(default)]
        theta: f32,
        #[serde(default)]
        look_at: [f32; 3],
        #[serde(with = "conv_deg_rad")]
        fov_y: f32,
        frame_per_rev: f32,
    },
}
