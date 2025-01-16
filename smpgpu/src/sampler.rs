use wgpu::SamplerDescriptor;

use crate::bind::{AsBinding, BindResource};

pub struct Sampler {
    inner: wgpu::Sampler,
    ty: wgpu::SamplerBindingType,
}

impl Sampler {
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> SamplerBuilder<'_> {
        SamplerBuilder::new(dev.as_ref())
    }
}

impl AsBinding for Sampler {
    fn as_binding(&self) -> (wgpu::BindingType, BindResource<'_>) {
        (
            wgpu::BindingType::Sampler(self.ty),
            BindResource::Sampler(&self.inner),
        )
    }
}

pub struct SamplerBuilder<'a> {
    dev: &'a wgpu::Device,
    label: Option<&'a str>,
}

impl<'a> SamplerBuilder<'a> {
    #[must_use]
    #[inline]
    pub const fn new(dev: &'a wgpu::Device) -> Self {
        Self { dev, label: None }
    }

    #[must_use]
    #[inline]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    #[must_use]
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
