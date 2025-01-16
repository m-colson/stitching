use crate::shader::CompiledShader;

use super::{Checkpoint, CheckpointItem, TypeOp};

pub type ComputeCheckpoint = Checkpoint<wgpu::ComputePipeline>;

impl ComputeCheckpoint {
    #[inline]
    pub fn to_item(&self) -> ComputeItem<'_> {
        ComputeItem {
            cp: self,
            work_groups: None,
        }
    }
}

pub struct ComputeCheckpointItem<'a> {
    shader: CompiledShader<'a>,
}

impl<'a> CheckpointItem for ComputeCheckpointItem<'a> {
    type ShaderType = CompiledShader<'a>;
    type Pipeline = wgpu::ComputePipeline;
    fn from_shader(shader: Self::ShaderType) -> Self {
        Self { shader }
    }

    fn build_pipeline(self, dev: &wgpu::Device, layout: wgpu::PipelineLayout) -> Self::Pipeline {
        dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            module: &self
                .shader
                .module
                .expect("no shader provided to compute builder"),
            entry_point: self.shader.entry,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }
}

#[derive(Default)]
pub struct ComputePass<'a> {
    items: Vec<ComputeItem<'a>>,
}

impl ComputePass<'_> {
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

impl TypeOp<wgpu::CommandEncoder> for ComputePass<'_> {
    #[inline]
    fn type_op(self, enc: &mut wgpu::CommandEncoder) {
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        for p in self.items {
            p.add_to_pass(&mut pass);
        }
    }
}

pub struct ComputeItem<'a> {
    cp: &'a ComputeCheckpoint,
    work_groups: Option<[u32; 3]>,
}

impl ComputeItem<'_> {
    #[inline]
    pub fn work_groups(mut self, x: u32, y: u32, z: u32) -> Self {
        self.work_groups = Some([x, y, z]);
        self
    }

    #[inline]
    pub(crate) fn add_to_pass(self, pass: &mut wgpu::ComputePass) {
        let Some([x, y, z]) = self.work_groups else {
            panic!("no workgroups where specified for compute pass");
        };

        pass.set_pipeline(&self.cp.pipeline);
        for (i, g) in self.cp.groups.iter().enumerate() {
            pass.set_bind_group(i as _, g, &[]);
        }
        pass.dispatch_workgroups(x, y, z);
    }
}
