use std::marker::PhantomData;

pub trait IntoBindGroup {
    fn into_wgpu_bind_group(self, dev: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup);
}

/// A single group of bindings.
#[derive(Default)]
pub struct Bindings<'a> {
    types: Vec<(wgpu::ShaderStages, wgpu::BindingType)>,
    resources: Vec<BindResource<'a>>,
}

impl<'a> Bindings<'a> {
    /// Create a new group with no bindings.
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add new binding to this group.
    #[must_use]
    #[inline]
    pub fn bind<T: AsBinding + 'a>(mut self, vis_bind: impl Into<VisBindable<'a, T>>) -> Self {
        let vis_bind = vis_bind.into();
        let vis = vis_bind.1;
        let (ty, recs) = vis_bind.0.as_binding();
        self.types.push((vis, ty));
        self.resources.push(recs);
        self
    }
}

impl<'a, B: AsBinding + 'a> std::ops::BitAnd<VisBindable<'a, B>> for Bindings<'a> {
    type Output = Self;

    fn bitand(self, rhs: VisBindable<'a, B>) -> Self {
        self.bind(rhs)
    }
}

impl IntoBindGroup for Bindings<'_> {
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

impl<'a, B1: AsBinding + 'a, B2: AsBinding + 'a> std::ops::BitAnd<VisBindable<'a, B2>>
    for VisBindable<'a, B1>
{
    type Output = Bindings<'a>;

    fn bitand(self, rhs: VisBindable<'a, B2>) -> Self::Output {
        Bindings::new().bind(self).bind(rhs)
    }
}

impl<'a, B: AsBinding + 'a> IntoBindGroup for VisBindable<'a, B> {
    fn into_wgpu_bind_group(self, dev: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        Bindings::new().bind(self).into_wgpu_bind_group(dev)
    }
}

pub trait AsBinding {
    fn as_binding(&self) -> (wgpu::BindingType, BindResource<'_>);
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

pub struct VisBindable<'a, T>(&'a T, wgpu::ShaderStages, PhantomData<()>);

impl<'a, T: AsBinding> VisBindable<'a, T> {
    #[inline]
    pub const fn new(inner: &'a T, stages: wgpu::ShaderStages) -> Self {
        Self(inner, stages, PhantomData)
    }

    pub fn visibilities(&self) -> wgpu::ShaderStages {
        self.1
    }

    pub fn in_compute(mut self) -> Self {
        self.1 |= wgpu::ShaderStages::COMPUTE;
        self
    }

    pub fn in_vertex(mut self) -> Self {
        self.1 |= wgpu::ShaderStages::VERTEX;
        self
    }

    pub fn in_frag(mut self) -> Self {
        self.1 |= wgpu::ShaderStages::FRAGMENT;
        self
    }
}

/// The `AutoVisBindable` trait allow the in_* functions to be available on all
/// types that implement [`AsBinding`], without having to first convert it to
/// [`VisBindable`]
pub trait AutoVisBindable: AsBinding + Sized {
    /// Creates a [`VisBindable`] referencing `self` with compute visibility.
    fn in_compute(&self) -> VisBindable<'_, Self> {
        VisBindable::new(self, wgpu::ShaderStages::COMPUTE)
    }

    /// Creates a [`VisBindable`] referencing `self` with vertex visibility.
    fn in_vertex(&self) -> VisBindable<'_, Self> {
        VisBindable::new(self, wgpu::ShaderStages::VERTEX)
    }

    /// Creates a [`VisBindable`] referencing `self` with fragment visibility.
    fn in_frag(&self) -> VisBindable<'_, Self> {
        VisBindable::new(self, wgpu::ShaderStages::FRAGMENT)
    }
}

impl<T: AsBinding> AutoVisBindable for T {}

impl<'a, T: AsBinding> From<&'a T> for VisBindable<'a, T> {
    fn from(v: &'a T) -> Self {
        Self::new(v, wgpu::ShaderStages::empty())
    }
}
