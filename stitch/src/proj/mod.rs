use std::future::Future;

use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use serde::{Deserialize, Serialize};

use crate::{
    camera::CameraSpec,
    loader::{LoadingBuffer, OwnedWriteBuffer},
    Camera, FrameBuffer, FrameBufferMut,
};

mod cpu;
pub use cpu::CpuProjector;

#[cfg(feature = "gpu")]
mod gpu;
#[cfg(feature = "gpu")]
pub use gpu::{GpuDirectBufferView, GpuDirectBufferWrite, GpuForwProj, GpuProjector};

mod util;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProjSpec {
    pub style: ProjStyle,
    pub avg_colors: bool,
}

impl<T: FrameBufferMut> Camera<T, ProjSpec>
where
    for<'a> &'a T: Send,
{
    pub fn load_projection<
        CB: FrameBuffer,
        K,
        O: AsRef<[Camera<CB, K>]> + Sync,
        J: UnitProjector + Sync,
    >(
        &mut self,
        proj: &J,
        others: O,
    ) {
        let ProjSpec { style, avg_colors } = self.meta;

        let width = self.buf.width();
        let height = self.buf.height();
        let chans = self.buf.chans();

        self.buf
            .as_bytes_mut()
            .par_chunks_mut(width * chans)
            .enumerate()
            .for_each(|(sy, row)| {
                row.chunks_mut(chans).enumerate().for_each(|(sx, p)| {
                    let sx = sx as f32 / width as f32 - 0.5;
                    let sy = sy as f32 / height as f32 - 0.5;
                    let int_p @ (xi, yi, zi) = proj.screen_to_world(style, self.spec, sx, sy);

                    if avg_colors {
                        let mut c_sum = [0f32, 0., 0.];
                        let mut c_count = 0;

                        others
                            .as_ref()
                            .iter()
                            .filter_map(|c| {
                                let (cx, cy) = proj.world_to_img_space(c.spec, int_p);
                                c.at(cx, cy)
                            })
                            .for_each(|p| {
                                c_sum[0] += (p[0] as f32).powi(2);
                                c_sum[1] += (p[1] as f32).powi(2);
                                c_sum[2] += (p[2] as f32).powi(2);
                                c_count += 1;
                            });

                        match c_count {
                            0 => {}
                            1 => p.copy_from_slice(&[
                                c_sum[0].sqrt() as u8,
                                c_sum[1].sqrt() as u8,
                                c_sum[2].sqrt() as u8,
                            ]),
                            n => p.copy_from_slice(&[
                                (c_sum[0] / n as f32).sqrt() as u8,
                                (c_sum[1] / n as f32).sqrt() as u8,
                                (c_sum[2] / n as f32).sqrt() as u8,
                            ]),
                        }
                    } else {
                        let Some((_, best_p)) = others
                            .as_ref()
                            .iter()
                            .filter_map(|c| {
                                let (dx, dy, dz) = (xi - c.spec.x, yi - c.spec.y, zi - c.spec.z);
                                let (cx, cy) = proj.world_to_img_space(c.spec, int_p);
                                Some((dx * dx + dy * dy + dz * dz, c.at(cx, cy)?))
                            })
                            .min_by(|a, b| a.0.total_cmp(&b.0))
                        else {
                            return;
                        };

                        p.copy_from_slice(best_p);
                    }
                });
            });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProjStyle {
    Spherical {
        radius: f32,
        rev_face: bool,
        tan_correction: bool,
        dist_correction: bool,
    },
    Hemisphere {
        radius: f32,
    },
}

pub trait UnitProjector {
    fn screen_to_world(
        &self,
        style: ProjStyle,
        spec: CameraSpec,
        sx: f32,
        sy: f32,
    ) -> (f32, f32, f32);

    fn world_to_cam_space(&self, spec: CameraSpec, xyzi: (f32, f32, f32)) -> (f32, f32);

    fn cam_to_img_space(&self, spec: CameraSpec, rev_az_pitch: (f32, f32)) -> (f32, f32);

    #[inline]
    fn world_to_img_space(&self, spec: CameraSpec, (xi, yi, zi): (f32, f32, f32)) -> (f32, f32) {
        self.cam_to_img_space(spec, self.world_to_cam_space(spec, (xi, yi, zi)))
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

    fn block_finish_fetch<'a, K>(
        &self,
        cams: &'a mut [Camera<LoadingBuffer<T, B>, K>],
        tickets: Vec<Self::Ticket>,
    ) -> Vec<Camera<Self::OutBuf<'a>>> {
        tokio::runtime::Handle::current().block_on(self.finish_fetch(cams, tickets))
    }

    fn finish_fetch<'a, K>(
        &self,
        cams: &'a mut [Camera<LoadingBuffer<T, B>, K>],
        tickets: Vec<Self::Ticket>,
    ) -> impl Future<Output = Vec<Camera<Self::OutBuf<'a>>>>;
}
