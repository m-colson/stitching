use std::{cell::Cell, sync::Arc};

use encase::ShaderType;
use glam::Mat4;
use smpgpu::{Bindable, Bindings, Buffer, Context, MemMapper, RenderCheckpoint, Texture};
use tokio::runtime::Handle;

use crate::{
    camera::CameraSpec,
    frame::{FrameBuffer, FrameBufferView, FrameSize},
    loader::{collect_empty_camera_tickets, FrameLoaderTicket, LoadingBuffer, OwnedWriteBuffer},
    Camera, FrameBufferMut, Result,
};

use super::{FetchProjector, ProjStyle, Projector};

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
    // Camera direction vectors
    forw: glam::Vec3,
    right: glam::Vec3,
    up: glam::Vec3,
    /// Camera's angle [azimuth, pitch, roll]
    ang: glam::Vec3,
    /// Camera's focal distance, relative to diagonal radius of 1
    foc_dist: f32,
    /// Camera's lens type
    lens_type: u32,
}

impl From<CameraSpec> for InputSpec {
    fn from(s: CameraSpec) -> Self {
        let (foc_dist, lens_type) = {
            let diag = s.fov.diag_radians() / 2.;
            match s.lens {
                crate::camera::CameraLens::Rectilinear => {
                    (/* cot(diag) */ diag.cos() / diag.sin(), 0)
                }
                crate::camera::CameraLens::Equidistant => (1. / diag, 1),
            }
        };

        let pv = glam::vec2(s.pitch.cos(), s.pitch.sin());
        let right = glam::vec3(s.azimuth.cos(), s.azimuth.sin(), 0.0);
        let forw = glam::vec3(-right.y * pv.x, right.x * pv.x, pv.y);
        let up = glam::vec3(right.y * pv.y, -right.x * pv.y, pv.x);

        Self {
            pos: glam::vec3(s.x, s.y, s.z),
            forw,
            right,
            up,
            ang: glam::vec3(s.azimuth, s.pitch, s.roll),
            foc_dist,
            lens_type,
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
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: glam::vec4(x, y, z, 1.),
        }
    }
}

impl GpuProjector {
    #[inline]
    pub async fn new_auto(w: usize, h: usize, input_dim: (usize, usize, usize)) -> Result<Self> {
        let ctx = smpgpu::Context::new().await?;
        Ok(Self::new_from_ctx(ctx, w, h, input_dim))
    }

    pub fn new_from_ctx(
        ctx: Arc<Context>,
        w: usize,
        h: usize,
        input_dim: (usize, usize, usize),
    ) -> Self {
        let out_texture = Texture::builder(&*ctx)
            .label("out_texture")
            .size(w, h)
            .render_target()
            .readable()
            .build();
        let out_staging = out_texture.new_staging(&*ctx);

        let pass_info = Buffer::builder(&*ctx)
            .label("pass_info")
            .size_for::<PassInfo>()
            .uniform()
            .writable()
            .build();

        let view_mat = Buffer::builder(&*ctx)
            .label("view")
            .size_for::<glam::Mat4>()
            .uniform()
            .writable()
            .build();

        let inp_frames = Buffer::builder(&*ctx)
            .label("inp_frames")
            .size(input_dim.0 * input_dim.1 * input_dim.2 * 4)
            .storage()
            .writable()
            .build();

        let inp_specs = Buffer::builder(&*ctx)
            .label("inp_specs")
            .size_for_many::<InputSpec>(input_dim.2 as _)
            .storage()
            .writable()
            .build();

        static MESH_DATA: [Vertex; 6] = [
            Vertex::new(-500., -500., 0.),
            Vertex::new(500., -500., 0.),
            Vertex::new(500., 500., 0.),
            Vertex::new(500., 500., 0.),
            Vertex::new(-500., 500., 0.),
            Vertex::new(-500., -500., 0.),
        ];

        let bound_mesh = Buffer::builder(&*ctx)
            .label("bound_mesh")
            .vertex()
            .build_with_data(&MESH_DATA);

        let back_cp = RenderCheckpoint::builder(&*ctx)
            .group(
                Bindings::new()
                    .bind(pass_info.in_frag())
                    .bind(view_mat.in_vertex())
                    .bind(inp_frames.in_frag())
                    .bind(inp_specs.in_frag()),
            )
            .vert_shader(smpgpu::include_wgsl!("shaders/render.wgsl"), "vs_proj")
            .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
            .frag_shader(None, "fs_proj")
            .frag_target(out_texture.format())
            .build()
            .vertices(0..(MESH_DATA.len() as _));

        GpuProjector {
            ctx,
            out_texture,
            out_staging,
            pass_info,
            pass_info_data: Cell::new(PassInfo {
                inp_sizes: glam::uvec3(input_dim.0 as _, input_dim.1 as _, input_dim.2 as _),
                bound_radius: f32::NAN,
            }),
            view_mat,
            inp_frames: Arc::new(inp_frames),
            inp_specs,
            bound_mesh,
            back_cp,
        }
    }

    pub fn update_proj_style(&self, style: ProjStyle) {
        let ProjStyle::Hemisphere { radius } = style else {
            panic!("only hemisphere projection is supported on GpuProjector");
        };

        let mut pass_info_data = self.pass_info_data.get();
        pass_info_data.bound_radius = radius;
        self.pass_info_data.set(pass_info_data);

        self.ctx.write_uniform(&self.pass_info, &pass_info_data);

        let rh = 100.;
        let aspect = 16. / 9.;
        let view = Mat4::orthographic_rh(-rh * aspect, rh * aspect, -rh, rh, 0.1, 200.)
            * Mat4::look_at_rh(
                glam::vec3(0., 0., 100.),
                glam::vec3(0., 0., 0.),
                glam::vec3(0., 1., 0.),
            );
        self.ctx.write_uniform(&self.view_mat, &view);
    }

    pub fn update_cam_specs<T, K>(&self, cams: &[Camera<T, K>]) {
        self.ctx.write_storage(
            &self.inp_specs,
            &cams
                .iter()
                .map(|c| c.spec.into())
                .collect::<Vec<InputSpec>>(),
        );
    }

    pub fn update_render(&self) {
        let back_cmd = self
            .back_cp
            .encoder(&*self.ctx)
            .attach(&self.out_texture.render_attach())
            .vert_buf(&self.bound_mesh)
            .then(self.out_texture.copy_to_buf_op(&self.out_staging))
            .build();

        self.ctx.submit([back_cmd]);
        self.ctx.signal_wake();
    }

    pub fn block_copy_render_to(&self, buf: &mut impl FrameBufferMut) {
        let cpy_fut = MemMapper::new()
            .with_cb(&self.out_staging, |data| {
                buf.as_bytes_mut().copy_from_slice(&data)
            })
            .run_all();

        self.ctx.signal_wake();

        Handle::current().block_on(cpy_fut);
    }

    pub fn take_input_buffers<K>(
        &self,
        cams: &[Camera<LoadingBuffer<(), GpuDirectBufferWrite>, K>],
    ) -> Vec<FrameLoaderTicket<GpuDirectBufferWrite>> {
        cams.iter()
            .scan(0, |off, c| {
                let size = c.buf.num_bytes() as u64;
                let buf_off = *off;
                *off += size;

                Some(c.buf.begin_load_with(self.inp_buffer_write(buf_off, size)))
            })
            .collect()
    }

    fn inp_buffer_write(&self, offset: u64, size: u64) -> GpuDirectBufferWrite {
        GpuDirectBufferWrite {
            ctx: self.ctx.clone(),
            buf: self.inp_frames.clone(),
            offset,
            size: size.try_into().unwrap(),
        }
    }
}

impl Projector for GpuProjector {
    type ForwProj = GpuForwProj;
    type LoadResult = ();

    #[inline]
    fn load_forw(
        &self,
        style: ProjStyle,
        _spec: CameraSpec,
        _fp: &mut Self::ForwProj,
    ) -> Self::LoadResult {
        self.update_proj_style(style);
    }

    #[inline]
    fn load_back<F: FrameBuffer, K>(
        &self,
        _fp: &Self::ForwProj,
        _: &[Camera<F, K>],
        buf: &mut impl FrameBufferMut,
    ) -> Self::LoadResult {
        self.update_render();
        self.block_copy_render_to(buf);
    }

    #[inline]
    fn new_forw(&self) -> Self::ForwProj {
        GpuForwProj
    }
}

impl FetchProjector<(), GpuDirectBufferWrite> for GpuProjector {
    type Ticket = FrameLoaderTicket<GpuDirectBufferWrite>;

    type OutBuf<'a> = FrameBufferView<'static>;

    #[inline]
    fn begin_fetch<K>(
        &self,
        cams: &mut [Camera<LoadingBuffer<(), GpuDirectBufferWrite>, K>],
    ) -> Vec<Self::Ticket> {
        let tickets = self.take_input_buffers(cams);
        self.update_cam_specs(cams);
        tickets
    }

    #[inline]
    async fn finish_fetch<'a, K>(
        &self,
        cams: &'a mut [Camera<LoadingBuffer<(), GpuDirectBufferWrite>, K>],
        tickets: Vec<Self::Ticket>,
    ) -> Vec<Camera<Self::OutBuf<'a>>> {
        collect_empty_camera_tickets(tickets, cams).await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GpuForwProj;

pub struct GpuDirectBufferWrite {
    ctx: Arc<Context>,
    buf: Arc<Buffer>,
    offset: u64,
    size: std::num::NonZero<u64>,
}

impl OwnedWriteBuffer for GpuDirectBufferWrite {
    type View<'a> = GpuDirectBufferView<'a>
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Self::View<'_> {
        self.ctx.write_with(&self.buf, self.offset, self.size)
    }
}

pub type GpuDirectBufferView<'a> = smpgpu::QueueWriteBufferView<'a>;
