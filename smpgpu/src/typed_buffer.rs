use std::marker::PhantomData;

use encase::ShaderSize;

use crate::{
    bind::{AsBinding, BindResource},
    Buffer, BufferBuilder,
};

pub(crate) trait TypedBuffer {
    fn as_buf_ref(&self) -> &Buffer;
}

impl<T: TypedBuffer> AsBinding for T {
    fn as_binding(&self) -> (wgpu::BindingType, BindResource<'_>) {
        self.as_buf_ref().as_binding()
    }
}

/// Typed wrapper over a [`Buffer`] that is supposed to contain a uniform value.
pub struct Uniform<T>(pub(crate) Buffer, PhantomData<T>);

impl<T: ShaderSize> Uniform<T> {
    /// Creates a new builder for `Uniform`.
    #[must_use]
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> UniformBuilder<'_, T> {
        UniformBuilder::new(dev.as_ref())
    }
}

impl<T> TypedBuffer for Uniform<T> {
    fn as_buf_ref(&self) -> &Buffer {
        &self.0
    }
}

pub struct UniformBuilder<'a, T: ShaderSize>(BufferBuilder<'a>, PhantomData<T>);

impl<'a, T: ShaderSize> UniformBuilder<'a, T> {
    #[must_use]
    #[inline]
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self(
            BufferBuilder::new(dev).size_for::<T>().uniform(),
            PhantomData,
        )
    }

    #[must_use]
    #[inline]
    pub fn label(mut self, label: &'a str) -> Self {
        self.0 = self.0.label(label);
        self
    }

    #[must_use]
    #[inline]
    pub fn writable(mut self) -> Self {
        self.0 = self.0.writable();
        self
    }

    #[must_use]
    #[inline]
    pub fn readable(mut self) -> Self {
        self.0 = self.0.readable();
        self
    }

    #[must_use]
    #[inline]
    pub fn init_data(mut self, v: &'a T) -> Self {
        self.0 = self.0.init_data(std::slice::from_ref(v));
        self
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> Uniform<T> {
        Uniform(self.0.build(), self.1)
    }
}

pub struct StorageBuffer<T>(pub(crate) Buffer, PhantomData<T>);

impl<T: ShaderSize> StorageBuffer<T> {
    #[must_use]
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> StorageBufferBuilder<'_, T> {
        StorageBufferBuilder::new(dev.as_ref())
    }
}

impl<T> TypedBuffer for StorageBuffer<T> {
    fn as_buf_ref(&self) -> &Buffer {
        &self.0
    }
}

pub struct StorageBufferBuilder<'a, T: ShaderSize>(BufferBuilder<'a>, PhantomData<T>);

impl<'a, T: ShaderSize> StorageBufferBuilder<'a, T> {
    #[must_use]
    #[inline]
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self(BufferBuilder::new(dev).storage(), PhantomData)
    }

    pub fn len(mut self, elms: u64) -> Self {
        self.0 = self.0.size_for_many::<T>(elms);
        self
    }

    #[must_use]
    #[inline]
    pub fn label(mut self, label: &'a str) -> Self {
        self.0 = self.0.label(label);
        self
    }

    #[must_use]
    #[inline]
    pub fn writable(mut self) -> Self {
        self.0 = self.0.writable();
        self
    }

    #[must_use]
    #[inline]
    pub fn readable(mut self) -> Self {
        self.0 = self.0.readable();
        self
    }

    #[must_use]
    #[inline]
    pub fn init_data(mut self, v: &'a [T]) -> Self {
        self.0 = self.0.init_data(v);
        self
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> StorageBuffer<T> {
        StorageBuffer(self.0.build(), self.1)
    }
}

pub struct VertexBuffer<T>(pub(crate) Buffer, pub(crate) u32, PhantomData<T>);

impl<T: ShaderSize> VertexBuffer<T> {
    #[must_use]
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> VertexBufferBuilder<'_, T> {
        VertexBufferBuilder::new(dev.as_ref())
    }
}

impl<T> TypedBuffer for VertexBuffer<T> {
    fn as_buf_ref(&self) -> &Buffer {
        &self.0
    }
}

impl<T> AsRef<Buffer> for VertexBuffer<T> {
    fn as_ref(&self) -> &Buffer {
        &self.0
    }
}

pub struct VertexBufferBuilder<'a, T: ShaderSize>(BufferBuilder<'a>, u32, PhantomData<T>);

impl<'a, T: ShaderSize> VertexBufferBuilder<'a, T> {
    #[must_use]
    #[inline]
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self(BufferBuilder::new(dev).vertex(), 0, PhantomData)
    }

    pub fn len(mut self, elms: u32) -> Self {
        self.0 = self.0.size_for_many::<T>(elms as _);
        self.1 = elms;
        self
    }

    #[must_use]
    #[inline]
    pub fn label(mut self, label: &'a str) -> Self {
        self.0 = self.0.label(label);
        self
    }

    #[must_use]
    #[inline]
    pub fn writable(mut self) -> Self {
        self.0 = self.0.writable();
        self
    }

    #[must_use]
    #[inline]
    pub fn readable(mut self) -> Self {
        self.0 = self.0.readable();
        self
    }

    #[must_use]
    #[inline]
    pub fn init_data(mut self, v: &'a [T]) -> Self {
        self.0 = self.0.init_data(v);
        self.1 = v.len() as _;
        self
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> VertexBuffer<T> {
        VertexBuffer(self.0.build(), self.1, self.2)
    }
}

pub struct IndexBuffer<T>(pub(crate) Buffer, PhantomData<T>);

impl<T: IndexBufferFormat> IndexBuffer<T> {
    #[must_use]
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> IndexBufferBuilder<'_, T> {
        IndexBufferBuilder::new(dev.as_ref())
    }
}

impl<T> TypedBuffer for IndexBuffer<T> {
    fn as_buf_ref(&self) -> &Buffer {
        &self.0
    }
}

impl<T> AsRef<Buffer> for IndexBuffer<T> {
    fn as_ref(&self) -> &Buffer {
        &self.0
    }
}

pub struct IndexBufferBuilder<'a, T: IndexBufferFormat>(BufferBuilder<'a>, PhantomData<T>);

impl<'a, T: IndexBufferFormat> IndexBufferBuilder<'a, T> {
    #[must_use]
    #[inline]
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self(BufferBuilder::new(dev).index(), PhantomData)
    }

    pub fn len(mut self, elms: u32) -> Self {
        self.0 = self.0.size(elms as usize * std::mem::size_of::<T>());
        self
    }

    #[must_use]
    #[inline]
    pub fn label(mut self, label: &'a str) -> Self {
        self.0 = self.0.label(label);
        self
    }

    #[must_use]
    #[inline]
    pub fn writable(mut self) -> Self {
        self.0 = self.0.writable();
        self
    }

    #[must_use]
    #[inline]
    pub fn readable(mut self) -> Self {
        self.0 = self.0.readable();
        self
    }

    #[must_use]
    #[inline]
    pub fn init_data(mut self, v: &'a [T]) -> Self {
        self.0 = self.0.init_data(v);
        self
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> IndexBuffer<T> {
        IndexBuffer(self.0.build(), self.1)
    }
}

pub trait IndexBufferFormat: Copy {
    fn index_format() -> wgpu::IndexFormat;
}

impl IndexBufferFormat for u16 {
    fn index_format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint16
    }
}

impl IndexBufferFormat for u32 {
    fn index_format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }
}
