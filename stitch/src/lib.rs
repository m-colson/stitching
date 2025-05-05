//! This crate contains the functionality required to perform stitching over
//! multiple cameras on the gpu.

#![warn(missing_docs)]

pub mod camera;
// pub mod loader;
pub mod proj;
pub(crate) mod util;

/// [`std::result::Result`] alias for functions that return crate [`Error`]s.
pub type Result<T> = std::result::Result<T, Error>;

/// Wrapper for any error types that could happen in this crate.
#[derive(thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("io error {0:?}")]
    IO(#[from] std::io::Error),

    #[error("io error {0:?} while {1}")]
    IOWhen(std::io::Error, String),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("image cast failed")]
    ImageCastFailure,

    #[error(transparent)]
    IntOOB(#[from] std::num::TryFromIntError),

    #[cfg(feature = "toml-cfg")]
    #[error("decode error: {0}")]
    DecodeError(#[from] toml::de::Error),

    #[error("loader error: {0}")]
    Loader(#[from] cam_loader::Error),

    #[cfg(feature = "gpu")]
    #[error("gpu error: {0}")]
    GpuError(#[from] smpgpu::Error),

    #[error("encountered a None value, which shouldn't have been possible")]
    UnexpectedNone,

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Creates a closure that will construct an [`Error::IOWhen`] with the given
    /// message. Useful as syntactic sugar inside of a [`Result::map_err`] call.
    pub fn io_ctx(msg: impl AsRef<str>) -> impl FnOnce(std::io::Error) -> Self {
        move |err| Self::IOWhen(err, msg.as_ref().to_string())
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
