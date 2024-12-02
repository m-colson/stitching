use std::{cell::Cell, num::NonZero, ops::DerefMut, path::PathBuf, sync::Arc};

use encase::ShaderType;
use glam::Mat4;
use smpgpu::{Bindable, Bindings, Buffer, Context, MemMapper, RenderCheckpoint, Texture};
use tokio::runtime::Handle;
use zerocopy::FromZeros;

use crate::{
    buf::FrameSize,
    camera::{live, Camera, Config, ViewParams},
    loader::{self, Loader, OwnedWriteBuffer},
    Result,
};

use super::ProjectionStyle;

pub struct GpuProjector {
    ctx: Arc<Context>,
    out_texture: Texture,
    out_staging: Buffer,
    pass_info: Buffer,
    pass_info_data: Cell<PassInfo>,
    view_mat: Buffer,
    inp_frames: Arc<Buffer>,
    inp_specs: Buffer,
    bound_mesh: Buffer,
    back_cp: RenderCheckpoint,
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

#[derive(ShaderType)]
struct Vertex {
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

#[derive(Clone)]
pub struct GpuProjectorBuilder<'a> {
    ctx: Arc<Context>,
    out_size: (usize, usize),
    input_size: (u32, u32, u32),
    bound_mesh: &'a [Vertex],
    mask_paths: Vec<Option<PathBuf>>,
}

impl<'a> GpuProjectorBuilder<'a> {
    const fn new(ctx: Arc<Context>) -> Self {
        Self {
            ctx,
            out_size: (0, 0),
            input_size: (0, 0, 0),
            bound_mesh: &[],
            mask_paths: Vec::new(),
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

    pub fn build(self) -> GpuProjector {
        let ctx = self.ctx.as_ref();

        let out_texture = Texture::builder(ctx)
            .label("out_texture")
            .size(self.out_size.0, self.out_size.1)
            .render_target()
            .readable()
            .build();
        let out_staging = out_texture.new_staging(ctx);

        let pass_info = Buffer::builder(ctx)
            .label("pass_info")
            .size_for::<PassInfo>()
            .uniform()
            .writable()
            .build();

        let view_mat = Buffer::builder(ctx)
            .label("view")
            .size_for::<glam::Mat4>()
            .uniform()
            .writable()
            .build();

        let inp_frames = Buffer::builder(ctx)
            .label("inp_frames")
            .size(self.input_bytes())
            .storage()
            .writable()
            .build();

        let inp_specs = Buffer::builder(ctx)
            .label("inp_specs")
            .size_for_many::<InputSpec>(self.input_size.2.into())
            .storage()
            .writable()
            .build();

        let inp_masks = Buffer::builder(ctx)
            .label("inp_masks")
            .storage()
            .writable()
            .build_with_data(&self.generate_masks());

        let bound_mesh = Buffer::builder(ctx)
            .label("bound_mesh")
            .vertex()
            .build_with_data(self.bound_mesh);

        let back_cp = RenderCheckpoint::builder(ctx)
            .group(
                Bindings::new()
                    .bind(pass_info.in_frag())
                    .bind(view_mat.in_vertex())
                    .bind(inp_frames.in_frag())
                    .bind(inp_specs.in_frag())
                    .bind(inp_masks.in_frag()),
            )
            .shader(smpgpu::include_shader!("shaders/render.wgsl" => "vs_proj" & "fs_proj"))
            .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
            .frag_target(out_texture.format())
            .build()
            .vertices(0..self.bound_mesh.len().try_into().unwrap());

        GpuProjector {
            ctx: self.ctx,
            out_texture,
            out_staging,
            pass_info,
            pass_info_data: Cell::new(PassInfo {
                inp_sizes: self.input_size.into(),
                bound_radius: f32::NAN,
            }),
            view_mat,
            inp_frames: Arc::new(inp_frames),
            inp_specs,
            bound_mesh,
            back_cp,
        }
    }

    const fn input_bytes(&self) -> usize {
        (self.input_size.0 * self.input_size.1 * self.input_size.2 * 4) as _
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
    /// # Errors
    /// see [`smpgpu::ctx::ContextAdapterBuilder::request_adapter`] and [`smpgpu::ctx::ContextDeviceBuilder::request_build`]
    #[inline]
    pub async fn builder_auto() -> Result<GpuProjectorBuilder<'static>> {
        Ok(GpuProjectorBuilder::new(
            smpgpu::Context::builder()
                .request_adapter()
                .await?
                .request_build()
                .await?,
        ))
    }

    #[inline]
    pub fn update_proj_view(&self, style: ProjectionStyle) {
        match style {
            ProjectionStyle::Hemisphere {
                pos: [x, y, _],
                radius,
            } => {
                let mut pass_info_data = self.pass_info_data.get();
                pass_info_data.bound_radius = radius;
                self.pass_info_data.set(pass_info_data);

                self.ctx.write_uniform(&self.pass_info, &pass_info_data);
                let out_size = self.out_texture.size();

                let rh = radius;

                #[allow(clippy::cast_precision_loss)]
                let aspect = out_size.width as f32 / out_size.height as f32;

                let view = Mat4::orthographic_rh(
                    rh.mul_add(-aspect, x),
                    rh.mul_add(aspect, x),
                    -rh + y,
                    rh + y,
                    0.1,
                    200.,
                ) * Mat4::look_at_rh(
                    glam::vec3(0., 0., 100.),
                    glam::vec3(0., 0., 0.),
                    glam::Vec3::Y,
                );
                self.ctx.write_uniform(&self.view_mat, &view);
            }
            ProjectionStyle::RawCamera(..) => todo!(),
        }
    }

    #[inline]
    pub fn update_cam_specs<T>(&self, cams: &[Camera<T>]) {
        self.ctx.write_storage(
            &self.inp_specs,
            &cams
                .iter()
                .map(|c| c.view.into())
                .collect::<Vec<InputSpec>>(),
        );
    }

    #[inline]
    pub fn update_render(&self) {
        let back_cmd = self
            .back_cp
            .encoder(&*self.ctx)
            .vert_buf(&self.bound_mesh)
            .attach(&self.out_texture.render_attach())
            .then(self.out_texture.copy_to_buf_op(&self.out_staging))
            .build();

        self.ctx.submit([back_cmd]);
        self.ctx.signal_wake();
    }

    #[inline]
    pub fn block_copy_render_to<T: DerefMut<Target = [u8]> + FrameSize>(&self, buf: &mut T) {
        let cpy_fut = MemMapper::new()
            .with_cb(&self.out_staging, |data| {
                buf.copy_from_slice(&data);
            })
            .run_all();

        self.ctx.signal_wake();

        Handle::current().block_on(cpy_fut);
    }

    /// # Errors
    /// see [`LoadingBuffer::begin_load_with`]
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
            ctx: self.ctx.clone(),
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
