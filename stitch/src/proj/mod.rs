use serde::{Deserialize, Serialize};

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
    Hemisphere { pos: [f32; 3], radius: f32 },
}

impl ProjectionStyle {
    #[must_use]
    pub const fn radius(self) -> f32 {
        match self {
            Self::RawCamera(_) => 100.0,
            Self::Hemisphere { radius, .. } => radius,
        }
    }
}
