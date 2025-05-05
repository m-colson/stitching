//! This module contains types and functions that represent and perform the image stitching process.

use std::{f32::consts::PI, path::PathBuf};

use cam_loader::Loader;
use glam::Mat4;
use render::Vertex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::util::conv_deg_rad;

#[cfg(feature = "gpu")]
mod render;
#[cfg(feature = "gpu")]
pub use render::{
    DepthData, GpuDirectBufferWrite, GpuProjector, GpuProjectorBuilder, ProjectionView,
    TexturedVertex,
};

use crate::camera;

/// Stores options used to configure a [`GpuProjector`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config<C> {
    /// The type of projection, currently only [`ProjectionStyle::Normal`].
    #[serde(default)]
    pub style: ProjectionStyle,
    /// The type of world (geometry that is projected onto) and associated options.
    pub world: WorldStyle,
    /// The position and type of camera view that will be used to render the image.
    pub view: ViewStyle,
    /// A 3d-model for some object that can be added into the rendering.
    pub model: Option<ModelConfig>,
    /// The list of cameras that images will be taken from.
    pub cameras: Vec<camera::Config<C>>,
}

/// Stores where a 3d-model is in the filesystem and any transformations
/// necessary to make it accurate to the world's scaling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelConfig {
    /// The path to an object file containing the model's geometry.
    pub path: PathBuf,
    #[serde(default)]
    /// The coordinate in the model that should be treated as \[0,0,0\].
    pub origin: [f32; 3],
    /// The factors to scale each dimension by or [1, 1, 1] if None.
    pub scale: Option<[f32; 3]>,
    /// The euler angles (ZXY) to rotate the model by in degrees.
    pub rot: Option<[f32; 3]>,
    /// A vector containing the direction of the model shading light. Automatically normalized.
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

/// Wrapper over [`cam_loader::Config`] that also includes the mask path.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MaskLoaderConfig {
    #[serde(flatten)]
    pub loader: cam_loader::Config,
    pub mask_path: Option<PathBuf>,
}

impl<B: cam_loader::OwnedWriteBuffer + Send + 'static> TryInto<Loader<B>> for MaskLoaderConfig {
    type Error = cam_loader::Error;
    fn try_into(self) -> Result<Loader<B>, Self::Error> {
        self.loader.try_into()
    }
}

/// Represents what the projector needs to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStyle {
    /// Perform the normal stitching process.
    #[default]
    Normal,
}

/// Represent where and how the viewpoint camera should be placed in the rendering.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewStyle {
    /// A top-down orthographic viewpoint.
    Orthographic {
        /// Location of the center of the view.
        pos: [f32; 3],
        /// Radial height (height / 2) of the view.
        radius: f32,
    },
    /// A perspective-based viewpoint
    Perspective {
        /// Location of the camera.
        pos: [f32; 3],
        #[serde(default)]
        /// Coordinate where the center of the camera is pointing.
        look_at: [f32; 3],
        /// FOV of the viewpoint vertically. Automatically (de)serialized from degrees.
        #[serde(with = "conv_deg_rad")]
        fov_y: f32,
    },
    /// A perespective-based viewpoint that will rotate around in the xy plane.
    Orbit {
        /// Distance from [0, 0] that the camera will stay away from while rotating.
        dist: f32,
        /// Height of the camera.
        z: f32,
        /// Current angle of rotation. Unneeded when deserializing.
        #[serde(default)]
        theta: f32,
        /// Coordinate that the center of camera should remain focused on.
        #[serde(default)]
        look_at: [f32; 3],
        /// FOV of the viewpoint vertically. Automatically (de)serialized from degrees.
        #[serde(with = "conv_deg_rad")]
        fov_y: f32,
        /// Amount of frames it should take to complete one full revolution. Higher is slower.
        frame_per_rev: f32,
    },
}

impl Default for ViewStyle {
    fn default() -> Self {
        Self::Orthographic {
            pos: [0., 0., 100.],
            radius: 100.,
        }
    }
}

impl ViewStyle {
    /// Create a graphics view transformation matrix based on `self` and the
    /// width and height.
    pub fn transform_matrix(self, width: f32, height: f32) -> Mat4 {
        let aspect = width / height;
        match self {
            ViewStyle::Orthographic {
                pos: [x, y, _],
                radius,
            } => {
                Mat4::orthographic_rh(
                    radius.mul_add(-aspect, x),
                    radius.mul_add(aspect, x),
                    -radius + y,
                    radius + y,
                    0.1,
                    1000.,
                ) * Mat4::look_at_rh(
                    glam::vec3(0., 0., 100.),
                    glam::vec3(0., 0., 0.),
                    glam::Vec3::Y,
                )
            }
            ViewStyle::Perspective {
                pos,
                look_at,
                fov_y,
            } => {
                Mat4::perspective_rh(fov_y, aspect, 0.1, 1000.)
                    * Mat4::look_at_rh(pos.into(), look_at.into(), glam::Vec3::Z)
            }
            ViewStyle::Orbit {
                dist,
                theta,
                z,
                look_at,
                fov_y,
                frame_per_rev: _,
            } => {
                Mat4::perspective_rh(fov_y, aspect, 0.1, 1000.)
                    * Mat4::look_at_rh(
                        [theta.sin() * dist, -theta.cos() * dist, z].into(),
                        look_at.into(),
                        glam::Vec3::Z,
                    )
            }
        }
    }

    /// Returns the (position, look_at coordinate, vertical fov degrees) if `self`
    /// is [`ViewStyle::Perspective`] or None otherwise.
    pub fn perspective_info(&self) -> Option<([f32; 3], [f32; 3], f32)> {
        if let Self::Perspective {
            pos,
            look_at,
            fov_y,
        } = self
        {
            Some((*pos, *look_at, fov_y.to_degrees()))
        } else {
            None
        }
    }
}

/// Represent the style of world geometry that should be generated
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum WorldStyle {
    Cylinder { radius: f32, height: Option<f32> },
    Plane { radius: f32 },
}

impl WorldStyle {
    /// Generates (vertices, triangle indicies) based on `self`'s settings.
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
