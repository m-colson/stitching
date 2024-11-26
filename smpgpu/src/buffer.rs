use encase::{CalculateSizeFor, ShaderSize};
use wgpu::util::DeviceExt;

use crate::{
    bind::{BindResource, VisBindable},
    cmd::{CopyOp, EncoderOp},
    Bindable,
};

pub struct Buffer {
    inner: wgpu::Buffer,
}

impl Buffer {
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> BufferBuilder<'_> {
        BufferBuilder::new(dev.as_ref())
    }

    #[inline]
    pub fn copy_to_buf_op<'a>(&'a self, buf: &'a Buffer) -> impl EncoderOp + 'a {
        CopyOp::BufBuf(self, 0, buf, 0, self.size())
    }
}

impl<'a> Bindable<'a> for &'a Buffer {
    type VisBind = Self;

    fn into_binding(self) -> (wgpu::BindingType, BindResource<'a>) {
        let ty = if self.usage().contains(wgpu::BufferUsages::STORAGE) {
            wgpu::BufferBindingType::Storage {
                read_only: !self.usage().contains(wgpu::BufferUsages::COPY_SRC),
            }
        } else if self.usage().contains(wgpu::BufferUsages::UNIFORM) {
            wgpu::BufferBindingType::Uniform
        } else {
            panic!("attempted to make a binding for a buffer that is neither STORAGE or UNIFORM");
        };

        (
            wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            BindResource::Buffer(self),
        )
    }

    #[inline]
    fn in_compute(self) -> VisBindable<'a, Self> {
        VisBindable::new(self, wgpu::ShaderStages::COMPUTE)
    }

    #[inline]
    fn in_vertex(self) -> VisBindable<'a, Self> {
        VisBindable::new(self, wgpu::ShaderStages::VERTEX)
    }

    #[inline]
    fn in_frag(self) -> VisBindable<'a, Self> {
        VisBindable::new(self, wgpu::ShaderStages::FRAGMENT)
    }
}

pub struct BufferBuilder<'a> {
    dev: &'a wgpu::Device,
    label: Option<&'a str>,
    size: u64,
    usage: wgpu::BufferUsages,
}

impl<'a> BufferBuilder<'a> {
    #[inline]
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self {
            dev,
            label: None,
            size: 0,
            usage: wgpu::BufferUsages::empty(),
        }
    }

    #[inline]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    #[inline]
    pub fn size(mut self, size: usize) -> Self {
        self.size = size as _;
        self
    }

    #[inline]
    pub fn size_for<T: ShaderSize>(mut self) -> Self {
        self.size = u64::from(T::SHADER_SIZE);
        self
    }

    #[inline]
    pub fn size_for_many<T>(mut self, elms: u64) -> Self
    where
        Vec<T>: CalculateSizeFor,
    {
        self.size = u64::from(Vec::<T>::calculate_size_for(elms));
        self
    }

    #[inline]
    fn with_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage |= usage;
        if (self.usage.contains(wgpu::BufferUsages::MAP_READ)
            || self.usage.contains(wgpu::BufferUsages::MAP_WRITE))
            && (self.usage.contains(wgpu::BufferUsages::UNIFORM)
                || self.usage.contains(wgpu::BufferUsages::STORAGE))
        {
            self.usage
                .remove(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE);
        }
        self
    }

    #[inline]
    pub fn readable(self) -> Self {
        self.with_usage(wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE)
    }

    #[inline]
    pub fn writable(self) -> Self {
        self.with_usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ)
    }

    #[inline]
    pub fn storage(self) -> Self {
        self.with_usage(wgpu::BufferUsages::STORAGE)
    }

    #[inline]
    pub fn uniform(self) -> Self {
        self.with_usage(wgpu::BufferUsages::UNIFORM)
    }

    #[inline]
    pub fn vertex(self) -> Self {
        self.with_usage(wgpu::BufferUsages::VERTEX)
    }

    #[inline]
    pub fn build(self) -> Buffer {
        let inner = self.dev.create_buffer(&wgpu::BufferDescriptor {
            label: self.label,
            size: self.size,
            usage: self.usage,
            mapped_at_creation: false,
        });

        Buffer { inner }
    }

    /// SAFETY: T must be safe to transmute to bytes (likely true for any type you would want to put in a buffer).
    #[inline]
    pub fn build_with_data<T>(self, data: &[T]) -> Buffer {
        let inner = self
            .dev
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.label,
                contents: unsafe {
                    std::slice::from_raw_parts(
                        data.as_ptr() as *const u8,
                        std::mem::size_of_val(data),
                    )
                },
                usage: self.usage,
            });
        Buffer { inner }
    }
}

impl std::ops::Deref for Buffer {
    type Target = wgpu::Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
