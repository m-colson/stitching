use std::path::PathBuf;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::util::conv_deg_rad;

#[cfg(feature = "gpu")]
mod render_gpu;
#[cfg(feature = "gpu")]
pub use render_gpu::{DepthData, GpuDirectBufferWrite, GpuProjector, InverseView, TexturedVertex};

use crate::camera;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config<C> {
    #[serde(default)]
    pub style: ProjectionStyle,
    pub view: ViewStyle,
    pub model: Option<ModelConfig>,
    pub cameras: Vec<camera::Config<C>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub path: PathBuf,
    #[serde(default)]
    pub origin: [f32; 3],
    pub scale: Option<[f32; 3]>,
}

impl<C: DeserializeOwned> Config<C> {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStyle {
    RawCamera(u8),
    #[default]
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
