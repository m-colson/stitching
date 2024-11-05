pub mod camera;
pub use camera::{Camera, CameraFov};

pub mod config;
pub use config::{CameraConfig, CameraType, Config, ConfigError};

pub mod frame;
pub use frame::{FrameBuffer, SizedFrameBuffer, StaticFrameBuffer};

pub mod grad;

#[derive(Debug)]
pub struct RenderState<P: FrameBuffer> {
    pub proj: Camera<P>,
    pub cams: Vec<Camera<SizedFrameBuffer>>,
}

impl<P: FrameBuffer + Sync> RenderState<P> {
    pub fn update_proj(&mut self) {
        self.proj.buf.as_bytes_mut().fill(0);
        self.proj.project_into(&self.cams);
    }
}
