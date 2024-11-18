use std::{
    f32::consts::PI,
    ops::{Deref, DerefMut},
};

use crate::{
    camera::{CameraLens, CameraSpec},
    frame::FrameBufferView,
    loader::{FrameLoaderTicket, OwnedWriteBuffer},
    Camera, FrameBuffer, FrameBufferMut,
};

use super::{
    util::{clamp_pi, SphereCoord},
    FetchProjector, ProjStyle, Projector, UnitProjector,
};

#[derive(Clone, Copy)]
pub struct CpuProjector(Option<(usize, usize)>);

impl CpuProjector {
    pub const fn none() -> Self {
        Self(None)
    }

    pub const fn sized(width: usize, height: usize) -> Self {
        Self(Some((width, height)))
    }
}

impl Projector for CpuProjector {
    type ForwProj = CpuForwProj;
    type LoadResult = ();

    #[inline]
    fn load_forw(&self, style: ProjStyle, spec: CameraSpec, forw_proj: &mut Self::ForwProj) {
        let width = forw_proj.width;
        let height = forw_proj.height;
        forw_proj
            .chunks_mut(width)
            .enumerate()
            .for_each(|(sy, row)| {
                row.iter_mut().enumerate().for_each(move |(sx, p)| {
                    let sx = sx as f32 / width as f32 - 0.5;
                    let sy = sy as f32 / height as f32 - 0.5;
                    *p = self.screen_to_world(style, spec, sx, sy)
                })
            });
    }

    #[inline]
    fn load_back<F: FrameBuffer, K>(
        &self,
        fp: &Self::ForwProj,
        cams: &[Camera<F, K>],
        buf: &mut impl FrameBufferMut,
    ) {
        buf.pixel_iter_mut().zip(fp.deref()).for_each(|(p, ip)| {
            // p.copy_from_slice(&[0, 1, 2, 3]);
            // p.sort_by_cached_key(|i| {
            //     let spec = cams[*i as usize].spec;
            //     ((xi - spec.x).powi(2) + (yi - spec.y).powi(2) + (zi - spec.z).powi(2)) as i32
            // });

            let Some(best_p) = cams.iter().find_map(|c| {
                let (cx, cy) = self.world_to_img_space(c.spec, *ip);
                c.at(cx, cy)
            }) else {
                return;
            };
            copy_pixel_into(best_p, p);
        });
    }

    fn new_forw(&self) -> Self::ForwProj {
        let (width, height) = self
            .0
            .expect("can't create forward projection with missing sizes");
        Self::ForwProj::new(width, height)
    }
}

impl<B: AsRef<[u8]> + OwnedWriteBuffer + 'static> FetchProjector<B, B> for CpuProjector {
    type Ticket = FrameLoaderTicket<B>;

    type OutBuf<'a> = FrameBufferView<'a>;

    fn begin_fetch<K>(
        &self,
        cams: &mut [Camera<crate::loader::LoadingBuffer<B, B>, K>],
    ) -> Vec<Self::Ticket> {
        cams.iter_mut()
            .map(|c| {
                c.buf
                    .begin_load()
                    .expect("failed to take frame buffer, probably an implementation bug")
            })
            .collect()
    }

    async fn finish_fetch<'a, K>(
        &self,
        cams: &'a mut [Camera<crate::loader::LoadingBuffer<B, B>, K>],
        tickets: Vec<Self::Ticket>,
    ) -> Vec<Camera<Self::OutBuf<'a>>> {
        futures::future::join_all(cams.iter_mut().zip(tickets).map(|(c, ticket)| async {
            c.buf.attach(ticket).await;
            c.to_frame_buf()
        }))
        .await
    }
}

impl UnitProjector for CpuProjector {
    #[inline]
    fn world_to_cam_space(&self, spec: CameraSpec, (xi, yi, zi): (f32, f32, f32)) -> (f32, f32) {
        let (revx, revy, revz) = (xi - spec.x, yi - spec.y, zi - spec.z);
        // // dot product to ensure points are only projected when the are semi parallel to the camera.
        // if (xi * revx + yi * revy + zi * revz) < 0. {
        //     return None;
        // }

        let rev = SphereCoord::from_cart(revx, revy, revz);
        (clamp_pi(rev.theta - spec.azimuth), rev.phi - spec.pitch)
    }

    #[inline]
    fn cam_to_img_space(&self, spec: CameraSpec, (rev_az, rev_pitch): (f32, f32)) -> (f32, f32) {
        match spec.lens {
            CameraLens::Rectilinear => {
                let loc_mag = (rev_az.powi(2) + rev_pitch.powi(2)).sqrt();
                let loc_dir = rev_pitch.atan2(rev_az) - spec.roll;

                let azimuth = loc_mag * loc_dir.cos();
                let pitch = loc_mag * loc_dir.sin();

                let (fx, fy) = spec.fov.radians();
                (azimuth / fx, -pitch / fy)
            }
            CameraLens::Equidistant => {
                let loc_mag = (rev_az.powi(2) + rev_pitch.powi(2)).sqrt();
                let loc_dir = rev_pitch.atan2(rev_az) - spec.roll;

                let azimuth = loc_mag * loc_dir.cos();
                let pitch = loc_mag * loc_dir.sin();

                let (fx, fy) = spec.fov.radians();
                (azimuth.atan() / fx.atan(), -pitch.atan() / fy.atan())
            }
        }
    }

    #[inline]
    fn screen_to_world(
        &self,
        style: ProjStyle,
        spec: CameraSpec,
        sx: f32,
        sy: f32,
    ) -> (f32, f32, f32) {
        match style {
            ProjStyle::Spherical {
                radius, rev_face, ..
            } => {
                let (fx, fy) = spec.fov.radians();

                let (bound_az, bound_pitch) = if rev_face {
                    (
                        clamp_pi(spec.azimuth + fx * sx + PI),
                        clamp_pi(-spec.pitch - fy * sy + PI),
                    )
                } else {
                    (
                        clamp_pi(spec.azimuth + fx * sx),
                        clamp_pi(spec.pitch + fy * sy),
                    )
                };

                SphereCoord::new(radius, bound_az, bound_pitch).to_cart()
            }
            ProjStyle::Hemisphere { radius } => {
                let r = radius;
                let (fx, fy) = spec.fov.radians();

                let bound_pitch = clamp_pi(spec.pitch - fy * sy);

                let (z, mag_xy) = {
                    if bound_pitch.abs() < 1e-4 {
                        (spec.z, (r.powi(2) - spec.z.powi(2)).sqrt())
                    } else {
                        let p_cot = bound_pitch.cos() / bound_pitch.sin();
                        let cam_xy = (spec.x.powi(2) + spec.y.powi(2)).sqrt();
                        let xy_plane_dist = -spec.z * p_cot + cam_xy;

                        if xy_plane_dist > 0. && xy_plane_dist < r {
                            (0., xy_plane_dist)
                        } else {
                            let p2 = p_cot.powi(2);
                            let p2_1 = p2 + 1.;

                            let det_sqrt =
                                (r.powi(2) * p2_1 - (cam_xy - spec.z * p_cot).powi(2)).sqrt();

                            let z = (bound_pitch.signum() * det_sqrt - p_cot * cam_xy
                                + p2 * spec.z)
                                / p2_1;

                            (z, (r.powi(2) - z.powi(2)).sqrt())
                        }
                    }
                };

                let bound_az = spec.azimuth + fx * sx;
                (mag_xy * bound_az.sin(), mag_xy * bound_az.cos(), z)
            }
        }
    }
}

#[inline]
fn copy_pixel_into(src: &[u8], dest: &mut [u8]) {
    match (src.len(), dest.len()) {
        (1, 2) => {
            dest[0] = src[0];
            dest[1] = 255;
        }
        (3, 4) => {
            dest[0] = src[0];
            dest[1] = src[1];
            dest[2] = src[2];
            dest[3] = 255;
        }
        _ => dest.copy_from_slice(src),
    }
}

pub struct CpuForwProj<D: Deref<Target = [(f32, f32, f32)]> = Vec<(f32, f32, f32)>> {
    width: usize,
    height: usize,
    inner: D,
}

impl CpuForwProj {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            inner: vec![(0., 0., 0.); width * height],
        }
    }
}

impl<D: DerefMut<Target = [(f32, f32, f32)]>> CpuForwProj<D> {}

impl<D: Deref<Target = [(f32, f32, f32)]>> Deref for CpuForwProj<D> {
    type Target = [(f32, f32, f32)];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<D: DerefMut<Target = [(f32, f32, f32)]>> DerefMut for CpuForwProj<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
