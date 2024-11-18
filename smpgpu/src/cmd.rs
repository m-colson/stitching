#![allow(dead_code)]

use wgpu::{BindGroupDescriptor, BindGroupLayoutDescriptor, ComputePassDescriptor};
pub struct CommandBuilder<'a> {
    dev: &'a wgpu::Device,
}

impl<'a> CommandBuilder<'a> {
    pub fn new(dev: &'a impl AsRef<wgpu::Device>) -> Self {
        Self { dev: dev.as_ref() }
    }

    pub fn with_shader(self, desc: wgpu::ShaderModuleDescriptor) -> CommandShaderBuilder<'a, '_> {
        let shader = self.dev.create_shader_module(desc);
        CommandShaderBuilder {
            dev: self.dev,
            shader,
            bind_types: Vec::new(),
            bind_resources: Vec::new(),
        }
    }
}

pub struct CommandShaderBuilder<'a, 'b> {
    dev: &'a wgpu::Device,
    shader: wgpu::ShaderModule,
    bind_types: Vec<wgpu::BindingType>,
    bind_resources: Vec<BindResource<'b>>,
}

pub trait Bindable {
    fn as_binding(&self) -> (wgpu::BindingType, BindResource<'_>);
}
pub enum BindResource<'a> {
    Buffer(&'a wgpu::Buffer),
    TextureView(wgpu::TextureView),
}

impl<'a> BindResource<'a> {
    pub fn as_resource(&'a self) -> wgpu::BindingResource<'a> {
        match self {
            BindResource::Buffer(v) => v.as_entire_binding(),
            BindResource::TextureView(v) => wgpu::BindingResource::TextureView(v),
        }
    }
}

impl<'a, 'b> CommandShaderBuilder<'a, 'b> {
    pub fn bind(mut self, txt: &'b impl Bindable) -> Self {
        let (ty, recs) = txt.as_binding();
        self.bind_types.push(ty);
        self.bind_resources.push(recs);
        self
    }

    pub fn entry_point(self, symbol: &str) -> CommandCheckpoint {
        let bind_layout = self
            .dev
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: self
                    .bind_types
                    .into_iter()
                    .enumerate()
                    .map(|(i, ty)| wgpu::BindGroupLayoutEntry {
                        binding: i as _,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty,
                        count: None,
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
            });

        let bind_group = self.dev.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_layout,
            entries: self
                .bind_resources
                .iter()
                .enumerate()
                .map(|(i, br)| wgpu::BindGroupEntry {
                    binding: i as _,
                    resource: br.as_resource(),
                })
                .collect::<Vec<_>>()
                .as_slice(),
        });

        let pipeline_layout = self
            .dev
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_layout],
                push_constant_ranges: &[],
            });
        let pipeline = self
            .dev
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &self.shader,
                entry_point: Some(symbol),
                compilation_options: Default::default(),
                cache: None,
            });

        CommandCheckpoint {
            bind_group,
            pipeline,
            work_groups: [0; 3],
        }
    }
}

pub struct CommandCheckpoint {
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    work_groups: [u32; 3],
}

impl CommandCheckpoint {
    pub fn work_groups(mut self, x: usize, y: usize, z: usize) -> Self {
        self.work_groups = [x as _, y as _, z as _];
        self
    }

    pub fn ref_builder<'a>(
        &'a self,
        dev: &'a impl AsRef<wgpu::Device>,
    ) -> CommandEncoderBuilder<'a> {
        let dev = dev.as_ref();
        let mut encoder: wgpu::CommandEncoder =
            dev.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.dispatch_workgroups(
            self.work_groups[0],
            self.work_groups[1],
            self.work_groups[2],
        );

        CommandEncoderBuilder {
            dev,
            bind_group: &self.bind_group,
            pipeline: &self.pipeline,
            encoder,
        }
    }
}

pub struct CommandEncoderBuilder<'a> {
    dev: &'a wgpu::Device,
    bind_group: &'a wgpu::BindGroup,
    pipeline: &'a wgpu::ComputePipeline,
    encoder: wgpu::CommandEncoder,
}

impl<'a> CommandEncoderBuilder<'a> {
    pub fn then(mut self, op: impl EncoderOp) -> Self {
        op.encoder_op(&mut self.encoder);
        self
    }

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
