use std::{borrow::Cow, f32::consts::PI, ops::DerefMut, path::PathBuf, sync::Arc};

use encase::ShaderType;
use glam::Mat4;
use smpgpu::{
    global as glob_gpu,
    model::{Model, ModelBuilder, VertPosNorm},
    AsRenderItem, AsyncMemMapper, AutoVisBindable, Buffer, Context, CopyOp, MemMapper, Pass,
    RenderCheckpoint, StorageBuffer, Texture, Uniform, VertexBuffer,
};
use tokio::sync::Mutex;
use zerocopy::{FromBytes, FromZeros};

use crate::{
    buf::FrameSize,
    camera::{live, Camera, Config, ViewParams},
    loader::{self, Loader, OwnedWriteBuffer},
    Result,
};

use super::ViewStyle;

#[derive(Clone, Debug)]
pub struct InverseView(pub Mat4);

pub struct GpuProjector {
    pass_info: Uniform<PassInfo>,

    inp_frames: Arc<Buffer>,
    inp_specs: StorageBuffer<InputSpec>,
    inp_sizes: Vec<glam::UVec2>,
    inp_starts: Vec<u32>,

    main_out: RenderOutput,
    depth_texture: Texture,
    back: Model<Vertex, u16>,
    object_model: Option<Model<VertPosNorm, u32>>,

    sub_outs: Vec<SubOutput>,
    last_sub_views: Vec<Mat4>,
    bounding_vertices: VertexBuffer<TexturedVertex>,
    bounding_vertices_len: usize,
    bounding_cp: RenderCheckpoint,
}

struct SubOutput {
    pub rend: RenderOutput,
    pub depth: Texture,
    pub depth_staging: Buffer,
    pub depth_data: Mutex<Box<[f32]>>,
    pub cp: RenderCheckpoint,
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
struct InputSpec {
    resolution: glam::UVec2,
    data_start: u32,
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

impl InputSpec {
    #[inline]
    fn from_view(s: ViewParams, resolution: glam::UVec2, data_start: u32) -> Self {
        let rev_mat = glam::Mat3::from_euler(glam::EulerRot::ZXY, s.azimuth, s.pitch, s.roll);

        Self {
            resolution,
            data_start,
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

    pub fn prepare(&self) -> CopyOp<'_> {
        self.texture.copy_to(&self.staging)
    }
}

#[derive(ShaderType, Clone, Copy, Debug)]
struct PassInfo {
    bound_radius: f32,
    num_cameras: u32,
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

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct TexturedVertex {
    pub pos: glam::Vec4,
    pub text_coord: glam::Vec2,
}

impl TexturedVertex {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, tx: f32, ty: f32) -> Self {
        Self {
            pos: glam::vec4(x, y, z, 1.),
            text_coord: glam::vec2(tx, ty),
        }
    }

    #[inline]
    pub const fn from_pos(pos: glam::Vec4, tx: f32, ty: f32) -> Self {
        Self {
            pos,
            text_coord: glam::vec2(tx, ty),
        }
    }
}

pub struct GpuProjectorBuilder<'a> {
    input_sizes: Vec<glam::UVec2>,
    out_size: (usize, usize),
    num_subs: usize,
    bound_verts: Vec<Vertex>,
    bound_idxs: Vec<u16>,
    mask_paths: Vec<Option<PathBuf>>,
    model_builder: Option<smpgpu::model::ModelBuilder<'a, smpgpu::model::VertPosNorm, u32>>,
    model_origin: glam::Vec3,
    model_scale: glam::Vec3,
}

impl<'a> GpuProjectorBuilder<'a> {
    const fn new() -> Self {
        Self {
            input_sizes: Vec::new(),
            out_size: (0, 0),
            num_subs: 0,
            bound_verts: Vec::new(),
            bound_idxs: Vec::new(),
            mask_paths: Vec::new(),
            model_builder: None,
            model_origin: glam::vec3(0., 0., 0.),
            model_scale: glam::vec3(1., 1., 1.),
        }
    }

    pub fn input_sizes<T: Into<glam::UVec2>>(
        mut self,
        input_sizes: impl IntoIterator<Item = T>,
    ) -> Self {
        self.input_sizes
            .extend(input_sizes.into_iter().map(|s| s.into()));
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
        const HEIGHT: f32 = 80.;

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
            ModelBuilder<'a, smpgpu::model::VertPosNorm, u32>,
        ) -> ModelBuilder<'a, smpgpu::model::VertPosNorm, u32>,
    ) -> Self {
        self.model_builder = Some(f(self
            .model_builder
            .take()
            .unwrap_or_else(|| glob_gpu::model())));
        self
    }

    pub fn model_origin(mut self, origin: [f32; 3]) -> Self {
        self.model_origin = origin.into();
        self
    }

    pub fn model_scale(mut self, scale: [f32; 3]) -> Self {
        self.model_scale = scale.into();
        self
    }

    pub fn build(self) -> GpuProjector {
        let pass_info = glob_gpu::uniform().label("pass_info").writable().build();

        let inp_ranges = self.calc_input_ranges();

        let inp_frames = glob_gpu::buffer()
            .label("inp_frames")
            .size((inp_ranges.last().unwrap().1 * 4) as _)
            .storage()
            .writable()
            .build();

        let inp_specs = glob_gpu::storage_buffer()
            .label("inp_specs")
            .len(self.input_sizes.len() as _)
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
                .frag_target(main_out.texture.frag_target_format())
                .build()
            });

        let sub_outs = (0..self.num_subs)
            .map(|_| {
                let rend = RenderOutput::new(640, 640);
                let depth = glob_gpu::texture()
                    .size(640, 640)
                    .depth()
                    .readable()
                    .build();
                let depth_staging = depth.new_staging_global();
                let depth_data = Mutex::new(<[f32]>::new_box_zeroed_with_elems(640 * 640).unwrap());
                let cp = glob_gpu::checkpoint()
                    .group(back.view.in_vertex() & rend.cam.in_vertex())
                    .group(
                        pass_info.in_frag()
                            & inp_frames.in_frag()
                            & inp_specs.in_frag()
                            & inp_masks.in_frag(),
                    )
                    .shader(render_shader.clone())
                    .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
                    .frag_target(rend.texture.format())
                    .enable_depth()
                    .build();
                SubOutput {
                    rend,
                    depth,
                    depth_staging,
                    depth_data,
                    cp,
                }
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
            .with_view(Mat4::from_scale_rotation_translation(
                self.model_scale,
                glam::Quat::IDENTITY,
                self.model_origin,
            ))
        });

        let depth_texture = glob_gpu::texture()
            .label("depth_texture")
            .size(self.out_size.0, self.out_size.1)
            .depth()
            .build();

        let bound_vertices = glob_gpu::vertex_buffer().len(4096).writable().build();
        let bound_cp = glob_gpu::checkpoint()
            .group(back.view.in_vertex() & main_out.cam.in_vertex())
            .shader(smpgpu::include_shader!("shaders/bounds.wgsl"))
            .enable_depth()
            .vert_buffer_of::<TexturedVertex>(
                &smpgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x2],
            )
            .frag_target(main_out.texture.frag_target_format().use_transparency())
            .build();

        GpuProjector {
            pass_info,
            inp_frames: Arc::new(inp_frames),
            inp_specs,
            inp_sizes: self.input_sizes.clone(),
            inp_starts: inp_ranges.into_iter().map(|v| v.0).collect(),
            main_out,
            depth_texture,
            back,
            object_model,
            sub_outs,
            last_sub_views: Vec::new(),
            bounding_vertices: bound_vertices,
            bounding_vertices_len: 0,
            bounding_cp: bound_cp,
        }
    }

    fn calc_input_ranges(&self) -> Vec<(u32, u32)> {
        self.input_sizes
            .iter()
            .scan(0, |o, v| {
                let out = *o;
                *o += v.x * v.y;
                Some((out, *o))
            })
            .collect()
    }

    fn generate_masks(&self) -> Box<[u32]> {
        let input_ranges = self.calc_input_ranges();

        let mut out =
            <[u32]>::new_box_zeroed_with_elems(input_ranges.last().unwrap().1 as _).unwrap();

        if self.mask_paths.is_empty() {
            out.fill(!0);
            return out;
        }

        tracing::info!("ranges {input_ranges:?}");

        self.mask_paths
            .iter()
            .zip(input_ranges)
            .for_each(|(p, (start, end))| {
                let view = &mut out[start as usize..end as usize];

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
            bound_radius: 0.0,
            num_cameras: self.inp_sizes.len() as _,
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
        for (i, sub) in self.sub_outs.iter().enumerate() {
            let rot = rm * (i as f32);

            let view = proj
                * Mat4::look_at_rh(
                    [0., 0., HEIGHT].into(),
                    [rot.sin(), rot.cos(), HEIGHT].into(),
                    glam::Vec3::Z,
                );

            sub.rend.cam.set_global(&view);

            let inv = view.inverse();

            out.push(inv);
        }
        self.last_sub_views = out;
    }

    pub fn update_bounding_verts(&mut self, vs: &[TexturedVertex]) {
        self.bounding_vertices.set_global(vs);
        self.bounding_vertices_len = vs.len();
    }

    #[inline]
    pub fn update_cam_specs<T>(&self, cams: &[Camera<T>]) {
        self.inp_specs.set_global(
            &cams
                .iter()
                .zip(&self.inp_sizes)
                .zip(&self.inp_starts)
                .map(|((c, res), start)| InputSpec::from_view(c.view, *res, *start))
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
                    | self.back.as_item()
                    | self.object_model.as_ref().map(Model::as_item)
                    | self
                        .bounding_cp
                        .vert_buf(&self.bounding_vertices)
                        .vert_range(0..(self.bounding_vertices_len as u32)),
            )
            .then(self.main_out.prepare())
            .submit();

        glob_gpu::force_wake();

        for SubOutput {
            rend,
            depth,
            depth_staging,
            depth_data: _,
            cp,
        } in &self.sub_outs
        {
            glob_gpu::command()
                .then(
                    Pass::render()
                        | &depth.depth_attach()
                        | &rend.texture.color_attach()
                        | cp.vert_buf(&self.back.verts)
                            .index_buf(&self.back.idx, 0..self.back.idx_len),
                )
                .then(depth.copy_to(&depth_staging))
                .then(rend.prepare())
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
    pub async fn wait_for_subs(
        &self,
        f: impl FnOnce(usize, InverseView, &[u8], DepthData<'_>) + Send + Clone,
    ) {
        let cpy_fut = AsyncMemMapper::fold_from(self.sub_outs.iter().enumerate(), |m, (i, sub)| {
            let f = f.clone();

            let (depth_done_send, depth_done) = kanal::oneshot_async();
            m.read_from(&sub.depth_staging, move |buf| async move {
                sub.depth_data
                    .lock()
                    .await
                    .copy_from_slice(<_>::ref_from_bytes(&*buf).expect("copy bug"));
                _ = depth_done_send.send(()).await;
            })
            .read_from(&sub.rend.staging, move |buf| async move {
                _ = depth_done.recv().await;
                f(
                    i,
                    InverseView(self.last_sub_views[i]),
                    &buf,
                    DepthData(Cow::Borrowed(&*sub.depth_data.lock().await), 640, 640),
                )
            })
        })
        .run();

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
            size,
        }
    }
}

pub struct GpuDirectBufferWrite {
    ctx: Arc<Context>,
    buf: Arc<Buffer>,
    offset: u64,
    size: u64,
}

impl OwnedWriteBuffer for GpuDirectBufferWrite {
    type View<'a>
        = smpgpu::DirectWritableBufferView<'a>
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Option<Self::View<'_>> {
        match self.size.try_into() {
            Ok(size) => Some(self.ctx.write_with(&self.buf, self.offset, size)),
            Err(_) => None,
        }
    }
}

pub struct DepthData<'a>(Cow<'a, [f32]>, u32, u32);

impl DepthData<'_> {
    #[inline]
    pub fn new_zeroed(width: usize, height: usize) -> Self {
        Self(vec![0.0; width * height].into(), width as _, height as _)
    }

    #[inline]
    pub fn copy_from(&mut self, src: &DepthData<'_>) {
        self.0.to_mut().copy_from_slice(&src.0);
    }

    #[inline]
    pub fn at(&self, x: u32, y: u32) -> f32 {
        self.0[(x.min(self.1 - 1) + y.min(self.2 - 1) * self.1) as usize]
    }

    #[inline]
    pub fn to_ref(&self) -> DepthData<'_> {
        DepthData(Cow::Borrowed(&self.0), self.1, self.2)
    }
}
