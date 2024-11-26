use std::{cell::Cell, sync::Arc};

use encase::ShaderType;
use smpgpu::{Bindings, Buffer, CommandCheckpoint, Context, MemMapper};
use tokio::runtime::Handle;

use crate::{
    camera::CameraSpec,
    frame::{FrameBuffer, FrameBufferView, FrameSize},
    loader::{FrameLoaderTicket, OwnedWriteBuffer},
    Camera, FrameBufferMut, Result,
};

use super::{FetchProjector, ProjStyle, Projector};

pub struct GpuProjector {
    ctx: Arc<Context>,
    out_frame: Buffer,
    out_staging: Buffer,
    pass_info: Buffer,
    pass_info_data: Cell<PassInfo>,
    inp_frames: Arc<Buffer>,
    inp_specs: Buffer,
    back_cp: CommandCheckpoint,
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
    out_spec: InputSpec,
    out_size: glam::UVec2,
    inp_sizes: glam::UVec3,
    bound_radius: f32,
}

impl GpuProjector {
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
        let out_frame = Buffer::builder(&*ctx)
            .label("out_frame")
            .size(w * h * 4)
            .storage()
            .readable()
            .build();
        let out_staging = Buffer::builder(&*ctx)
            .label("out_staging")
            .size(w * h * 4)
            .writable()
            .build();

        let pass_info = Buffer::builder(&*ctx)
            .label("pass_info")
            .size_for::<PassInfo>()
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

        let back_bindings = Bindings::new(&*ctx)
            .bind(&out_frame)
            .bind(&pass_info)
            .bind(&inp_frames)
            .bind(&inp_specs);

        let back_cp = back_bindings
            .compute_shader(smpgpu::include_wgsl!("shaders/compute.wgsl"), "main")
            .work_groups(w, h, 1)
            .into();

        // let back_cp = back_bindings
        //     .render_shader(
        //         smpgpu::include_wgsl!("shaders/render.wgsl"),
        //         "vs_main",
        //         "fs_main",
        //     )
        //     .vertices(0..4)
        //     .instances(0..1)
        //     .into();

        GpuProjector {
            ctx,
            out_frame,
            out_staging,
            pass_info,
            pass_info_data: Cell::new(PassInfo {
                out_spec: InputSpec::default(),
                out_size: glam::uvec2(w as _, h as _),
                inp_sizes: glam::uvec3(input_dim.0 as _, input_dim.1 as _, input_dim.2 as _),
                bound_radius: f32::NAN,
            }),
            inp_frames: Arc::new(inp_frames),
            inp_specs,
            back_cp,
        }
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

    fn load_forw(
        &self,
        style: ProjStyle,
        spec: CameraSpec,
        _fp: &mut Self::ForwProj,
    ) -> Self::LoadResult {
        let ProjStyle::Hemisphere { radius } = style else {
            panic!("only hemisphere projection is supported on GpuProjector");
        };

        let mut data = self.pass_info_data.get();
        data.out_spec = spec.into();
        data.bound_radius = radius;
        self.pass_info_data.set(data);

        self.ctx.write_uniform(&self.pass_info, &data);
    }

    fn load_back<F: FrameBuffer, K>(
        &self,
        _fp: &Self::ForwProj,
        _: &[Camera<F, K>],
        buf: &mut impl FrameBufferMut,
    ) -> Self::LoadResult {
        if buf.chans() != 4 {
            panic!("buf must have 4 channels, got {}", buf.chans());
        }

        let back_cmd = self
            .back_cp
            .builder(&*self.ctx)
            .then(self.out_frame.copy_to_buf_op(&self.out_staging))
            .build();

        self.ctx.submit([back_cmd]);

        let cpy_fut = MemMapper::new()
            .with_cb(&self.out_staging, |data| {
                buf.as_bytes_mut().copy_from_slice(&data)
            })
            .run_all();

        self.ctx.signal_wake();

        Handle::current().block_on(cpy_fut);
    }

    fn new_forw(&self) -> Self::ForwProj {
        let info = self.pass_info_data.get();
        GpuForwProj {
            width: info.out_size.x as _,
            height: info.out_size.y as _,
        }
    }
}

impl FetchProjector<(), GpuDirectBufferWrite> for GpuProjector {
    type Ticket = FrameLoaderTicket<GpuDirectBufferWrite>;

    type OutBuf<'a> = FrameBufferView<'static>;

    fn begin_fetch<K>(
        &self,
        cams: &mut [Camera<crate::loader::LoadingBuffer<(), GpuDirectBufferWrite>, K>],
    ) -> Vec<Self::Ticket> {
        let tickets = cams
            .iter()
            .scan(0, |off, c| {
                let size = c.buf.num_bytes() as u64;
                let buf_off = *off;
                *off += size;

                Some(c.buf.begin_load_with(self.inp_buffer_write(buf_off, size)))
            })
            .collect();

        self.ctx.write_storage::<Vec<InputSpec>>(
            &self.inp_specs,
            &cams.iter().map(|c| c.spec.into()).collect(),
        );

        tickets
    }

    fn block_finish_fetch<'a, K>(
        &self,
        cams: &'a mut [Camera<crate::loader::LoadingBuffer<(), GpuDirectBufferWrite>, K>],
        tickets: Vec<Self::Ticket>,
    ) -> Vec<Camera<Self::OutBuf<'a>>> {
        cams.iter()
            .zip(tickets)
            .map(|(c, ticket)| {
                _ = ticket.block_take();
                c.with_map(|b| b.as_empty_view())
            })
            .collect()
    }

    async fn finish_fetch<'a, K>(
        &self,
        cams: &'a mut [Camera<crate::loader::LoadingBuffer<(), GpuDirectBufferWrite>, K>],
        tickets: Vec<Self::Ticket>,
    ) -> Vec<Camera<Self::OutBuf<'a>>> {
        futures::future::join_all(cams.iter().zip(tickets).map(|(c, ticket)| async {
            _ = ticket.take().await;
            c.with_map(|b| b.as_empty_view())
        }))
        .await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GpuForwProj {
    pub width: usize,
    pub height: usize,
}

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
