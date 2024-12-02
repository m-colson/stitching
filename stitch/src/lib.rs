pub mod camera;

pub mod buf;

pub mod loader;

pub mod proj;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("io error while {1}: {0}")]
    IO(std::io::Error, String),

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

    #[cfg(feature = "live")]
    #[error("live err: {0}")]
    LiveErr(#[from] nokhwa::NokhwaError),

    #[cfg(feature = "gpu")]
    #[error("gpu error: {0}")]
    GpuError(#[from] smpgpu::Error),

    #[error("an option had the value of none, which shouldn't be possible")]
    UnexpectedNone,
}

impl Error {
    pub fn io_ctx(msg: String) -> impl FnOnce(std::io::Error) -> Self {
        move |err| Self::IO(err, msg)
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
