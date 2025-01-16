use std::{num::NonZero, ops::DerefMut, path::PathBuf, sync::Arc};

use encase::ShaderType;
use glam::Mat4;
use smpgpu::{
    global as glob_gpu,
    model::{Model, ModelBuilder, VertPosNorm},
    AutoVisBindable, Buffer, Context, MemMapper, Pass, RenderCheckpoint, StorageBuffer, Texture,
    Uniform, VertexBuffer,
};
use tokio::runtime::Handle;
use zerocopy::FromZeros;

use crate::{
    buf::FrameSize,
    camera::{live, Camera, Config, ViewParams},
    loader::{self, Loader, OwnedWriteBuffer},
    Result,
};

use super::ViewStyle;

pub struct GpuProjector {
    out_texture: Texture,
    out_staging: Buffer,
    pass_info: Uniform<PassInfo>,
    pass_info_inp_sizes: glam::UVec3,
    view_mat: Uniform<Mat4>,
    inp_frames: Arc<Buffer>,
    inp_specs: StorageBuffer<InputSpec>,
    bound_mesh: VertexBuffer<Vertex>,
    back_cp: RenderCheckpoint,
    model: Option<Model<VertPosNorm, u16>>,
    depth_texture: Texture,
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
struct InputSpec {
    /// Camera's position [x, y, z]
    pos: glam::Vec3,
    // Camera reverse mat
    rev_mat: glam::Mat3,
    // Image's offset from cameras optical center
    img_off: glam::Vec2,
    /// Camera's focal distance, relative to diagonal radius of 1
    foc_dist: f32,
    /// Camera's lens type
    lens_type: u32,
}

impl From<ViewParams> for InputSpec {
    #[inline]
    fn from(s: ViewParams) -> Self {
        let rev_mat = glam::Mat3::from_euler(glam::EulerRot::ZXY, s.azimuth, s.pitch, s.roll);

        Self {
            pos: s.pos.into(),
            rev_mat,
            img_off: s.sensor.img_off.into(),
            foc_dist: s
                .sensor
                .fov
                .assume_focal_dist()
                .expect("focal distance not set"),
            lens_type: s.lens as _,
        }
    }
}

#[derive(ShaderType, Clone, Copy, Debug)]
struct PassInfo {
    inp_sizes: glam::UVec3,
    bound_radius: f32,
}

#[derive(ShaderType, Clone, Copy)]
pub struct Vertex {
    pub pos: glam::Vec4,
}

impl Vertex {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: glam::vec4(x, y, z, 1.),
        }
    }
}

pub struct GpuProjectorBuilder<'a> {
    out_size: (usize, usize),
    input_size: (u32, u32, u32),
    bound_mesh: &'a [Vertex],
    mask_paths: Vec<Option<PathBuf>>,
    model_builder: Option<smpgpu::model::ModelBuilder<'a, smpgpu::model::VertPosNorm, u16>>,
}

impl<'a> GpuProjectorBuilder<'a> {
    const fn new() -> Self {
        Self {
            out_size: (0, 0),
            input_size: (0, 0, 0),
            bound_mesh: &[],
            mask_paths: Vec::new(),
            model_builder: None,
        }
    }

    pub const fn input_size(mut self, w: u32, h: u32, n: u32) -> Self {
        self.input_size = (w, h, n);
        self
    }

    pub const fn out_size(mut self, w: usize, h: usize) -> Self {
        self.out_size = (w, h);
        self
    }

    pub fn flat_bound(mut self) -> Self {
        static MESH_DATA: [Vertex; 6] = [
            Vertex::new(-500., -500., 0.),
            Vertex::new(500., -500., 0.),
            Vertex::new(500., 500., 0.),
            Vertex::new(500., 500., 0.),
            Vertex::new(-500., 500., 0.),
            Vertex::new(-500., -500., 0.),
        ];
        self.bound_mesh = &MESH_DATA;
        self
    }

    pub fn masks_from_cfgs(mut self, cfgs: &[Config<live::Config>]) -> Self {
        self.mask_paths = cfgs.iter().map(|c| c.meta.mask_path.clone()).collect();
        self
    }

    pub fn model(
        mut self,
        f: impl FnOnce(
            ModelBuilder<'a, smpgpu::model::VertPosNorm, u16>,
        ) -> ModelBuilder<'a, smpgpu::model::VertPosNorm, u16>,
    ) -> Self {
        self.model_builder = Some(f(self
            .model_builder
            .take()
            .unwrap_or_else(|| glob_gpu::model())));
        self
    }

    pub fn build(self) -> GpuProjector {
        let out_texture = glob_gpu::texture()
            .label("out_texture")
            .size(self.out_size.0, self.out_size.1)
            .render_target()
            .readable()
            .build();
        let out_staging = out_texture.new_staging_global();

        let pass_info = glob_gpu::uniform().label("pass_info").writable().build();

        let view_mat = glob_gpu::uniform().label("view").writable().build();

        let inp_frames = glob_gpu::buffer()
            .label("inp_frames")
            .size((self.input_size.0 * self.input_size.1 * self.input_size.2 * 4) as _)
            .storage()
            .writable()
            .build();

        let inp_specs = glob_gpu::storage_buffer()
            .label("inp_specs")
            .len(self.input_size.2.into())
            .writable()
            .build();

        let inp_masks = glob_gpu::storage_buffer()
            .label("inp_masks")
            .writable()
            .init_data(&self.generate_masks())
            .build();

        let bound_mesh = glob_gpu::vertex_buffer()
            .label("bound_mesh")
            .init_data(self.bound_mesh)
            .build();

        let back_cp = glob_gpu::checkpoint()
            .group(
                pass_info.in_frag()
                    & view_mat.in_vertex()
                    & inp_frames.in_frag()
                    & inp_specs.in_frag()
                    & inp_masks.in_frag(),
            )
            .shader(smpgpu::include_shader!("shaders/render.wgsl" => "vs_proj" & "fs_proj"))
            .enable_depth()
            .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
            .frag_target(out_texture.format())
            .build();

        let model = self.model_builder.map(|b| {
            b.cp_build(|cp| {
                cp.shader(smpgpu::include_shader!("shaders/model.wgsl"))
                    .cull_backface()
                    .enable_depth()
                    .vert_buffer_of::<VertPosNorm>(
                        &smpgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
                    )
                    .frag_target(out_texture.format())
                    .build()
            })
        });

        let depth_texture = glob_gpu::texture()
            .label("depth_texture")
            .size(self.out_size.0, self.out_size.1)
            .render_target()
            .depth()
            .build();

        GpuProjector {
            out_texture,
            out_staging,
            pass_info,
            pass_info_inp_sizes: self.input_size.into(),
            view_mat,
            inp_frames: Arc::new(inp_frames),
            inp_specs,
            bound_mesh,
            back_cp,
            model,
            depth_texture,
        }
    }

    fn generate_masks(&self) -> Box<[u32]> {
        let img_size = self.input_size.0 * self.input_size.1;

        let mut out =
            <[u32]>::new_box_zeroed_with_elems((img_size * self.input_size.2) as _).unwrap();

        self.mask_paths
            .iter()
            .zip(out.chunks_mut(img_size as _))
            .for_each(|(p, view)| {
                let opt_data = p.as_deref().and_then(|p| {
                    image::open(p)
                        .inspect_err(|err| tracing::error!("failed to load mask {:?}: {err}", p))
                        .ok()
                });

                if let Some(data) = opt_data {
                    data.to_luma8()
                        .iter()
                        .zip(view)
                        .for_each(|(p, o)| *o = if *p >= 128 { !0 } else { 0 });
                } else {
                    view.fill(!0);
                }
            });

        out
    }
}

impl GpuProjector {
    #[inline]
    pub fn builder() -> GpuProjectorBuilder<'static> {
        GpuProjectorBuilder::new()
    }

    #[inline]
    pub fn update_proj_view(&self, style: ViewStyle) {
        self.pass_info.set_global(&PassInfo {
            inp_sizes: self.pass_info_inp_sizes,
            bound_radius: 0.0,
        });

        let out_size = self.out_texture.size();
        #[allow(clippy::cast_precision_loss)]
        let aspect = out_size.width as f32 / out_size.height as f32;

        let view = match style {
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
        };

        self.view_mat.set_global(&view);
        if let Some(model) = &self.model {
            model.set_view(view * Mat4::from_translation(glam::vec3(0., 0., 6.68)));
        }
    }

    #[inline]
    pub fn update_cam_specs<T>(&self, cams: &[Camera<T>]) {
        self.inp_specs.set_global(
            &cams
                .iter()
                .map(|c: &Camera<T>| c.view.into())
                .collect::<Vec<_>>(),
        );
    }

    #[inline]
    pub fn update_render(&self) {
        glob_gpu::command()
            .then(
                Pass::render()
                    | &self.depth_texture.depth_attach()
                    | &self.out_texture.color_attach()
                    | self.back_cp.to_item().vert_buf(&self.bound_mesh)
                    | self.model.as_ref().map(Model::to_item),
            )
            .then(self.out_texture.copy_to(&self.out_staging))
            .submit();

        glob_gpu::force_wake();
    }

    #[inline]
    pub fn block_copy_render_to<T: DerefMut<Target = [u8]>>(&self, buf: &mut T) {
        let cpy_fut = MemMapper::new().copy(&self.out_staging, buf).run_all();
        glob_gpu::force_wake();
        Handle::current().block_on(cpy_fut);
    }

    #[inline]
    pub fn take_input_buffers(
        &self,
        cams: &[Camera<Loader<GpuDirectBufferWrite>>],
    ) -> Result<Vec<loader::Ticket<GpuDirectBufferWrite>>> {
        cams.iter()
            .scan(0, |off, c| {
                let size = c.data.num_bytes() as u64;
                let buf_off = *off;
                *off += size;

                Some(c.data.give(self.inp_buffer_write(buf_off, size)))
            })
            .collect()
    }

    #[inline]
    fn inp_buffer_write(&self, offset: u64, size: u64) -> GpuDirectBufferWrite {
        GpuDirectBufferWrite {
            ctx: glob_gpu::get_global_context(),
            buf: self.inp_frames.clone(),
            offset,
            size: size.try_into().unwrap(),
        }
    }
}

pub struct GpuDirectBufferWrite {
    ctx: Arc<Context>,
    buf: Arc<Buffer>,
    offset: u64,
    size: NonZero<u64>,
}

impl OwnedWriteBuffer for GpuDirectBufferWrite {
    type View<'a> = smpgpu::DirectWritableBufferView<'a>
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Self::View<'_> {
        self.ctx.write_with(&self.buf, self.offset, self.size)
    }
}
