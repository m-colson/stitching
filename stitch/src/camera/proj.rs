use std::{
    f32::consts::PI,
    ops::{Deref, DerefMut},
};

use itertools::Itertools;
use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use serde::{Deserialize, Serialize};

use crate::FrameBuffer;

use super::{
    util::{clamp_pi, SphereCoord},
    Camera, CameraSpec,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProjSpec {
    pub style: ProjStyle,
    pub avg_colors: bool,
}

impl<T: FrameBuffer> Camera<T, ProjSpec>
where
    for<'a> &'a T: Send,
{
    pub fn load_projection<CB: FrameBuffer, K, O: AsRef<[Camera<CB, K>]> + Sync>(
        &mut self,
        others: O,
    ) {
        let ProjSpec { style, avg_colors } = self.ty;

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
                    let (xi, yi, zi) = style.proj_forw(self.spec, sx, sy);

                    if avg_colors {
                        let mut c_sum = [0f32, 0., 0.];
                        let mut c_count = 0;

                        others
                            .as_ref()
                            .iter()
                            .filter_map(|c| style.proj_back(c, (xi, yi, zi)))
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
                                Some((
                                    dx * dx + dy * dy + dz * dz,
                                    style.proj_back(c, (xi, yi, zi))?,
                                ))
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

    // pub fn load_projection_pre_forw<CB: FrameBuffer, K, O: AsRef<[Camera<CB, K>]>>(
    //     &mut self,
    //     others: O,
    // ) {
    //     let ProjSpec {
    //         style,
    //         avg_colors: _,
    //     } = self.ty;

    //     let width = self.buf.width();
    //     let height = self.buf.height();
    //     let chans = self.buf.chans();

    //     let forws = style.forward_proj(self.spec, width, height);

    //     let others = others.as_ref();
    //     let cams = forws
    //         .iter()
    //         .flat_map(|&(xi, yi, zi)| {
    //             others
    //                 .iter()
    //                 .map(|c| {
    //                     let (dx, dy, dz) = (xi - c.spec.x, yi - c.spec.y, zi - c.spec.z);
    //                     (c, dx * dx + dy * dy + dz * dz)
    //                 })
    //                 .sorted_by(|(_, a), (_, b)| a.total_cmp(b))
    //                 .map(|(c, _)| c)
    //         })
    //         .collect::<Vec<_>>();

    //     self.buf
    //         .as_bytes_mut()
    //         .chunks_mut(chans)
    //         .zip(cams.chunks(others.len()).zip(&forws))
    //         .for_each(|(p, (cs, &(xi, yi, zi)))| {
    //             let Some(best_p) = cs.iter().find_map(|&c| style.proj_back(c, (xi, yi, zi))) else {
    //                 return;
    //             };
    //             p.copy_from_slice(best_p);
    //         });
    // }
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

impl ProjStyle {
    pub fn proj_forw(self, spec: CameraSpec, sx: f32, sy: f32) -> (f32, f32, f32) {
        match self {
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

                let (bound_az, bound_pitch) = (
                    clamp_pi(spec.azimuth + fx * sx),
                    clamp_pi(spec.pitch - fy * sy),
                );

                let (z, mag_xy) = {
                    let p_cot = bound_pitch.cos() / bound_pitch.sin();
                    let cam_xy = (spec.x.powi(2) + spec.y.powi(2)).sqrt();
                    let xy_plane_dist = -spec.z * p_cot + cam_xy;

                    if bound_pitch.abs() < 1e-4 {
                        (spec.z, (r.powi(2) - spec.z.powi(2)).sqrt())
                    } else if xy_plane_dist > 0. && xy_plane_dist < r {
                        (0., xy_plane_dist)
                    } else {
                        let p2 = p_cot.powi(2);
                        let p2_1 = p2 + 1.;

                        let det_sqrt =
                            (r.powi(2) * p2_1 - (cam_xy - spec.z * p_cot).powi(2)).sqrt();

                        let z =
                            (bound_pitch.signum() * det_sqrt - p_cot * cam_xy + p2 * spec.z) / p2_1;

                        (z, (r.powi(2) - z.powi(2)).sqrt())
                    }
                };

                (mag_xy * bound_az.sin(), mag_xy * bound_az.cos(), z)
            }
        }
    }

    pub fn proj_back<B: FrameBuffer, K>(
        self,
        cam: &Camera<B, K>,
        (xi, yi, zi): (f32, f32, f32),
    ) -> Option<&[u8]> {
        let spec = cam.spec;
        match self {
            ProjStyle::Spherical {
                radius,
                tan_correction,
                dist_correction,
                ..
            } => {
                let (revx, revy, revz) = (xi - spec.x, yi - spec.y, zi - spec.z);
                let rev = SphereCoord::from_cart(revx, revy, revz);

                if xi * revx + yi * revy + zi * revz < 0. {
                    return None;
                }

                let azimuth = clamp_pi(rev.theta - spec.azimuth);
                let pitch = rev.phi - spec.pitch;

                let (fx, fy) = spec.fov.radians();
                let (mut cx, mut cy) = if tan_correction {
                    let (ax, ay) = ((fx / 2.).tan(), (fy / 2.).tan());
                    (0.5 * azimuth.tan() / ax, 0.5 * pitch.tan() / ay)
                } else {
                    (azimuth / fx, pitch / fy)
                };

                if dist_correction {
                    let factor = rev.r / radius;
                    cx *= factor;
                    cy *= factor;
                }

                cam.at(cx, cy)
            }
            ProjStyle::Hemisphere { .. } => {
                let (revx, revy, revz) = (xi - spec.x, yi - spec.y, zi - spec.z);
                // dot product to ensure points are only projected when the are semi parallel to the camera.
                if (xi * revx + yi * revy + zi * revz) < 0. {
                    return None;
                }

                let rev = SphereCoord::from_cart(revx, revy, revz);

                let mut azimuth = clamp_pi(rev.theta - spec.azimuth);
                let mut pitch = rev.phi - spec.pitch;

                if spec.roll != 0. {
                    let loc_mag = (azimuth.powi(2) + pitch.powi(2)).sqrt();
                    let loc_dir = pitch.atan2(azimuth) - spec.roll;
                    azimuth = loc_mag * loc_dir.cos();
                    pitch = loc_mag * loc_dir.sin();
                }

                let (fx, fy) = spec.fov.radians();
                let (cx, cy) = (azimuth / fx, pitch / fy);

                cam.at(cx, -cy)
            }
        }
    }

    pub fn forward_proj(self, spec: CameraSpec, width: usize, height: usize) -> ForwardProj {
        (0..height)
            .flat_map(|sy| {
                (0..width).map(move |sx| {
                    let sx = sx as f32 / width as f32 - 0.5;
                    let sy = sy as f32 / height as f32 - 0.5;
                    self.proj_forw(spec, sx, sy)
                })
            })
            .collect()
    }
}

pub struct ForwardProj<D: Deref<Target = [(f32, f32, f32)]> = Vec<(f32, f32, f32)>>(D);

impl<D: Deref<Target = [(f32, f32, f32)]>> ForwardProj<D> {
    pub fn load_back<CB: FrameBuffer, K, O: AsRef<[Camera<CB, K>]>>(
        &self,
        style: ProjStyle,
        others: O,
        buf: &mut impl FrameBuffer,
    ) {
        let others = others.as_ref();
        let cams = self
            .iter()
            .flat_map(|&(xi, yi, zi)| {
                others
                    .iter()
                    .map(|c| {
                        let (dx, dy, dz) = (xi - c.spec.x, yi - c.spec.y, zi - c.spec.z);
                        (c, dx * dx + dy * dy + dz * dz)
                    })
                    .sorted_by(|(_, a), (_, b)| a.total_cmp(b))
                    .map(|(c, _)| c)
            })
            .collect::<Vec<_>>();

        let chans = buf.chans();
        buf.as_bytes_mut()
            .chunks_mut(chans)
            .zip(cams.chunks(others.len()).zip(self.deref()))
            .for_each(|(p, (cs, &(xi, yi, zi)))| {
                let Some(best_p) = cs.iter().find_map(|&c| style.proj_back(c, (xi, yi, zi))) else {
                    return;
                };
                p.copy_from_slice(best_p);
            });
    }
}

impl<D: FromIterator<(f32, f32, f32)> + Deref<Target = [(f32, f32, f32)]>>
    FromIterator<(f32, f32, f32)> for ForwardProj<D>
{
    fn from_iter<T: IntoIterator<Item = (f32, f32, f32)>>(iter: T) -> Self {
        ForwardProj(D::from_iter(iter))
    }
}

impl<D: Deref<Target = [(f32, f32, f32)]>> Deref for ForwardProj<D> {
    type Target = [(f32, f32, f32)];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: DerefMut<Target = [(f32, f32, f32)]>> DerefMut for ForwardProj<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// impl<
//         I: Iterator<Item = (f32, f32, f32)>,
//         D: IntoIterator<Item = (f32, f32, f32), IntoIter = I> + Deref<Target = [(f32, f32, f32)]>,
//     > IntoIterator for ForwardProj<D>
// {
//     type Item = (f32, f32, f32);
//     type IntoIter = I;

//     fn into_iter(self) -> Self::IntoIter {
//         self.0.into_iter()
//     }
// }

// impl<'a, I: Iterator<Item = &'a (f32, f32, f32)>, D: Deref<Target = [(f32, f32, f32)]>> IntoIterator
//     for &'a ForwardProj<D>
// where
//     &'a D: IntoIterator<IntoIter = I>,
// {
//     type Item = &'a (f32, f32, f32);
//     type IntoIter = I;

//     fn into_iter(self) -> Self::IntoIter {
//         self.0.into_iter()
//     }
// }
