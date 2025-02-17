pub mod buf;
pub mod camera;
pub mod loader;
pub mod proj;
pub(crate) mod util;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error)]
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
    Dims(#[from] DimError),

    #[error(transparent)]
    IntOOB(#[from] std::num::TryFromIntError),

    #[error("loader failed to accept or return buffer")]
    BufferLost,

    #[cfg(feature = "toml-cfg")]
    #[error("decode error: {0}")]
    DecodeError(#[from] toml::de::Error),

    #[error(transparent)]
    ArgusError(#[from] argus::Error),

    #[cfg(feature = "gpu")]
    #[error("gpu error: {0}")]
    GpuError(#[from] smpgpu::Error),

    #[error("encountered a None value, which shouldn't have been possible")]
    UnexpectedNone,

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn io_ctx(msg: impl AsRef<str>) -> impl FnOnce(std::io::Error) -> Self {
        move |err| Self::IOWhen(err, msg.as_ref().to_string())
    }
}

#[derive(thiserror::Error, Debug)]
#[error("{kind} mismatch: {exp} != {got}")]
pub struct DimError {
    pub kind: DimErrorKind,
    pub exp: usize,
    pub got: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum DimErrorKind {
    Width,
    Height,
    Channel,
    Bytes,
}

impl std::fmt::Display for DimErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Width => write!(f, "width"),
            Self::Height => write!(f, "height"),
            Self::Channel => write!(f, "channel"),
            Self::Bytes => write!(f, "bytes"),
        }
    }
}

impl DimErrorKind {
    #[must_use]
    pub const fn err(self, exp: usize, got: usize) -> DimError {
        DimError {
            kind: self,
            exp,
            got,
        }
    }

    /// # Errors
    /// the `exp` value is different from the `got` value
    pub fn check(self, exp: usize, got: usize) -> std::result::Result<(), DimError> {
        (exp == got).then_some(()).ok_or_else(|| self.err(exp, got))
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
