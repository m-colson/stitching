use std::{borrow::Cow, path::PathBuf, sync::Arc};

use cam_loader::{FrameSize, Loader, OwnedWritable, OwnedWriteBuffer, util::log_recv_err};
use encase::ShaderType;
use glam::{Mat4, UVec2};
use smpgpu::{
    AsRenderItem, AutoVisBindable, Buffer, Context, CopyOp, MemMapper, Pass, StorageBuffer,
    Texture, Uniform, VertexBuffer,
    global::prelude::*,
    model::{Model, ModelBuilder, RendModel, VertPosNorm},
};
use tokio::sync::Mutex;
use zerocopy::FromZeros;

use crate::{
    Error, Result,
    camera::{Camera, Config, ViewParams},
};

use super::{MaskLoaderConfig, ViewStyle, WorldStyle};

/// Contains the settings and gpu buffers needed for projection.
pub struct GpuProjector {
    out_size: (usize, usize),

    pass_info: Uniform<PassInfo>,
    inp: GpuInputs,

    world: Model<Vertex, u16>,
    object_model: Option<ObjectModel>,

    bounds: Arc<GpuBounds>,
}

/// Contains the buffers that contain the input settings, frame data and mask data.
struct GpuInputs {
    /// Storage buffer containing ALL camera's frames back-to-back.
    /// Each pixel is a u32 containing the bytes with a RGBA format.
    pub frames: Arc<StorageBuffer<u32>>,
    pub specs: StorageBuffer<InputSpec>,
    pub sizes: Vec<glam::UVec2>,
    pub starts: Vec<u32>,
    pub masks: StorageBuffer<u32>,
}

impl GpuInputs {
    /// Creates the input buffers that can contain each camera's frame and loads the mask images.
    pub fn new(sizes: &[UVec2], mask_paths: &[Option<PathBuf>]) -> Self {
        let ranges = sizes
            .iter()
            .scan(0, |o, v| {
                let out = *o;
                *o += v.x * v.y;
                Some((out, *o))
            })
            .collect::<Vec<_>>();

        let frames = buffer("inp_frames")
            .storage()
            .len(ranges.last().unwrap().1 as _)
            .writable()
            .build();

        let specs = buffer("inp_specs")
            .storage()
            .len(sizes.len() as _)
            .writable()
            .build();

        let mut mask_data =
            <[u32]>::new_box_zeroed_with_elems(ranges.last().unwrap().1 as _).unwrap();

        for (p, &(start, end)) in mask_paths.iter().zip(&ranges) {
            let view = &mut mask_data[start as usize..end as usize];

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
        }

        let masks = buffer("inp_masks")
            .storage()
            .writable()
            .init_data(&mask_data)
            .build();

        Self {
            frames: Arc::new(frames),
            specs,
            sizes: sizes.to_vec(),
            starts: ranges.into_iter().map(|v| v.0).collect(),
            masks,
        }
    }
}

struct ObjectModel {
    pub model: Model<VertPosNorm, u32>,
    pub opts: ModelOptions,
}

impl ObjectModel {
    pub fn as_rend_model(&self, out: &RenderOutput) -> RendModel<VertPosNorm, u32> {
        self.model.rend_with_cam_cp_global(&out.cam, |cp| {
            cp.group(self.opts.light_dir.in_frag())
                .shader(smpgpu::include_shader!("shaders/model.wgsl"))
                .cull_backface()
                .enable_depth()
                .vert_buffer_of::<VertPosNorm>(
                    &smpgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
                )
                .target_format(out.texture.format())
                .build()
        })
    }
}

struct GpuBounds {
    pub verts: Mutex<VertexBuffer<TexturedVertex>>,
}

impl GpuBounds {
    pub fn new() -> Self {
        let verts = buffer("bound_verts").vertex().len(4096).writable().build();
        Self {
            verts: Mutex::new(verts),
        }
    }

    pub async fn update(&self, vs: &[TexturedVertex]) {
        self.verts.lock().await.set_global(vs);
    }
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
        let rev_mat = glam::Mat3::from_euler(glam::EulerRot::YXZ, s.roll, s.pitch, s.azimuth);

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
        let cam = uniform("cam").writable().build();
        let texture = texture("out_texture")
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

/// Represents a single bouding box vertex.
#[derive(ShaderType, Clone, Copy, Debug)]
pub struct TexturedVertex {
    /// The 3d-coordinate of the vertex, pos.w is 1.
    pub pos: glam::Vec4,
    /// The RGBA color components (0-1) of the vertex.
    pub color: glam::Vec4,
    /// The location of the vertex in to the bounding box.
    pub text_coord: glam::Vec2,
}

impl TexturedVertex {
    /// Create a new vertex at `[x,y,z]`, texture coordinate `[tx,ty]` and the provided `color`.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, tx: f32, ty: f32, color: glam::Vec3) -> Self {
        Self {
            pos: glam::vec4(x, y, z, 1.),
            text_coord: glam::vec2(tx, ty),
            color: color.extend(1.),
        }
    }

    /// Create a new vertex at `pos`, texture coordinate `[tx,ty]` and the provided `color`.
    #[inline]
    pub fn from_pos(pos: glam::Vec4, tx: f32, ty: f32, color: glam::Vec3) -> Self {
        Self {
            pos,
            text_coord: glam::vec2(tx, ty),
            color: color.extend(1.),
        }
    }
}

/// Stores the settings needed to create a [`GpuProjector`].
pub struct GpuProjectorBuilder<'a> {
    input_sizes: Vec<glam::UVec2>,
    out_size: (usize, usize),
    world_verts: Vec<Vertex>,
    world_idxs: Vec<u16>,
    mask_paths: Vec<Option<PathBuf>>,
    model_builder: Option<(
        smpgpu::model::ModelBuilder<'a, smpgpu::model::VertPosNorm, u32>,
        ModelOptions,
    )>,
    model_origin: glam::Vec3,
    model_scale: glam::Vec3,
    model_rot: glam::Vec3,
}

/// Stores settings needed for the model not already in [`smpgpu::model::ModelBuilder`].
pub struct ModelOptions {
    pub light_dir: Uniform<glam::Vec3>,
}

impl<'a> GpuProjectorBuilder<'a> {
    /// Creates a new builder with default settings.
    const fn new() -> Self {
        Self {
            input_sizes: Vec::new(),
            out_size: (0, 0),
            world_verts: Vec::new(),
            world_idxs: Vec::new(),
            mask_paths: Vec::new(),
            model_builder: None,
            model_origin: glam::vec3(0., 0., 0.),
            model_scale: glam::vec3(1., 1., 1.),
            model_rot: glam::vec3(0., 0., 0.),
        }
    }

    /// Adds inputs with `input_sizes` to the settings.
    pub fn input_sizes<T: Into<glam::UVec2>>(
        mut self,
        input_sizes: impl IntoIterator<Item = T>,
    ) -> Self {
        self.input_sizes
            .extend(input_sizes.into_iter().map(|s| s.into()));
        self
    }

    /// Sets the size of the primary view to `w`x`h`.
    pub const fn out_size(mut self, w: usize, h: usize) -> Self {
        self.out_size = (w, h);
        self
    }

    /// Sets a new world mesh based on the world style. See [`WorldStyle::make_mesh`].
    pub fn world(mut self, world: &WorldStyle) -> Self {
        (self.world_verts, self.world_idxs) = world.make_mesh();
        self
    }

    /// Sets the paths to the camera masks from the provided camera configs.
    pub fn masks_from_cfgs(mut self, cfgs: &[Config<MaskLoaderConfig>]) -> Self {
        self.mask_paths = cfgs.iter().map(|c| c.meta.mask_path.clone()).collect();
        self
    }

    /// Constructs or reuses a [`ModelBuilder`] and gives it to the callback to configure further.
    pub fn model(
        mut self,
        f: impl FnOnce(
            ModelBuilder<'a, smpgpu::model::VertPosNorm, u32>,
            &mut ModelOptions,
        ) -> ModelBuilder<'a, smpgpu::model::VertPosNorm, u32>,
    ) -> Self {
        let (m, mut opts) = self.model_builder.take().unwrap_or_else(|| {
            (
                model(),
                ModelOptions {
                    light_dir: buffer("light_dir").uniform().writable().build(),
                },
            )
        });
        self.model_builder = Some((f(m, &mut opts), opts));
        self
    }

    /// Sets the model origin.
    pub fn model_origin(mut self, origin: [f32; 3]) -> Self {
        self.model_origin = origin.into();
        self
    }

    /// Sets the model axis scales.
    pub fn model_scale(mut self, scale: [f32; 3]) -> Self {
        self.model_scale = scale.into();
        self
    }

    /// Sets the model rotation angles, in degrees.
    pub fn model_rot_deg(mut self, angles: [f32; 3]) -> Self {
        self.model_rot = angles.map(|v| v.to_radians()).into();
        self
    }

    /// Creates a [`GpuProjector`] using `self`'s settings.
    pub fn build(self) -> GpuProjector {
        let pass_info = uniform("pass_info")
            .writable()
            .init(&PassInfo {
                bound_radius: 0.0,
                num_cameras: self.input_sizes.len() as _,
            })
            .build();

        let inp = GpuInputs::new(&self.input_sizes, &self.mask_paths);

        let world = model()
            .verts(&self.world_verts)
            .indices(&self.world_idxs)
            .build();

        let object_model = self.model_builder.map(|(b, opts)| {
            let model = b.build().with_view(
                Mat4::from_euler(
                    glam::EulerRot::ZXY,
                    self.model_rot[0],
                    self.model_rot[1],
                    self.model_rot[2],
                ) * Mat4::from_translation(-self.model_origin)
                    * Mat4::from_scale(self.model_scale),
            );

            ObjectModel { model, opts }
        });

        let bounds = Arc::new(GpuBounds::new());

        GpuProjector {
            out_size: self.out_size,
            pass_info,
            inp,
            world,
            object_model,
            bounds,
        }
    }
}

impl GpuProjector {
    /// Creates a new [`GpuProjectorBuilder`].
    #[inline]
    pub fn builder() -> GpuProjectorBuilder<'static> {
        GpuProjectorBuilder::new()
    }

    /// Creates a [`ProjectionView`] with the provided width, height and [`ViewStyle`].
    /// Will render the world, model (if given), and bounding boxes.
    pub fn create_vis_view<B>(&self, w: usize, h: usize, init_view: ViewStyle) -> ProjectionView<B>
    where
        B: OwnedWriteBuffer + Send + 'static,
        for<'a> B::View<'a>: Send,
    {
        let rend_out = RenderOutput::new(w, h);
        rend_out
            .cam
            .set_global(&init_view.transform_matrix(w as _, h as _));

        let depth = texture("depth_texture")
            .size(self.out_size.0, self.out_size.1)
            .depth()
            .build();

        let world = self.world.rend_with_cam_cp_global(&rend_out.cam, |cp| {
            cp.group(
                self.pass_info.in_frag()
                    & self.inp.frames.in_frag()
                    & self.inp.specs.in_frag()
                    & self.inp.masks.in_frag(),
            )
            .shader(smpgpu::include_shader!("shaders/render.wgsl" => "vs_proj" & "fs_proj"))
            .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
            .target_format(rend_out.texture.format())
            .enable_depth()
            .build()
        });

        let object_model = self
            .object_model
            .as_ref()
            .map(|m| m.as_rend_model(&rend_out));

        let bounds = self.bounds.clone();
        let bound_cp = checkpoint()
            .group(world.view.in_vertex() & rend_out.cam.in_vertex())
            .shader(smpgpu::include_shader!("shaders/bounds.wgsl"))
            .enable_depth()
            .vert_buffer_of::<TexturedVertex>(
                &smpgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x2],
            )
            .target_format(rend_out.texture.as_frag_target().use_transparency())
            .build();

        let (update_send, updates) = kanal::unbounded();

        let loader = Loader::<B>::new_async_manual_recv(w as _, h as _, 4, |req_recv| async move {
            let mut style = ViewStyle::default();

            while let Ok((mut req, resp_send)) = req_recv.recv().await.inspect_err(log_recv_err) {
                while let Ok(Some(upd)) = updates.try_recv() {
                    match upd {
                        ProjUpdater::View(upd) => {
                            upd(&mut style);

                            let view = style.transform_matrix(w as f32, h as f32);
                            rend_out.cam.set_global(&view);
                        }
                    };
                }

                if let Some(mut v) = req.owned_to_view() {
                    let verts = bounds.verts.lock().await;
                    command()
                        .then(
                            Pass::render()
                                | &depth.depth_attach()
                                | &rend_out.texture.color_attach()
                                | world.as_item()
                                | object_model.as_ref().map(RendModel::as_item)
                                | bound_cp.vert_buf(&verts),
                        )
                        .then(rend_out.prepare())
                        .submit();

                    MemMapper::new()
                        .copy(&rend_out.staging, v.as_mut())
                        .run_all()
                        .await;
                }

                // if the receiver has been dropped, they don't want their buffer back!
                _ = resp_send.send(req);
            }
        });

        update_send
            .send(ProjUpdater::View(Box::new(move |v| *v = init_view)))
            .unwrap();

        ProjectionView {
            loader,
            update_send,
        }
    }

    /// Creates a [`ProjectionView`] with the provided width, height and [`ViewStyle`].
    /// Will only render the world but returns the (image data, f32 vertex depths).
    pub fn create_depth_view<B1, B2>(&self, w: usize, h: usize) -> ProjectionView<(B1, B2)>
    where
        B1: OwnedWriteBuffer + Send + 'static,
        for<'a> B1::View<'a>: Send,
        B2: OwnedWriteBuffer + Send + 'static,
        for<'a> B2::View<'a>: Send,
    {
        let rend_out = RenderOutput::new(w, h);
        let depth = texture("depth_texture")
            .size(w, h)
            .depth()
            .readable()
            .build();
        let depth_staging = depth.new_staging_global();

        let rend_shader = smpgpu::include_shader!("shaders/render.wgsl" => "vs_proj" & "fs_proj");

        let world = self.world.rend_with_cam_cp_global(&rend_out.cam, |cp| {
            cp.group(
                self.pass_info.in_frag()
                    & self.inp.frames.in_frag()
                    & self.inp.specs.in_frag()
                    & self.inp.masks.in_frag(),
            )
            .shader(rend_shader)
            .vert_buffer_of::<Vertex>(&smpgpu::vertex_attr_array![0 => Float32x4])
            .target_format(rend_out.texture.format())
            .enable_depth()
            .build()
        });

        let (update_send, updates) = kanal::unbounded();

        let loader =
            Loader::<(B1, B2)>::new_async_manual_recv(w as _, h as _, 4, |req_recv| async move {
                let mut style = ViewStyle::default();

                while let Ok((mut req, resp_send)) = req_recv.recv().await.inspect_err(log_recv_err)
                {
                    while let Ok(Some(upd)) = updates.try_recv() {
                        match upd {
                            ProjUpdater::View(upd) => {
                                upd(&mut style);

                                let view = style.transform_matrix(w as f32, h as f32);
                                rend_out.cam.set_global(&view)
                            }
                        };
                    }

                    if let Some((mut img, mut depth_buf)) = req.owned_to_inner() {
                        command()
                            .then(
                                Pass::render()
                                    | &depth.depth_attach()
                                    | &rend_out.texture.color_attach()
                                    | world.as_item(),
                            )
                            .then(depth.copy_to(&depth_staging))
                            .then(rend_out.prepare())
                            .submit();

                        MemMapper::new()
                            .copy(&rend_out.staging, img.as_mut())
                            .copy(&depth_staging, depth_buf.as_mut())
                            .run_all()
                            .await;
                    }

                    // if the receiver has been dropped, they don't want their buffer back!
                    _ = resp_send.send(req);
                }
            });

        ProjectionView {
            loader,
            update_send,
        }
    }

    /// Sets the vertices for the rendered bounding boxes.
    pub async fn update_bounding_verts(&mut self, vs: &[TexturedVertex]) {
        self.bounds.update(vs).await;
    }

    /// Sets the input specifications based on the provided cameras' views.
    #[inline]
    pub fn update_cam_specs<T>(&self, cams: &[Camera<T>]) {
        self.inp.specs.set_global(
            &std::iter::zip(&self.inp.sizes, &self.inp.starts)
                .zip(cams)
                .map(|((&res, &start), c)| InputSpec::from_view(c.view, res, start))
                .collect::<Vec<_>>(),
        );
    }

    /// Send requests to each camera for the next frame. See [`Loader::give`].
    #[inline]
    pub fn take_input_buffers(
        &self,
        cams: &[Camera<Loader<GpuDirectBufferWrite>>],
    ) -> Result<Vec<cam_loader::Ticket<GpuDirectBufferWrite>>> {
        cams.iter()
            .scan(0, |off, c| {
                let size = c.data.num_bytes() as u64;
                let buf_off = *off;
                *off += size;

                Some(
                    c.data
                        .give(self.inp_buffer_write(buf_off, size))
                        .map_err(Error::Loader),
                )
            })
            .collect()
    }

    #[inline]
    fn inp_buffer_write(&self, offset: u64, size: u64) -> GpuDirectBufferWrite {
        GpuDirectBufferWrite {
            ctx: smpgpu::global::get_global_context(),
            buf: self.inp.frames.clone(),
            offset,
            size,
        }
    }
}

/// Handle to a projector and the channels to retrieve projected frames and
/// send updates.
pub struct ProjectionView<B> {
    loader: Loader<B>,
    update_send: kanal::Sender<ProjUpdater>,
}

pub enum ProjUpdater {
    View(Box<dyn FnOnce(&mut ViewStyle) + Send>),
}

impl<B> ProjectionView<B> {
    /// Mutate the projector's current [`ViewStyle`].
    pub fn update_view(&self, f: impl FnOnce(&mut ViewStyle) + Send + 'static) -> Result<()> {
        self.update_send
            .send(ProjUpdater::View(Box::new(f)))
            .map_err(|_| Error::Other("projection view closed".to_string()))
    }
}

impl<B: OwnedWriteBuffer + Send + 'static> ProjectionView<B> {
    /// Load a stitched view into `buf` and wait for it to be updated.
    pub async fn load_image(&self, buf: B) -> Result<B> {
        let ticket = self.loader.give(buf)?;
        let buf = ticket.take().await?;
        Ok(buf)
    }
}

impl<B1: OwnedWriteBuffer + Send + 'static, B2: OwnedWriteBuffer + Send + 'static>
    ProjectionView<(B1, B2)>
{
    /// Load a stitch view and depth data into buf1 and buf2 and wait for it to be updated.
    pub async fn load_image2(&self, buf1: B1, buf2: B2) -> Result<(B1, B2)> {
        let ticket = self.loader.give2(buf1, buf2)?;
        let buf = ticket.take().await?;
        Ok(buf)
    }
}

/// Contains a shared reference to a [`StorageBuffer`] and the location to save a result to.
pub struct GpuDirectBufferWrite {
    ctx: Arc<Context>,
    buf: Arc<StorageBuffer<u32>>,
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
            Ok(size) => Some(
                self.ctx
                    .write_with(self.buf.as_untyped(), self.offset, size),
            ),
            Err(_) => {
                tracing::error!("attempted to copy zero bytes, ignoring...");
                None
            }
        }
    }
}

/// Wrapper over a list of `f32`s that stores the depth data found during the
/// rendering of a depth view.
pub struct DepthData<'a>(Cow<'a, [f32]>, u32, u32);

impl DepthData<'_> {
    /// Creates a new buffer than can store data for an image of size `width`x`height`.
    #[inline]
    pub fn new_zeroed(width: usize, height: usize) -> Self {
        Self(vec![0.0; width * height].into(), width as _, height as _)
    }

    /// Copies `src` into the `self`.
    /// # Panics
    /// This function will panic if `self` and `src` have different lengths.
    #[inline]
    pub fn copy_from(&mut self, src: &'_ [f32]) {
        self.0.to_mut().copy_from_slice(src);
    }

    /// Returns the depth at `x` and `y` in `self`.
    #[inline]
    pub fn at(&self, x: u32, y: u32) -> f32 {
        self.0[(x.min(self.1 - 1) + y.min(self.2 - 1) * self.1) as usize]
    }

    /// Returns another [`DepthData`] that references `self` instead of cloning.
    #[inline]
    pub fn to_ref(&self) -> DepthData<'_> {
        DepthData(Cow::Borrowed(&self.0), self.1, self.2)
    }

    /// Returns the total amount of elements in `self`.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if `self` has no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
