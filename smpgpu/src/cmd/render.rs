use std::ops::{BitOr, Range};

use encase::ShaderSize;

use crate::{
    shader::CompiledRenderShader,
    typed_buffer::{IndexBuffer, IndexBufferFormat},
    VertexBuffer,
};

use super::{Checkpoint, CheckpointBuilder, CheckpointItem, TypeOp};

pub type RenderCheckpoint = Checkpoint<wgpu::RenderPipeline>;

impl AsRenderItem for RenderCheckpoint {
    #[inline]
    fn as_item(&self) -> RenderItem<'_> {
        RenderItem {
            cp: self,
            vert_bufs: Vec::new(),
            vert_range: None,
            index_buf: Vec::new(),
            insts_range: 0..1,
        }
    }
}

pub struct RenderCheckpointItem<'a> {
    shader: CompiledRenderShader<'a>,
    cw_space: bool,
    cull: Option<wgpu::Face>,
    enable_depth: bool,
    vert_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    frag_targets: Vec<Option<wgpu::ColorTargetState>>,
}

impl<'a> CheckpointItem for RenderCheckpointItem<'a> {
    type ShaderType = CompiledRenderShader<'a>;
    type Pipeline = wgpu::RenderPipeline;

    #[inline]
    fn from_shader(shader: Self::ShaderType) -> Self {
        Self {
            shader,
            cw_space: false,
            cull: None,
            enable_depth: false,
            vert_buffers: Vec::new(),
            frag_targets: Vec::new(),
        }
    }

    fn build_pipeline(self, dev: &wgpu::Device, layout: wgpu::PipelineLayout) -> Self::Pipeline {
        let (vert_module, vert_entry, frag_module, frag_entry) = self
            .shader
            .split()
            .expect("no shader module in RenderShader");

        dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: vert_module,
                entry_point: vert_entry,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &self.vert_buffers,
            },
            primitive: wgpu::PrimitiveState {
                front_face: if self.cw_space {
                    wgpu::FrontFace::Cw
                } else {
                    wgpu::FrontFace::Ccw
                },
                cull_mode: self.cull,
                ..Default::default()
            },
            depth_stencil: self.enable_depth.then_some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: frag_module,
                entry_point: frag_entry,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &self.frag_targets,
            }),
            multiview: None,
            cache: None,
        })
    }
}

impl<'a> CheckpointBuilder<'a, RenderCheckpointItem<'a>> {
    #[inline]
    pub fn lh_coords(mut self) -> Self {
        self.data.cw_space = true;
        self
    }

    #[inline]
    pub fn cull_backface(mut self) -> Self {
        self.data.cull = Some(wgpu::Face::Back);
        self
    }

    #[inline]
    pub fn enable_depth(mut self) -> Self {
        self.data.enable_depth = true;
        self
    }

    #[inline]
    pub fn vert_buffer_of<T: ShaderSize>(
        mut self,
        attributes: &'a [wgpu::VertexAttribute],
    ) -> Self {
        self.data.vert_buffers.push(wgpu::VertexBufferLayout {
            array_stride: T::SHADER_SIZE.into(),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        });
        self
    }

    #[inline]
    pub fn frag_target(mut self, target: impl Into<wgpu::ColorTargetState>) -> Self {
        self.data.frag_targets.push(Some(target.into()));
        self
    }
}

pub struct FragTarget(wgpu::ColorTargetState);

impl From<FragTarget> for wgpu::ColorTargetState {
    #[inline]
    fn from(value: FragTarget) -> Self {
        value.0
    }
}

impl FragTarget {
    #[inline]
    pub fn new(f: wgpu::TextureFormat) -> Self {
        Self(f.into())
    }

    #[inline]
    pub fn use_transparency(mut self) -> Self {
        self.0.blend = Some(wgpu::BlendState::ALPHA_BLENDING);
        self
    }
}

#[derive(Default)]
pub struct RenderPass<'a> {
    color_attachs: Vec<Option<wgpu::RenderPassColorAttachment<'a>>>,
    depth_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
    items: Vec<RenderItem<'a>>,
}

impl RenderPass<'_> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with(mut self, op: impl TypeOp<Self>) -> Self {
        op.type_op(&mut self);
        self
    }
}

impl<'a, T: TypeOp<RenderPass<'a>>> BitOr<T> for RenderPass<'a> {
    type Output = RenderPass<'a>;

    #[inline]
    fn bitor(self, rhs: T) -> Self::Output {
        self.with(rhs)
    }
}

impl TypeOp<wgpu::CommandEncoder> for RenderPass<'_> {
    #[inline]
    fn type_op(self, enc: &mut wgpu::CommandEncoder) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &self.color_attachs,
            depth_stencil_attachment: self.depth_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        for p in self.items {
            p.add_to_pass(&mut pass);
        }
    }
}

impl<'a> TypeOp<RenderPass<'a>> for RenderItem<'a> {
    #[inline]
    fn type_op(self, pass: &mut RenderPass<'a>) {
        pass.items.push(self);
    }
}

impl<'a> TypeOp<RenderPass<'a>> for Option<RenderItem<'a>> {
    #[inline]
    fn type_op(self, pass: &mut RenderPass<'a>) {
        if let Some(it) = self {
            pass.items.push(it);
        }
    }
}

pub struct ColorAttachment {
    view: wgpu::TextureView,
    ops: wgpu::Operations<wgpu::Color>,
}

impl ColorAttachment {
    #[inline]
    pub fn new(view: wgpu::TextureView) -> Self {
        Self {
            view,
            ops: wgpu::Operations::default(),
        }
    }

    #[inline]
    pub const fn load_clear(mut self, [r, g, b, a]: [f64; 4]) -> Self {
        self.ops.load = wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a });
        self
    }

    #[inline]
    pub const fn store(mut self) -> Self {
        self.ops.store = wgpu::StoreOp::Store;
        self
    }
}

impl<'a> TypeOp<RenderPass<'a>> for &'a ColorAttachment {
    #[inline]
    fn type_op(self, pass: &mut RenderPass<'a>) {
        pass.color_attachs
            .push(Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                resolve_target: None,
                ops: self.ops,
            }));
    }
}

pub struct DepthAttachment {
    view: wgpu::TextureView,
}

impl DepthAttachment {
    #[inline]
    pub fn new(view: wgpu::TextureView) -> Self {
        Self { view }
    }
}

impl<'a> TypeOp<RenderPass<'a>> for &'a DepthAttachment {
    #[inline]
    fn type_op(self, pass: &mut RenderPass<'a>) {
        pass.depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        })
    }
}

pub struct RenderItem<'a> {
    cp: &'a RenderCheckpoint,
    vert_bufs: Vec<&'a wgpu::Buffer>,
    vert_range: Option<Range<u32>>,
    index_buf: Vec<(&'a wgpu::Buffer, wgpu::IndexFormat, Range<u32>)>,
    insts_range: Range<u32>,
}

impl<'a> RenderItem<'a> {
    #[inline]
    pub fn vert_buf<T>(self, buf: &'a VertexBuffer<T>) -> Self {
        self.raw_vert_buf(&buf.0).vert_range(0..buf.1)
    }

    #[inline]
    pub fn raw_vert_buf(mut self, buf: &'a wgpu::Buffer) -> Self {
        self.vert_bufs.push(buf);
        self
    }

    #[inline]
    pub fn vert_range(mut self, range: Range<u32>) -> Self {
        self.vert_range = self.vert_range.or(Some(range));
        self
    }

    #[inline]
    pub fn index_buf<T: IndexBufferFormat>(
        self,
        buf: &'a IndexBuffer<T>,
        range: Range<u32>,
    ) -> Self {
        self.raw_index_buf::<T>(&buf.0, range)
    }

    #[inline]
    pub fn raw_index_buf<T: IndexBufferFormat>(
        mut self,
        buf: &'a wgpu::Buffer,
        range: Range<u32>,
    ) -> Self {
        self.index_buf.push((buf, T::index_format(), range));
        self
    }

    #[inline]
    pub(crate) fn add_to_pass(&self, pass: &mut wgpu::RenderPass) {
        let Some(vert_range) = &self.vert_range else {
            panic!("no vertex buffers added to render pipeline!");
        };

        pass.set_pipeline(&self.cp.pipeline);

        for (i, g) in self.cp.groups.iter().enumerate() {
            pass.set_bind_group(i as _, g, &[]);
        }

        for (i, s) in self.vert_bufs.iter().enumerate() {
            pass.set_vertex_buffer(i as _, s.slice(..));
        }

        if self.index_buf.is_empty() {
            pass.draw(vert_range.clone(), self.insts_range.clone());
            return;
        }

        for (b, f, indices) in &self.index_buf {
            pass.set_index_buffer(b.slice(..), *f);
            pass.draw_indexed(indices.clone(), 0, self.insts_range.clone());
        }
    }
}

pub trait AsRenderItem {
    fn as_item(&self) -> RenderItem<'_>;

    #[inline]
    fn vert_buf<'a, T>(&'a self, buf: &'a VertexBuffer<T>) -> RenderItem<'a> {
        self.as_item().vert_buf(buf)
    }

    #[inline]
    fn raw_vert_buf<'a>(&'a self, buf: &'a wgpu::Buffer) -> RenderItem<'a> {
        self.as_item().raw_vert_buf(buf)
    }

    #[inline]
    fn vert_range(&self, range: Range<u32>) -> RenderItem<'_> {
        self.as_item().vert_range(range)
    }

    #[inline]
    fn index_buf<'a, T: IndexBufferFormat>(
        &'a self,
        buf: &'a IndexBuffer<T>,
        range: Range<u32>,
    ) -> RenderItem<'a> {
        self.as_item().index_buf::<T>(buf, range)
    }

    #[inline]
    fn raw_index_buf<'a, T: IndexBufferFormat>(
        &'a self,
        buf: &'a wgpu::Buffer,
        range: Range<u32>,
    ) -> RenderItem<'a> {
        self.as_item().raw_index_buf::<T>(buf, range)
    }
}
