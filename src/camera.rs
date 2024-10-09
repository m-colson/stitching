use std::{f32::consts::PI, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub pitch: f32,
    pub azimuth: f32,
    pub fov: CameraFov,
    pub ty: CameraType,
}

impl Camera {
    pub fn new((x, y, z): (f32, f32, f32), pitch: f32, azimuth: f32, fov: CameraFov) -> Self {
        Self {
            x,
            y,
            z,
            pitch: pitch.to_radians(),
            azimuth: azimuth.to_radians(),
            fov,
            ty: CameraType::None,
        }
    }

    pub fn with_image(mut self, path: impl AsRef<Path>) -> Result<Self, CameraError> {
        self.load_image(path)?;
        Ok(self)
    }

    pub fn load_image(&mut self, path: impl AsRef<Path>) -> Result<(), CameraError> {
        let dyn_img = image::open(path)?;
        let rgb_img = dyn_img.to_rgb8();

        self.ty = CameraType::Image(
            image::ImageBuffer::from_raw(
                rgb_img.width(),
                rgb_img.height(),
                rgb_img.into_raw().into(),
            )
            .unwrap(),
        );
        self.fov = self
            .fov
            .with_aspect(dyn_img.width() as f32, dyn_img.height() as f32);
        Ok(())
    }

    pub fn project_settings(&mut self, style: ProjectionStyle, avg_colors: bool) {
        if self.azimuth < 0. {
            self.azimuth += 2. * PI;
        }
        if self.azimuth > 2. * PI {
            self.azimuth -= 2. * PI;
        }

        self.ty = CameraType::Projection { style, avg_colors };
    }

    pub fn project_into(&self, width: usize, height: usize, others: &[Self], out: &mut [u8]) {
        use rayon::prelude::*;

        let CameraType::Projection { style, avg_colors } = &self.ty else {
            panic!("can't project with a non projection camera");
        };

        let mut new_self = self.clone();
        new_self.fov = self.fov.with_aspect(width as f32, height as f32);

        out.par_chunks_mut(width * 3)
            .enumerate()
            .for_each(|(sy, row)| {
                row.chunks_mut(3).enumerate().for_each(|(sx, p)| {
                    let sx = sx as f32 / width as f32 - 0.5;
                    let sy = sy as f32 / height as f32 - 0.5;
                    let (xi, yi, zi) = style.proj_forw(&new_self, sx, sy);

                    if *avg_colors {
                        let mut c_sum = [0., 0., 0.];
                        let mut c_count = 0;

                        others
                            .iter()
                            .filter_map(|c| style.proj_back(c, xi, yi, zi))
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
                            .iter()
                            .filter_map(|c| {
                                let (dx, dy, dz) = (xi - c.x, yi - c.y, zi - c.z);
                                Some((dx * dx + dy * dy + dz * dz, style.proj_back(c, xi, yi, zi)?))
                            })
                            .min_by(|a, b| a.0.total_cmp(&b.0))
                        else {
                            return;
                        };

                        p.copy_from_slice(&best_p);
                    }
                });
            });
    }

    pub fn at(&self, x: f32, y: f32) -> Option<[u8; 3]> {
        let x = x + 0.5;
        let y = y + 0.5;
        if !(0.0..1.).contains(&x) || !(0.0..1.).contains(&y) {
            return None;
        }

        let CameraType::Image(img) = &self.ty else {
            panic!("can only use at on image type camera");
        };

        Some(
            img.get_pixel(
                (x * img.width() as f32) as u32,
                (y * img.height() as f32) as u32,
            )
            .0,
        )
    }
}

fn clamp_pi(v: f32) -> f32 {
    if v < 0. {
        let rots = (-v / (2. * PI)).round();
        v + rots * 2. * PI
    } else {
        let rots = (v / (2. * PI)).round();
        v - rots * 2. * PI
    }
}

#[derive(Clone, Debug)]
pub enum CameraType {
    None,
    Image(image::ImageBuffer<image::Rgb<u8>, Arc<[u8]>>),
    Projection {
        style: ProjectionStyle,
        avg_colors: bool,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum CameraError {
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("image cast ")]
    ImageCastFailure,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CameraFov {
    W(f32),
    H(f32),
    D(f32),
    WHRadians(f32, f32),
}

impl CameraFov {
    pub fn with_aspect(self, width: f32, height: f32) -> Self {
        match self {
            CameraFov::W(fw) => {
                CameraFov::WHRadians(fw.to_radians(), (fw * height / width).to_radians())
            }
            CameraFov::H(fh) => {
                CameraFov::WHRadians((fh * width / height).to_radians(), fh.to_radians())
            }
            CameraFov::D(fd) => {
                let fw = (fd.powi(2) / (1. + (height / width).powi(2))).sqrt();
                CameraFov::WHRadians(fw.to_radians(), (fw * height / width).to_radians())
            }
            CameraFov::WHRadians(_, _) => self,
        }
    }

    pub fn radians(self) -> (f32, f32) {
        let Self::WHRadians(x, y) = self else {
            panic!("can't get radians of CameraFov::W or CameraFov::H");
        };
        (x, y)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProjectionStyle {
    Spherical {
        radius: f32,
        rev_face: bool,
        tan_correction: bool,
        dist_correction: bool,
    },
}

impl ProjectionStyle {
    pub fn proj_forw(&self, cam: &Camera, sx: f32, sy: f32) -> (f32, f32, f32) {
        match self {
            ProjectionStyle::Spherical {
                radius, rev_face, ..
            } => {
                let (fx, fy) = cam.fov.radians();

                let (bound_az, bound_pitch) = if *rev_face {
                    (
                        clamp_pi(cam.azimuth + fx * sx + PI),
                        clamp_pi(-cam.pitch - fy * sy + PI),
                    )
                } else {
                    (
                        clamp_pi(cam.azimuth + fx * sx),
                        clamp_pi(cam.pitch + fy * sy),
                    )
                };

                SphereCoord::new(*radius, bound_az, bound_pitch).to_cart()
            }
        }
    }

    pub fn proj_back(&self, c: &Camera, xi: f32, yi: f32, zi: f32) -> Option<[u8; 3]> {
        match self {
            ProjectionStyle::Spherical {
                radius,
                tan_correction,
                dist_correction,
                ..
            } => {
                let (revx, revy, revz) = (xi - c.x, yi - c.y, zi - c.z);
                let rev = SphereCoord::from_cart(revx, revy, revz);

                if xi * revx + yi * revy + zi * revz < 0. {
                    return None;
                }

                let azimuth = clamp_pi(rev.theta - c.azimuth);
                let pitch = rev.phi - c.pitch;

                let (fx, fy) = c.fov.radians();
                let (mut cx, mut cy) = if *tan_correction {
                    let (ax, ay) = ((fx / 2.).tan(), (fy / 2.).tan());
                    (0.5 * azimuth.tan() / ax, 0.5 * pitch.tan() / ay)
                } else {
                    (azimuth / fx, pitch / fy)
                };

                if *dist_correction {
                    let factor = rev.r / *radius;
                    cx *= factor;
                    cy *= factor;
                }

                c.at(cx, cy)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SphereCoord {
    pub r: f32,
    pub theta: f32,
    pub phi: f32,
}

impl SphereCoord {
    pub fn new(r: f32, theta: f32, phi: f32) -> Self {
        Self { r, theta, phi }
    }

    pub fn from_cart(x: f32, y: f32, z: f32) -> Self {
        let r = (x * x + y * y + z * z).sqrt();
        let theta = x.atan2(y);
        let phi = z.atan2((x * x + y * y).sqrt());
        Self { r, theta, phi }
    }

    pub fn to_cart(self) -> (f32, f32, f32) {
        let (x, y) = self.theta.sin_cos();
        let (z, m) = self.phi.sin_cos();

        (self.r * x * m, self.r * y * m, self.r * z)
    }
}
