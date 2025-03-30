use thiserror::Error;

mod adapter;
pub mod buf;
mod loader;
pub mod util;

pub use adapter::*;
pub use loader::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error("loader failed to accept or return buffer")]
    BufferLost,

    #[error("io error {0:?}")]
    IO(#[from] std::io::Error),

    #[error("io error {0:?} while {1}")]
    IOWhen(std::io::Error, String),

    #[cfg(feature = "argus")]
    #[error(transparent)]
    ArgusError(#[from] argus::Error),

    #[error("error: {0}")]
    Other(String),
}

impl Error {
    pub fn io_ctx(msg: impl AsRef<str>) -> impl FnOnce(std::io::Error) -> Self {
        move |err| Self::IOWhen(err, msg.as_ref().to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
