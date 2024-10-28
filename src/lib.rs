use camera::Camera;

pub mod camera;
pub mod config;

pub mod grad;

#[derive(Clone, Debug)]
pub struct RenderState {
    pub proj: Camera,
    pub cams: Vec<Camera>,
}
