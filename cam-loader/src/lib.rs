//! This crate contains a simple, universal frame loader for live cameras that is compatible with v4l,
//! libargus, and others.

#![warn(missing_docs)]

use thiserror::Error;

mod adapter;
mod buf;
pub mod loader;
pub mod util;

pub use adapter::*;
pub use buf::FrameSize;
pub use loader::*;

/// Wrapper for any error types that could happen in this crate.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum Error {
    /// Error that occurs when a loader isn't responding but was already given
    /// a buffer.
    #[error("loader failed to accept or return buffer")]
    BufferLost,

    #[error("io error {0:?}")]
    IO(#[from] std::io::Error),

    #[error("io error {0:?} while {1}")]
    IOWhen(std::io::Error, String),

    #[cfg(feature = "argus")]
    #[error(transparent)]
    ArgusError(#[from] argus::Error),

    #[cfg(feature = "image")]
    #[error(transparent)]
    ImageError(#[from] image::error::ImageError),

    #[error("error: {0}")]
    Other(String),
}

impl Error {
    /// Creates a closure that will construct an [`Error::IOWhen`] with the given
    /// message. Useful as syntactic sugar inside of a [`Result::map_err`] call.
    pub fn io_ctx(msg: impl AsRef<str>) -> impl FnOnce(std::io::Error) -> Self {
        move |err| Self::IOWhen(err, msg.as_ref().to_string())
    }
}

/// [`std::result::Result`] alias for functions that return crate [`enum@Error`]s.
pub type Result<T, E = Error> = std::result::Result<T, E>;
