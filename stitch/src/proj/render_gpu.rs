use std::{f32::consts::PI, num::NonZero, ops::DerefMut, path::PathBuf, sync::Arc};

use encase::ShaderType;
use glam::Mat4;
use smpgpu::{
    global as glob_gpu,
    model::{Model, ModelBuilder, VertPosNorm},
    AutoVisBindable, Buffer, Context, MemMapper, Pass, RenderCheckpoint, StorageBuffer, Texture,
    Uniform,
};
use zerocopy::FromZeros;

use crate::{
    buf::FrameSize,
    camera::{live, Camera, Config, ViewParams},
    loader::{self, Loader, OwnedWriteBuffer},
    Result,
};

use super::ViewStyle;

pub struct GpuProjector {
    pass_info: Uniform<PassInfo>,
    pass_info_inp_sizes: glam::UVec3,

    inp_frames: Arc<Buffer>,
    inp_specs: StorageBuffer<InputSpec>,

    main_out: RenderOutput,
    depth_texture: Texture,
    back: Model<Vertex, u16>,
    object_model: Option<Model<VertPosNorm, u16>>,

    sub_outs: Vec<(RenderOutput, RenderCheckpoint)>,
    last_sub_views: Vec<Mat4>,
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

struct RenderOutput {
    pub cam: Uniform<Mat4>,
    pub texture: Texture,
    pub staging: Buffer,
}

impl RenderOutput {
    pub fn new(width: usize, height: usize) -> Self {
        let cam = glob_gpu::uniform().writable().build();
        let texture = glob_gpu::texture()
            .label("out_texture")
            .size(width, height)
            .render_target()
            .readable()
            .build();
        let staging = texture.new_staging_global();

        Self {
            cam,
            texture,
            staging,
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
    num_subs: usize,
    bound_verts: Vec<Vertex>,
    bound_idxs: Vec<u16>,
    mask_paths: Vec<Option<PathBuf>>,
    model_builder: Option<smpgpu::model::ModelBuilder<'a, smpgpu::model::VertPosNorm, u16>>,
}

impl<'a> GpuProjectorBuilder<'a> {
    const fn new() -> Self {
        Self {
            out_size: (0, 0),
            input_size: (0, 0, 0),
            num_subs: 0,
            bound_verts: Vec::new(),
            bound_idxs: Vec::new(),
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

    pub const fn num_subs(mut self, n: usize) -> Self {
        self.num_subs = n;
        self
    }

    pub fn cylinder_bound(mut self) -> Self {
        const SLICES: u16 = 20;
        const RADIUS: f32 = 70.;
        const HEIGHT: f32 = 50.;

        let mut verts = Vec::new();
        verts.push(Vertex::new(0., 0., 0.));

        for n in 0..SLICES {
            let (x, y) = (2. * PI * n as f32 / SLICES as f32).sin_cos();
            let (x, y) = (x * RADIUS, y * RADIUS);
            verts.extend([Vertex::new(x, y, 0.), Vertex::new(x, y, HEIGHT)])
        }

        let mut idxs = Vec::new();
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

        self.bound_verts = verts;
        self.bound_idxs = idxs;
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
        let pass_info = glob_gpu::uniform().label("pass_info").writable().build();

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

        let main_out = RenderOutput::new(self.out_size.0, self.out_size.1);

        let render_shader = smpgpu::include_shader!("shaders/render.wgsl" => "vs_proj" & "fs_proj");

        let back = glob_gpu::model()
            .verts(&self.bound_verts)
            .indices(&self.bound_idxs)
            .cp_build_cam(&main_out.cam, |cp| {
                cp.group(
                    pass_info.in_frag()
                        & inp_frames.in_frag()
                        & inp_specs.in_frag()
                        & inp_masks.in_frag(),
                )
                .shader(render_shader.clone())
                .enable_depth()
                .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
                .frag_target(main_out.texture.format())
                .build()
            });

        let sub_outs = (0..self.num_subs)
            .map(|_| {
                let out = RenderOutput::new(640, 640);
                let cp = glob_gpu::checkpoint()
                    .group(back.view.in_vertex() & out.cam.in_vertex())
                    .group(
                        pass_info.in_frag()
                            & inp_frames.in_frag()
                            & inp_specs.in_frag()
                            & inp_masks.in_frag(),
                    )
                    .shader(render_shader.clone())
                    .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
                    .frag_target(out.texture.format())
                    .build();
                (out, cp)
            })
            .collect();

        let object_model = self.model_builder.map(|b| {
            b.cp_build_cam(&main_out.cam, |cp| {
                cp.shader(smpgpu::include_shader!("shaders/model.wgsl"))
                    .cull_backface()
                    .enable_depth()
                    .vert_buffer_of::<VertPosNorm>(
                        &smpgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
                    )
                    .frag_target(main_out.texture.format())
                    .build()
            })
            .with_view(Mat4::from_translation(glam::vec3(0., 0., 6.68)))
        });

        let depth_texture = glob_gpu::texture()
            .label("depth_texture")
            .size(self.out_size.0, self.out_size.1)
            .render_target()
            .depth()
            .build();

        GpuProjector {
            pass_info,
            pass_info_inp_sizes: self.input_size.into(),
            inp_frames: Arc::new(inp_frames),
            inp_specs,
            main_out,
            depth_texture,
            back,
            object_model,
            sub_outs,
            last_sub_views: Vec::new(),
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

        let out_size = self.main_out.texture.size();
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

        self.main_out.cam.set_global(&view);
    }

    pub fn update_sub_views(&mut self) {
        let proj = Mat4::perspective_rh(80f32.to_radians(), 1., 0.1, 1000.);
        let rm = 2. * PI / self.sub_outs.len() as f32;

        const HEIGHT: f32 = 10.;

        let mut out = Vec::new();
        for (i, (o, _)) in self.sub_outs.iter().enumerate() {
            let rot = rm * (i as f32);

            let view = proj
                * Mat4::look_at_rh(
                    [0., 0., HEIGHT].into(),
                    [rot.sin(), rot.cos(), HEIGHT].into(),
                    glam::Vec3::Z,
                );

            o.cam.set_global(&view);

            out.push(view.inverse());
        }
        self.last_sub_views = out;
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
                    | &self.main_out.texture.color_attach()
                    | self.back.to_item()
                    | self.object_model.as_ref().map(Model::to_item),
            )
            .then(self.main_out.texture.copy_to(&self.main_out.staging))
            .submit();

        glob_gpu::force_wake();

        for (o, cp) in &self.sub_outs {
            glob_gpu::command()
                .then(
                    Pass::render()
                        | &o.texture.color_attach()
                        | cp.to_item()
                            .vert_buf(&self.back.verts)
                            .index_buf(&self.back.idx, 0..self.back.idx_len),
                )
                .then(o.texture.copy_to(&o.staging))
                .submit();
        }

        glob_gpu::force_wake();
    }

    #[inline]
    pub async fn copy_render_to<T: DerefMut<Target = [u8]>>(&self, buf: &mut T) {
        let cpy_fut = MemMapper::new().copy(&self.main_out.staging, buf).run_all();
        glob_gpu::force_wake();
        cpy_fut.await;
    }

    #[inline]
    pub async fn wait_for_subs(&self, f: impl FnOnce(usize, &[u8]) + Send + Clone) {
        let cpy_fut = self
            .sub_outs
            .iter()
            .enumerate()
            .fold(MemMapper::new(), |mapper, (i, (o, _))| {
                let f = f.clone();
                mapper.read_from(&o.staging, move |buf| {
                    // let img =
                    //     image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(640, 640, buf).unwrap();
                    tracing::info!("saving view{i}");
                    f(i, &buf)
                    // img.save(format!("view{i}.png")).unwrap();
                })
            })
            .run_all();

        glob_gpu::force_wake();
        cpy_fut.await;
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
    type View<'a>
        = smpgpu::DirectWritableBufferView<'a>
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Self::View<'_> {
        self.ctx.write_with(&self.buf, self.offset, self.size)
    }
}
