pub mod camera;
pub use camera::{Camera, CameraFov, CameraGroupAsync, ImageSpec, ProjSpec};
#[cfg(feature = "live")]
pub use camera::{LiveBuffer, LiveSpec};

pub mod config;
pub use config::{CameraConfig, Config};

pub mod frame;
use frame::DimError;
pub use frame::{FrameBuffer, SizedFrameBuffer, StaticFrameBuffer};

pub mod grad;

#[cfg(feature = "tokio")]
pub mod sync_frame;

#[derive(Debug)]
pub struct RenderState<
    P: frame::FrameBufferable,
    C: frame::FrameBufferable = SizedFrameBuffer,
    S = ImageSpec,
> {
    pub proj: Camera<P, ProjSpec>,
    pub cams: Vec<Camera<C, S>>,
}

impl<P: FrameBuffer + Sync, C: FrameBuffer + Sync> RenderState<P, C> {
    pub fn update_proj(&mut self) {
        self.proj.buf.as_bytes_mut().fill(0);
        self.proj.load_projection(&self.cams);
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("io error while {1}: {0}")]
    IO(std::io::Error, String),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("image cast failed")]
    ImageCastFailure,

    #[error("{0}")]
    Dims(#[from] DimError),

    #[cfg(feature = "toml-cfg")]
    #[error("decode error: {0}")]
    DecodeError(#[from] toml::de::Error),

    #[cfg(feature = "watch")]
    #[error("watch err: {0}")]
    WatchErr(#[from] notify::Error),

    #[cfg(feature = "live")]
    #[error("live err: {0}")]
    LiveErr(#[from] nokhwa::NokhwaError),
}

impl Error {
    pub fn io_ctx(msg: String) -> impl FnOnce(std::io::Error) -> Self {
        move |err| Self::IO(err, msg)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
