use std::{f32::consts::PI, path::PathBuf};

use render_gpu::Vertex;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::util::conv_deg_rad;

#[cfg(feature = "gpu")]
mod render_gpu;
#[cfg(feature = "gpu")]
pub use render_gpu::{DepthData, GpuDirectBufferWrite, GpuProjector, InverseView, TexturedVertex};

use crate::camera;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config<C> {
    #[serde(default)]
    pub style: ProjectionStyle,
    pub world: WorldStyle,
    pub view: ViewStyle,
    pub model: Option<ModelConfig>,
    pub cameras: Vec<camera::Config<C>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    pub path: PathBuf,
    #[serde(default)]
    pub origin: [f32; 3],
    pub scale: Option<[f32; 3]>,
    pub rot: Option<[f32; 3]>,
    pub light_dir: Option<[f32; 3]>,
}

impl<C: DeserializeOwned> Config<C> {
    /// # Errors
    /// path can't be read or decoded
    #[cfg(feature = "toml-cfg")]
    pub fn open(p: impl AsRef<std::path::Path>) -> crate::Result<Self> {
        toml::from_str::<Self>(
            &std::fs::read_to_string(&p)
                .map_err(crate::Error::io_ctx(format!("reading {:?}", p.as_ref())))?,
        )
        .map_err(From::from)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStyle {
    RawCamera(u8),
    #[default]
    Normal,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewStyle {
    Orthographic {
        pos: [f32; 3],
        radius: f32,
    },
    Perspective {
        pos: [f32; 3],
        #[serde(default)]
        look_at: [f32; 3],
        #[serde(with = "conv_deg_rad")]
        fov_y: f32,
    },
    Orbit {
        dist: f32,
        z: f32,
        #[serde(default)]
        theta: f32,
        #[serde(default)]
        look_at: [f32; 3],
        #[serde(with = "conv_deg_rad")]
        fov_y: f32,
        frame_per_rev: f32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldStyle {
    Cylinder { radius: f32, height: Option<f32> },
    Plane { radius: f32 },
}

impl WorldStyle {
    pub fn make_mesh(&self) -> (Vec<Vertex>, Vec<u16>) {
        let mut verts = Vec::new();
        let mut idxs = Vec::new();
        match self {
            WorldStyle::Cylinder { radius, height } => {
                const SLICES: u16 = 20;
                let height = height.unwrap_or(80.0);

                verts.push(Vertex::new(0., 0., 0.));

                for n in 0..SLICES {
                    let (x, y) = (2. * PI * n as f32 / SLICES as f32).sin_cos();
                    let (x, y) = (x * radius, y * radius);
                    verts.extend([Vertex::new(x, y, 0.), Vertex::new(x, y, height)])
                }

                for n in 0..(SLICES - 1) {
                    let bn = n * 2 + 1;
                    idxs.extend([0, bn, bn + 2]);
                    idxs.extend([bn + 2, bn, bn + 1]);
                    idxs.extend([bn + 1, bn + 3, bn + 2]);
                }

                let last_bn = SLICES * 2 - 1;
                idxs.extend([0, last_bn, 1]);
                idxs.extend([1, last_bn, last_bn + 1]);
                idxs.extend([last_bn + 1, 2, 1]);
            }
            WorldStyle::Plane { radius } => {
                let r = *radius;
                verts.extend([
                    Vertex::new(-r, -r, 0.),
                    Vertex::new(-r, r, 0.),
                    Vertex::new(r, r, 0.),
                    Vertex::new(r, -r, 0.),
                ]);
                idxs.extend([0, 1, 2, 2, 3, 0]);
            }
        }

        (verts, idxs)
    }
}
