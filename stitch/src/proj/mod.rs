use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::{
    camera::CameraSpec,
    loader::{LoadingBuffer, OwnedWriteBuffer},
    Camera, FrameBuffer, FrameBufferMut,
};

// #[cfg(feature = "gpu")]
// mod gpu;
// #[cfg(feature = "gpu")]
// pub use gpu::{GpuDirectBufferView, GpuDirectBufferWrite, GpuForwProj, GpuProjector};

#[cfg(feature = "gpu")]
mod render_gpu;
#[cfg(feature = "gpu")]
pub use render_gpu::{GpuDirectBufferView, GpuDirectBufferWrite, GpuForwProj, GpuProjector};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProjSpec {
    pub style: ProjStyle,
    pub avg_colors: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjStyle {
    RawCamera(u32),
    Hemisphere { radius: f32 },
}

impl ProjStyle {
    pub fn radius(self) -> f32 {
        match self {
            Self::RawCamera(_) => 100.0,
            Self::Hemisphere { radius } => radius,
        }
    }
}

pub trait Projector {
    type ForwProj;
    type LoadResult;

    fn load_forw(
        &self,
        style: ProjStyle,
        spec: CameraSpec,
        forw_proj: &mut Self::ForwProj,
    ) -> Self::LoadResult;

    fn load_back<F: FrameBuffer, K>(
        &self,
        fp: &Self::ForwProj,
        cams: &[Camera<F, K>],
        buf: &mut impl FrameBufferMut,
    ) -> Self::LoadResult;

    fn new_forw(&self) -> Self::ForwProj;
}

pub trait FetchProjector<T, B: OwnedWriteBuffer>: Projector {
    type Ticket;
    type OutBuf<'a>: FrameBuffer
    where
        B: 'a,
        T: 'a;

    fn begin_fetch<K>(&self, cams: &mut [Camera<LoadingBuffer<T, B>, K>]) -> Vec<Self::Ticket>;

    fn finish_fetch<'a, K>(
        &self,
        cams: &'a mut [Camera<LoadingBuffer<T, B>, K>],
        tickets: Vec<Self::Ticket>,
    ) -> impl Future<Output = Vec<Camera<Self::OutBuf<'a>>>>;
}
