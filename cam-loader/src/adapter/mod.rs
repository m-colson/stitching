use serde::{Deserialize, Serialize};

use crate::{Error, Loader, OwnedWriteBuffer, Result};

#[cfg(feature = "argus")]
pub mod argus;
#[cfg(feature = "image")]
pub mod image;
pub mod pattern;
#[cfg(feature = "v4l")]
pub mod v4l;

/// Contains basic settings to construct one of the loaders in this module.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// The kind of loader to use.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub mode: Mode,
    /// The loader's output resolution \[width, height\].
    pub resolution: [u32; 2],
    /// The framerate of the camera, only used in a few adapters.
    pub frame_rate: Option<u32>,
}

/// Wraps the possible adapter configurations based on the enabled feature flags.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Mode {
    /// [`pattern`] loader
    Pattern {
        /// the RGB color for the "on" part of the grid, the "off" part is always black.
        color: [u8; 3],
        /// number of pixels between "on" and "off" flips.
        grid_size: Option<u32>,
    },
    #[cfg(feature = "v4l")]
    /// [`v4l`] loader
    #[serde(rename = "v4l")]
    V4L(v4l::Config),
    /// [`argus`] loader
    #[cfg(feature = "argus")]
    Argus(argus::Config),
    /// static image loader
    #[cfg(feature = "image")]
    Image(PathBuf),
}

impl<B: OwnedWriteBuffer + Send + 'static> TryFrom<Config> for Loader<B> {
    type Error = Error;

    fn try_from(spec: Config) -> Result<Self> {
        const DEFAULT_GRID_SIZE: u32 = 16;
        match spec.mode {
            Mode::Pattern { color, grid_size } => {
                pattern::from_spec(&spec, color, grid_size.unwrap_or(DEFAULT_GRID_SIZE))
            }
            #[cfg(feature = "v4l")]
            Mode::V4L(ref cfg) => v4l::from_spec(&spec, cfg.clone()),
            #[cfg(feature = "argus")]
            Mode::Argus(ref cfg) => Ok(argus::from_spec(&spec, cfg.clone())),
            #[cfg(feature = "image")]
            Mode::Image(ref path) => image::from_spec(&spec, &path),
        }
    }
}
