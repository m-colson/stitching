use encase::{CalculateSizeFor, ShaderSize};
use wgpu::util::DeviceExt;

use crate::{
    bind::{AsBinding, BindResource},
    cmd::CopyOp,
};

/// Wrapper type over a [`wgpu::Buffer`]
pub struct Buffer {
    inner: wgpu::Buffer,
}

impl Buffer {
    /// Create a new [`BufferBuilder`] for `dev`.
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> BufferBuilder<'_> {
        BufferBuilder::new(dev.as_ref())
    }

    /// Create a new operation that will copy the data of `self` to `buf`.
    #[inline]
    pub fn copy_to_buf_op<'a>(&'a self, buf: &'a Self) -> CopyOp<'a> {
        CopyOp::BufBuf(self, 0, buf, 0, self.size())
    }
}

impl AsBinding for Buffer {
    fn as_binding(&self) -> (wgpu::BindingType, BindResource<'_>) {
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
}

/// Builder type for creating a [`Buffer`].
pub struct BufferBuilder<'a> {
    dev: &'a wgpu::Device,
    label: Option<&'a str>,
    size: u64,
    usage: wgpu::BufferUsages,
    init_data: Option<&'a [u8]>,
}

impl<'a> BufferBuilder<'a> {
    /// Create a new [`BufferBuilder`] for `dev`.
    #[must_use]
    #[inline]
    pub const fn new(dev: &'a wgpu::Device) -> Self {
        Self {
            dev,
            label: None,
            size: 0,
            usage: wgpu::BufferUsages::empty(),
            init_data: None,
        }
    }

    /// Use `label` when creating the buffer.
    #[must_use]
    #[inline]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Use the specified `size` when creating the buffer.
    #[must_use]
    #[inline]
    pub const fn size(mut self, size: usize) -> Self {
        self.size = size as _;
        self
    }

    /// Use the size required for one `T` when creating the buffer.
    #[must_use]
    #[inline]
    pub fn size_for<T: ShaderSize>(mut self) -> Self {
        self.size = u64::from(T::SHADER_SIZE);
        self
    }

    /// Use the size required for `elms` `T` when creating the buffer.
    #[must_use]
    #[inline]
    pub fn size_for_many<T>(mut self, elms: u64) -> Self
    where
        Vec<T>: CalculateSizeFor,
    {
        self.size = u64::from(Vec::<T>::calculate_size_for(elms));
        self
    }

    /// Use an additional [`wgpu::BufferUsages`] flag when creating the buffer.
    #[inline]
    fn with_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage |= usage;
        if (self.usage.contains(wgpu::BufferUsages::MAP_READ)
            || self.usage.contains(wgpu::BufferUsages::MAP_WRITE))
            && (self.usage.contains(wgpu::BufferUsages::UNIFORM)
                || self.usage.contains(wgpu::BufferUsages::STORAGE)
                || self.usage.contains(wgpu::BufferUsages::VERTEX))
        {
            self.usage
                .remove(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE);
        }
        self
    }

    /// Mark the created buffer as being readable on cpu side (can copy from it).
    #[must_use]
    #[inline]
    pub fn readable(self) -> Self {
        self.with_usage(wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE)
    }

    /// Mark the created buffer as being writable on cpu side (can copy to it).
    #[must_use]
    #[inline]
    pub fn writable(self) -> Self {
        self.with_usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ)
    }

    /// Mark the created buffer for storage use.
    #[must_use]
    #[inline]
    pub fn storage(self) -> Self {
        self.with_usage(wgpu::BufferUsages::STORAGE)
    }

    /// Mark the created buffer for uniform value use.
    #[must_use]
    #[inline]
    pub fn uniform(self) -> Self {
        self.with_usage(wgpu::BufferUsages::UNIFORM)
    }

    /// Mark the created buffer for vertex use.
    #[must_use]
    #[inline]
    pub fn vertex(self) -> Self {
        self.with_usage(wgpu::BufferUsages::VERTEX)
    }

    /// Mark the created buffer for index use.
    #[must_use]
    #[inline]
    pub fn index(self) -> Self {
        self.with_usage(wgpu::BufferUsages::INDEX)
    }

    /// Initalize with raw `data` when creating the buffer.
    #[must_use]
    #[inline]
    pub fn init_bytes(mut self, data: &'a [u8]) -> Self {
        self.init_data = Some(data);
        self
    }

    /// Initialize with the contents of `data` when creating the buffer.
    /// SAFETY: T must be safe to transmute to bytes (likely true for any type you would want to put in a buffer).
    #[must_use]
    #[inline]
    pub fn init_data<T>(self, data: &'a [T]) -> Self {
        self.init_bytes(unsafe {
            std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), std::mem::size_of_val(data))
        })
    }

    /// Complete the builder and create the final [`Buffer`].
    #[must_use]
    #[inline]
    pub fn build(self) -> Buffer {
        let inner = match self.init_data {
            Some(contents) => self
                .dev
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label,
                    contents,
                    usage: self.usage,
                }),
            None => self.dev.create_buffer(&wgpu::BufferDescriptor {
                label: self.label,
                size: self.size,
                usage: self.usage,
                mapped_at_creation: false,
            }),
        };

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
