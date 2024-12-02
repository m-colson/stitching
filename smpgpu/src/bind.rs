use std::marker::PhantomData;

pub trait IntoBindGroup {
    fn into_wgpu_bind_group(self, dev: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup);
}

#[derive(Default)]
pub struct Bindings<'a> {
    types: Vec<(wgpu::ShaderStages, wgpu::BindingType)>,
    resources: Vec<BindResource<'a>>,
}

impl<'a> Bindings<'a> {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    #[inline]
    pub fn bind(mut self, txt: impl Bindable<'a>) -> Self {
        let vis = txt.as_visibility();
        let (ty, recs) = txt.into_binding();
        self.types.push((vis, ty));
        self.resources.push(recs);
        self
    }
}

impl<'a> IntoBindGroup for Bindings<'a> {
    fn into_wgpu_bind_group(self, dev: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let bind_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: self
                .types
                .into_iter()
                .enumerate()
                .map(|(i, (visibility, ty))| wgpu::BindGroupLayoutEntry {
                    binding: i as _,
                    visibility,
                    ty,
                    count: None,
                })
                .collect::<Vec<_>>()
                .as_slice(),
        });

        let bind_group = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_layout,
            entries: self
                .resources
                .iter()
                .enumerate()
                .map(|(i, br)| wgpu::BindGroupEntry {
                    binding: i as _,
                    resource: br.as_resource(),
                })
                .collect::<Vec<_>>()
                .as_slice(),
        });

        (bind_layout, bind_group)
    }
}

pub trait Bindable<'a>: Sized {
    type VisBind: Bindable<'a> + Sized;

    fn into_binding(self) -> (wgpu::BindingType, BindResource<'a>);

    fn as_visibility(&self) -> wgpu::ShaderStages {
        wgpu::ShaderStages::all()
    }

    fn in_compute(self) -> VisBindable<'a, Self::VisBind>;

    fn in_vertex(self) -> VisBindable<'a, Self::VisBind>;

    fn in_frag(self) -> VisBindable<'a, Self::VisBind>;
}

pub enum BindResource<'a> {
    Buffer(&'a wgpu::Buffer),
    TextureView(wgpu::TextureView),
    Sampler(&'a wgpu::Sampler),
}

impl<'a> BindResource<'a> {
    pub fn as_resource(&'a self) -> wgpu::BindingResource<'a> {
        match self {
            BindResource::Buffer(v) => v.as_entire_binding(),
            BindResource::TextureView(v) => wgpu::BindingResource::TextureView(v),
            BindResource::Sampler(s) => wgpu::BindingResource::Sampler(s),
        }
    }
}

pub struct VisBindable<'a, T: Bindable<'a>>(T, wgpu::ShaderStages, PhantomData<&'a ()>);

impl<'a, T: Bindable<'a>> VisBindable<'a, T> {
    #[inline]
    pub const fn new(inner: T, stages: wgpu::ShaderStages) -> Self {
        Self(inner, stages, PhantomData)
    }
}

impl<'a, T: Bindable<'a>> Bindable<'a> for VisBindable<'a, T> {
    type VisBind = T;

    fn into_binding(self) -> (wgpu::BindingType, BindResource<'a>) {
        self.0.into_binding()
    }

    fn as_visibility(&self) -> wgpu::ShaderStages {
        self.1
    }

    fn in_compute(mut self) -> Self {
        self.1 |= wgpu::ShaderStages::COMPUTE;
        self
    }

    fn in_vertex(mut self) -> Self {
        self.1 |= wgpu::ShaderStages::VERTEX;
        self
    }

    fn in_frag(mut self) -> Self {
        self.1 |= wgpu::ShaderStages::FRAGMENT;
        self
    }
}
