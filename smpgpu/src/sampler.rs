use wgpu::SamplerDescriptor;

use crate::{
    bind::{BindResource, VisBindable},
    Bindable,
};

pub struct Sampler {
    inner: wgpu::Sampler,
    ty: wgpu::SamplerBindingType,
}

impl Sampler {
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> SamplerBuilder<'_> {
        SamplerBuilder::new(dev.as_ref())
    }
}

impl<'a> Bindable<'a> for &'a Sampler {
    type VisBind = Self;

    fn into_binding(self) -> (wgpu::BindingType, BindResource<'a>) {
        (
            wgpu::BindingType::Sampler(self.ty),
            BindResource::Sampler(&self.inner),
        )
    }

    #[inline]
    fn in_compute(self) -> VisBindable<'a, Self::VisBind> {
        VisBindable::new(self, wgpu::ShaderStages::COMPUTE)
    }

    #[inline]
    fn in_vertex(self) -> VisBindable<'a, Self::VisBind> {
        VisBindable::new(self, wgpu::ShaderStages::VERTEX)
    }

    #[inline]
    fn in_frag(self) -> VisBindable<'a, Self::VisBind> {
        VisBindable::new(self, wgpu::ShaderStages::FRAGMENT)
    }
}

pub struct SamplerBuilder<'a> {
    dev: &'a wgpu::Device,
    label: Option<&'a str>,
}

impl<'a> SamplerBuilder<'a> {
    #[inline]
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self { dev, label: None }
    }

    #[inline]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    #[inline]
    pub fn build(self) -> Sampler {
        let inner = self.dev.create_sampler(&SamplerDescriptor {
            label: self.label,
            address_mode_u: wgpu::AddressMode::ClampToBorder,
            address_mode_v: wgpu::AddressMode::ClampToBorder,
            address_mode_w: wgpu::AddressMode::ClampToBorder,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: Some(wgpu::SamplerBorderColor::TransparentBlack),
        });

        Sampler {
            inner,
            ty: wgpu::SamplerBindingType::Filtering,
        }
    }
}