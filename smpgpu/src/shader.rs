use std::borrow::Cow;

use crate::OntoDevice;

#[macro_export]
macro_rules! include_shader {
    ($vp: literal $(=> $ve: literal)? $(& $fe: literal)?) => {
        ::smpgpu::Shader::new()
            .wgsl_module(include_str!($vp))
            $(.entry($ve))?
            $(.frag_entry($fe))?
    };
}

#[derive(Clone, Default, Debug)]
pub struct Shader<'a, 'b> {
    desc: Option<wgpu::ShaderModuleDescriptor<'a>>,
    entry: Option<&'b str>,
}

impl<'a, 'b> Shader<'a, 'b> {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    #[inline]
    pub fn module(mut self, desc: impl Into<Option<wgpu::ShaderModuleDescriptor<'a>>>) -> Self {
        self.desc = desc.into();
        self
    }

    #[must_use]
    #[inline]
    pub fn wgsl_module(self, content: impl Into<Cow<'a, str>>) -> Self {
        self.module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(content.into()),
        })
    }

    #[must_use]
    #[inline]
    pub fn entry(mut self, entry: impl Into<Option<&'b str>>) -> Self {
        self.entry = entry.into();
        self
    }

    #[must_use]
    #[inline]
    pub fn frag_entry(self, entry: impl Into<Option<&'b str>>) -> RenderShader<'a, 'b> {
        RenderShader {
            vert: self,
            frag: Shader::new().entry(entry),
        }
    }
}

impl<'a, 'b> From<Shader<'a, 'b>> for RenderShader<'a, 'b> {
    fn from(vert: Shader<'a, 'b>) -> Self {
        RenderShader {
            vert,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct CompiledShader<'b> {
    pub module: Option<wgpu::ShaderModule>,
    pub entry: Option<&'b str>,
}

impl<'b> OntoDevice<CompiledShader<'b>> for Shader<'_, 'b> {
    fn onto_device(self, dev: &wgpu::Device) -> CompiledShader<'b> {
        CompiledShader {
            module: self.desc.map(|desc| dev.create_shader_module(desc)),
            entry: self.entry,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct RenderShader<'a, 'b> {
    vert: Shader<'a, 'b>,
    frag: Shader<'a, 'b>,
}

impl RenderShader<'_, '_> {}

impl<'b> OntoDevice<CompiledRenderShader<'b>> for Shader<'_, 'b> {
    fn onto_device(self, dev: &wgpu::Device) -> CompiledRenderShader<'b> {
        RenderShader::from(self).onto_device(dev)
    }
}

impl<'b> OntoDevice<CompiledRenderShader<'b>> for RenderShader<'_, 'b> {
    fn onto_device(self, dev: &wgpu::Device) -> CompiledRenderShader<'b> {
        CompiledRenderShader {
            vert: self.vert.onto_device(dev),
            frag: self.frag.onto_device(dev),
        }
    }
}

#[derive(Debug, Default)]
pub struct CompiledRenderShader<'b> {
    vert: CompiledShader<'b>,
    frag: CompiledShader<'b>,
}

impl<'b> CompiledRenderShader<'b> {
    #[inline]
    pub(crate) const fn split(
        &self,
    ) -> Option<(
        &wgpu::ShaderModule,
        Option<&'b str>,
        &wgpu::ShaderModule,
        Option<&'b str>,
    )> {
        match (self.vert.module.as_ref(), self.frag.module.as_ref()) {
            (Some(vm), Some(fm)) => Some((vm, self.vert.entry, fm, self.frag.entry)),
            (Some(m), _) | (_, Some(m)) => Some((m, self.vert.entry, m, self.frag.entry)),
            _ => None,
        }
    }
}
