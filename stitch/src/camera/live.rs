use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    loader::{Loader, OwnedWriteBuffer},
    Error, Result,
};

mod argus_conv;
mod mmap_v4l;
mod pattern;

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
    #[serde(rename = "v4l")]
    V4L(mmap_v4l::Config),
    Argus(argus_conv::Config),
}

fn default_grid_size() -> u32 {
    16
}

impl<B: OwnedWriteBuffer + 'static> TryFrom<Config> for Loader<B> {
    type Error = Error;

    fn try_from(spec: Config) -> Result<Self> {
        match spec.mode {
            Mode::Pattern { color, grid_size } => pattern::from_spec(&spec, color, grid_size),
            Mode::V4L(ref cfg) => mmap_v4l::from_spec(&spec, cfg.clone()),
            Mode::Argus(ref cfg) => Ok(argus_conv::from_spec(&spec, cfg.clone())),
        }
    }
}
