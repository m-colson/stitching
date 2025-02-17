#![allow(dead_code)]

use crate::{bind::IntoBindGroup, Context, OntoDevice};

pub(crate) mod compute;
pub use compute::{ComputeCheckpoint, ComputeItem, ComputePass};

pub(crate) mod render;
pub use render::{
    AsRenderItem, ColorAttachment, DepthAttachment, FragTarget, RenderCheckpoint, RenderItem,
    RenderPass,
};

/// Contains the information necessary to build a specific command without
/// the need for lifetimes.
pub struct Checkpoint<T> {
    groups: Box<[wgpu::BindGroup]>,
    pipeline: T,
}

impl Checkpoint<()> {
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> CheckpointBuilder<'_, ()> {
        CheckpointBuilder::new(dev.as_ref())
    }
}

pub struct CheckpointBuilder<'a, T = ()> {
    dev: &'a wgpu::Device,
    groups: Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)>,
    data: T,
}

impl<'a> CheckpointBuilder<'a> {
    pub(crate) fn new(dev: &'a wgpu::Device) -> Self {
        Self {
            dev,
            groups: Vec::new(),
            data: (),
        }
    }

    pub fn group(mut self, b: impl IntoBindGroup) -> Self {
        self.groups.push(b.into_wgpu_bind_group(self.dev));
        self
    }

    pub fn shader<T: CheckpointItem>(
        self,
        shader: impl OntoDevice<T::ShaderType>,
    ) -> CheckpointBuilder<'a, T> {
        CheckpointBuilder {
            dev: self.dev,
            groups: self.groups,
            data: T::from_shader(shader.onto_device(self.dev)),
        }
    }
}

impl<T: CheckpointItem> CheckpointBuilder<'_, T> {
    pub fn build(self) -> Checkpoint<T::Pipeline> {
        let layout = self
            .dev
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &self.groups.iter().map(|(l, _)| l).collect::<Vec<_>>(),
                push_constant_ranges: &[],
            });

        let pipeline = self.data.build_pipeline(self.dev, layout);
        let groups = self.groups.into_iter().map(|(_, g)| g).collect();
        Checkpoint { groups, pipeline }
    }
}

pub trait CheckpointItem {
    type ShaderType;
    type Pipeline;
    fn from_shader(shader: Self::ShaderType) -> Self;
    fn build_pipeline(self, dev: &wgpu::Device, layout: wgpu::PipelineLayout) -> Self::Pipeline;
}

pub struct CommandBuilder<'a> {
    ctx: &'a Context,
    encoder: wgpu::CommandEncoder,
}

impl<'a> CommandBuilder<'a> {
    pub fn new(ctx: &'a Context) -> Self {
        let encoder = ctx
            .as_ref()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        Self { ctx, encoder }
    }

    #[inline]
    pub fn then(mut self, op: impl TypeOp<wgpu::CommandEncoder>) -> Self {
        op.type_op(&mut self.encoder);
        self
    }

    #[inline]
    pub fn submit(self) {
        self.ctx.submit([self.build()]);
    }

    #[inline]
    fn build(self) -> wgpu::CommandBuffer {
        self.encoder.finish()
    }
}

pub struct Pass;

impl Pass {
    pub fn render() -> RenderPass<'static> {
        RenderPass::new()
    }

    pub fn compute() -> ComputePass<'static> {
        ComputePass::new()
    }
}

pub trait TypeOp<T> {
    fn type_op(self, other: &mut T);
}

pub enum CopyOp<'a> {
    TextBuf(
        &'a wgpu::Texture,
        wgpu::Origin3d,
        wgpu::TextureAspect,
        &'a wgpu::Buffer,
        wgpu::Extent3d,
    ),
    BufText(
        &'a wgpu::Buffer,
        &'a wgpu::Texture,
        wgpu::Origin3d,
        wgpu::TextureAspect,
        wgpu::Extent3d,
    ),
    BufBuf(
        &'a wgpu::Buffer,
        wgpu::BufferAddress,
        &'a wgpu::Buffer,
        wgpu::BufferAddress,
        wgpu::BufferAddress,
    ),
}

impl TypeOp<wgpu::CommandEncoder> for CopyOp<'_> {
    fn type_op(self, enc: &mut wgpu::CommandEncoder) {
        match self {
            CopyOp::TextBuf(texture, origin, aspect, buffer, ext) => enc.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin,
                    aspect,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(ext.width * 4),
                        rows_per_image: Some(ext.height),
                    },
                },
                ext,
            ),
            CopyOp::BufText(buffer, texture, origin, aspect, ext) => enc.copy_buffer_to_texture(
                wgpu::TexelCopyBufferInfo {
                    buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(ext.width * 4),
                        rows_per_image: Some(ext.height * 4),
                    },
                },
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin,
                    aspect,
                },
                ext,
            ),
            CopyOp::BufBuf(src, src_off, dst, dst_off, size) => {
                enc.copy_buffer_to_buffer(src, src_off, dst, dst_off, size);
            }
        }
    }
}
