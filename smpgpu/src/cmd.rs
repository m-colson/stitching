#![allow(dead_code)]

use std::ops::Range;

use encase::ShaderSize;
use wgpu::ComputePassDescriptor;

use crate::{bind::IntoBindGroup, Buffer};

pub struct ComputeCheckpoint {
    groups: Vec<wgpu::BindGroup>,
    pipeline: wgpu::ComputePipeline,
    work_groups: [u32; 3],
}

impl ComputeCheckpoint {
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> ComputeCheckpointBuilder<'_> {
        ComputeCheckpointBuilder::new(dev.as_ref())
    }

    pub fn work_groups(mut self, x: usize, y: usize, z: usize) -> Self {
        self.work_groups = [x as _, y as _, z as _];
        self
    }

    pub fn encoder(&self, dev: &impl AsRef<wgpu::Device>) -> CommandBuilder {
        let dev = dev.as_ref();
        let mut encoder =
            dev.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        for (i, g) in self.groups.iter().enumerate() {
            compute_pass.set_bind_group(i as _, g, &[]);
        }
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.dispatch_workgroups(
            self.work_groups[0],
            self.work_groups[1],
            self.work_groups[2],
        );

        CommandBuilder { encoder }
    }
}

pub struct ComputeCheckpointBuilder<'a> {
    dev: &'a wgpu::Device,
    groups: Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)>,
    shader: Option<wgpu::ShaderModule>,
    entry: Option<&'a str>,
}

impl<'a> ComputeCheckpointBuilder<'a> {
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self {
            dev,
            groups: Vec::new(),
            shader: None,
            entry: None,
        }
    }

    pub fn group(mut self, b: impl IntoBindGroup) -> Self {
        let (layout, group) = b.into_wgpu_bind_group(self.dev);
        self.groups.push((layout, group));
        self
    }

    pub fn shader(
        mut self,
        desc: wgpu::ShaderModuleDescriptor,
        entry: impl Into<Option<&'a str>>,
    ) -> Self {
        self.shader = Some(self.dev.create_shader_module(desc));
        self.entry = entry.into();
        self
    }

    pub fn build(self) -> ComputeCheckpoint {
        let pipeline_layout = self
            .dev
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &self.groups.iter().map(|(l, _)| l).collect::<Vec<_>>(),
                push_constant_ranges: &[],
            });

        let pipeline = self
            .dev
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &self.shader.expect("no shader provided to compute builder"),
                entry_point: self.entry,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let groups = self.groups.into_iter().map(|(_, g)| g).collect();

        ComputeCheckpoint {
            groups,
            pipeline,
            work_groups: [0; 3],
        }
    }
}

pub struct RenderCheckpoint {
    groups: Box<[wgpu::BindGroup]>,
    pipeline: wgpu::RenderPipeline,
    vert_range: Range<u32>,
    insts_range: Range<u32>,
}

impl RenderCheckpoint {
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> RenderCheckpointBuilder {
        RenderCheckpointBuilder::new(dev)
    }

    pub fn encoder<'a>(&'a self, dev: &'a impl AsRef<wgpu::Device>) -> RenderCommandBuilder {
        RenderCommandBuilder {
            dev: dev.as_ref(),
            cp: self,
            color_attachs: Vec::new(),
            vert_bufs: Vec::new(),
            index_buf: None,
        }
    }

    pub fn vertices(mut self, range: Range<u32>) -> Self {
        self.vert_range = range;
        self
    }

    pub fn instances(mut self, range: Range<u32>) -> Self {
        self.insts_range = range;
        self
    }
}

pub struct RenderCheckpointBuilder<'a> {
    dev: &'a wgpu::Device,
    groups: Vec<(wgpu::BindGroupLayout, wgpu::BindGroup)>,
    vert_shader: Option<wgpu::ShaderModule>,
    vert_entry: Option<&'a str>,
    vert_buffers: Vec<wgpu::VertexBufferLayout<'a>>,
    frag_shader: Option<wgpu::ShaderModule>,
    frag_entry: Option<&'a str>,
    frag_targets: Vec<Option<wgpu::ColorTargetState>>,
}

impl<'a> RenderCheckpointBuilder<'a> {
    pub fn new(dev: &'a impl AsRef<wgpu::Device>) -> Self {
        Self {
            dev: dev.as_ref(),
            groups: Vec::new(),
            vert_shader: None,
            vert_entry: None,
            vert_buffers: Vec::new(),
            frag_shader: None,
            frag_entry: None,
            frag_targets: Vec::new(),
        }
    }

    pub fn group(mut self, b: impl IntoBindGroup) -> Self {
        let (layout, group) = b.into_wgpu_bind_group(self.dev);
        self.groups.push((layout, group));
        self
    }

    pub fn vert_shader(
        mut self,
        desc: impl Into<Option<wgpu::ShaderModuleDescriptor<'a>>>,
        entry: impl Into<Option<&'a str>>,
    ) -> Self {
        self.vert_shader = desc.into().map(|desc| self.dev.create_shader_module(desc));
        self.vert_entry = entry.into();
        self
    }

    pub fn vert_buffer_of<T: ShaderSize>(
        mut self,
        attributes: &'a [wgpu::VertexAttribute],
    ) -> Self {
        self.vert_buffers.push(wgpu::VertexBufferLayout {
            array_stride: T::SHADER_SIZE.into(),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        });
        self
    }

    pub fn frag_shader(
        mut self,
        desc: impl Into<Option<wgpu::ShaderModuleDescriptor<'a>>>,
        entry: impl Into<Option<&'a str>>,
    ) -> Self {
        self.frag_shader = desc.into().map(|desc| self.dev.create_shader_module(desc));
        self.frag_entry = entry.into();
        self
    }

    pub fn frag_target(mut self, target: impl Into<wgpu::ColorTargetState>) -> Self {
        self.frag_targets.push(Some(target.into()));
        self
    }

    pub fn build(self) -> RenderCheckpoint {
        let pipeline_layout = self
            .dev
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &self.groups.iter().map(|(l, _)| l).collect::<Vec<_>>(),
                push_constant_ranges: &[],
            });

        let (vert_shader, frag_shader) = match (&self.vert_shader, &self.frag_shader) {
            (Some(vs), Some(fs)) => (vs, fs),
            (_, Some(s)) | (Some(s), _) => (s, s),
            _ => panic!("no shader provided to render builder"),
        };

        let pipeline = self
            .dev
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: vert_shader,
                    entry_point: self.vert_entry,
                    compilation_options: Default::default(),
                    buffers: &self.vert_buffers,
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: frag_shader,
                    entry_point: self.frag_entry,
                    compilation_options: Default::default(),
                    targets: &self.frag_targets,
                }),
                multiview: None,
                cache: None,
            });

        let groups = self.groups.into_iter().map(|(_, g)| g).collect();

        RenderCheckpoint {
            groups,
            pipeline,
            vert_range: 0..0,
            insts_range: 0..1,
        }
    }
}

pub struct RenderCommandBuilder<'a> {
    dev: &'a wgpu::Device,
    cp: &'a RenderCheckpoint,
    color_attachs: Vec<Option<wgpu::RenderPassColorAttachment<'a>>>,
    vert_bufs: Vec<wgpu::BufferSlice<'a>>,
    index_buf: Option<(wgpu::BufferSlice<'a>, wgpu::IndexFormat, Range<u32>)>,
}

impl<'a> RenderCommandBuilder<'a> {
    #[inline]
    pub fn attach(mut self, op: &'a impl RenderAttachOp) -> Self {
        op.attach_op(&mut self);
        self
    }

    #[inline]
    pub fn vert_buf(mut self, buf: &'a Buffer) -> Self {
        self.vert_bufs.push(buf.slice(..));
        self
    }

    #[inline]
    pub fn index_buf(mut self, buf: &'a Buffer, range: Range<u32>) -> Self {
        self.index_buf = Some((buf.slice(..), wgpu::IndexFormat::Uint32, range));
        self
    }

    #[inline]
    pub fn then(self, op: impl EncoderOp) -> CommandBuilder {
        self.build().then(op)
    }

    #[inline]
    pub fn build(self) -> CommandBuilder {
        let mut encoder = self
            .dev
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &self.color_attachs,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.cp.pipeline);

        for (i, g) in self.cp.groups.iter().enumerate() {
            pass.set_bind_group(i as _, g, &[]);
        }

        for (i, s) in self.vert_bufs.into_iter().enumerate() {
            pass.set_vertex_buffer(i as _, s)
        }

        if let Some((b, f, indices)) = self.index_buf {
            pass.set_index_buffer(b, f);
            pass.draw_indexed(indices, 0, self.cp.insts_range.clone());
        } else {
            pass.draw(self.cp.vert_range.clone(), self.cp.insts_range.clone());
        }

        CommandBuilder { encoder }
    }
}

pub trait RenderAttachOp {
    fn attach_op<'a>(&'a self, enc_builder: &mut RenderCommandBuilder<'a>);
}

pub struct RenderAttachment {
    view: wgpu::TextureView,
    ops: wgpu::Operations<wgpu::Color>,
}

impl RenderAttachment {
    #[inline]
    pub fn new(view: wgpu::TextureView) -> Self {
        Self {
            view,
            ops: wgpu::Operations::default(),
        }
    }

    #[inline]
    pub fn load_clear(mut self, [r, g, b, a]: [f64; 4]) -> Self {
        self.ops.load = wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a });
        self
    }

    #[inline]
    pub fn store(mut self) -> Self {
        self.ops.store = wgpu::StoreOp::Store;
        self
    }
}

impl RenderAttachOp for RenderAttachment {
    #[inline]
    fn attach_op<'a>(&'a self, enc_builder: &mut RenderCommandBuilder<'a>) {
        enc_builder.color_attachs.push(Some(self.into()));
    }
}

impl<'a> From<&'a RenderAttachment> for wgpu::RenderPassColorAttachment<'a> {
    #[inline]
    fn from(v: &'a RenderAttachment) -> Self {
        wgpu::RenderPassColorAttachment {
            view: &v.view,
            resolve_target: None,
            ops: v.ops,
        }
    }
}

pub struct CommandBuilder {
    encoder: wgpu::CommandEncoder,
}

impl CommandBuilder {
    #[inline]
    pub fn then(mut self, op: impl EncoderOp) -> Self {
        op.encoder_op(&mut self.encoder);
        self
    }

    #[inline]
    pub fn build(self) -> wgpu::CommandBuffer {
        self.encoder.finish()
    }
}

pub trait EncoderOp {
    fn encoder_op(self, enc: &mut wgpu::CommandEncoder);
}

pub(crate) enum CopyOp<'a> {
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

impl<'a> EncoderOp for CopyOp<'a> {
    fn encoder_op(self, enc: &mut wgpu::CommandEncoder) {
        match self {
            CopyOp::TextBuf(texture, origin, aspect, buffer, ext) => enc.copy_texture_to_buffer(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin,
                    aspect,
                },
                wgpu::ImageCopyBuffer {
                    buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(ext.width * 4),
                        rows_per_image: Some(ext.height),
                    },
                },
                ext,
            ),
            CopyOp::BufText(buffer, texture, origin, aspect, ext) => enc.copy_buffer_to_texture(
                wgpu::ImageCopyBuffer {
                    buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(ext.width * 4),
                        rows_per_image: Some(ext.height * 4),
                    },
                },
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin,
                    aspect,
                },
                ext,
            ),
            CopyOp::BufBuf(src, src_off, dst, dst_off, size) => {
                enc.copy_buffer_to_buffer(src, src_off, dst, dst_off, size)
            }
        }
    }
}
