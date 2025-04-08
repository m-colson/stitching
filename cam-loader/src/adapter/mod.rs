use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Error, Loader, OwnedWriteBuffer, Result};

#[cfg(feature = "argus")]
mod argus;
#[cfg(feature = "image")]
mod image;
mod pattern;
#[cfg(feature = "v4l")]
mod v4l;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub mode: Mode,
    pub mask_path: Option<PathBuf>,
    pub resolution: [u32; 2],
    pub frame_rate: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Pattern {
        color: [u8; 3],
        #[serde(default = "default_grid_size")]
        grid_size: u32,
    },
    #[cfg(feature = "v4l")]
    #[serde(rename = "v4l")]
    V4L(v4l::Config),
    #[cfg(feature = "argus")]
    Argus(argus::Config),
    #[cfg(feature = "image")]
    Image(PathBuf),
}

fn default_grid_size() -> u32 {
    16
}

impl<B: OwnedWriteBuffer + Send + 'static> TryFrom<Config> for Loader<B> {
    type Error = Error;

    fn try_from(spec: Config) -> Result<Self> {
        match spec.mode {
            Mode::Pattern { color, grid_size } => pattern::from_spec(&spec, color, grid_size),
            #[cfg(feature = "v4l")]
            Mode::V4L(ref cfg) => v4l::from_spec(&spec, cfg.clone()),
            #[cfg(feature = "argus")]
            Mode::Argus(ref cfg) => Ok(argus::from_spec(&spec, cfg.clone())),
            #[cfg(feature = "image")]
            Mode::Image(ref path) => image::from_spec(&spec, &path),
        }
    }
}
