use std::marker::PhantomData;

use encase::ShaderSize;

use crate::{
    Buffer,
    bind::{AsBinding, BindResource},
    buffer::BufferBuilder,
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
#[derive(Clone, Debug)]
pub struct Uniform<T>(pub(crate) Buffer, PhantomData<T>);

impl<T: ShaderSize> Uniform<T> {
    /// Creates a new builder for `Uniform`.
    #[must_use]
    #[inline]
    pub fn builder<'a>(dev: &'a impl AsRef<wgpu::Device>, label: &'a str) -> UniformBuilder<'a, T> {
        Buffer::builder(dev, label).uniform()
    }
}

impl<T> TypedBuffer for Uniform<T> {
    fn as_buf_ref(&self) -> &Buffer {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct UniformKind<T: ShaderSize>(PhantomData<T>);

impl<T: ShaderSize> Default for UniformKind<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub type UniformBuilder<'a, T> = BufferBuilder<'a, UniformKind<T>>;

impl<'a, T: ShaderSize> UniformBuilder<'a, T> {
    /// Initialize with the contents of `data` when creating the buffer.
    /// SAFETY: T must be safe to transmute to bytes (likely true for any type you would want to put in a buffer).
    #[must_use]
    #[inline]
    pub fn init(self, data: &'a T) -> Self {
        self.init_bytes(unsafe {
            std::slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of_val(data))
        })
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> Uniform<T> {
        Uniform(self.build_untyped(), PhantomData)
    }
}

#[derive(Clone, Debug)]
pub struct StorageBuffer<T>(pub(crate) Buffer, PhantomData<T>);

impl<T: ShaderSize> StorageBuffer<T> {
    pub fn as_untyped(&self) -> &Buffer {
        &self.0
    }
}

impl<T> TypedBuffer for StorageBuffer<T> {
    fn as_buf_ref(&self) -> &Buffer {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct StorageKind<T: ShaderSize>(PhantomData<T>);

impl<T: ShaderSize> Default for StorageKind<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub type StorageBufferBuilder<'a, T> = BufferBuilder<'a, StorageKind<T>>;

impl<'a, T: ShaderSize> StorageBufferBuilder<'a, T> {
    #[must_use]
    #[inline]
    pub fn len(self, elms: u64) -> Self {
        self.size_for_many::<T>(elms)
    }

    #[must_use]
    #[inline]
    pub fn init_data(self, v: &'a [T]) -> Self {
        self.init_bytes(unsafe {
            std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), std::mem::size_of_val(v))
        })
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> StorageBuffer<T> {
        StorageBuffer(self.build_untyped(), PhantomData)
    }
}

#[derive(Clone, Debug)]
pub struct VertexBuffer<T>(pub(crate) Buffer, pub(crate) u32, PhantomData<T>);

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

pub struct VertexKind<T: ShaderSize>(PhantomData<T>, u32);

impl<T: ShaderSize> Default for VertexKind<T> {
    fn default() -> Self {
        Self(Default::default(), 0)
    }
}

pub type VertexBufferBuilder<'a, T> = BufferBuilder<'a, VertexKind<T>>;

impl<'a, T: ShaderSize> VertexBufferBuilder<'a, T> {
    #[must_use]
    #[inline]
    pub fn len(self, elms: u32) -> Self {
        let mut b = self.size_for_many::<T>(elms as _);
        b.k.1 = elms;
        b
    }

    #[must_use]
    #[inline]
    pub fn init_data(self, v: &'a [T]) -> Self {
        let mut b = self.init_bytes(unsafe {
            std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), std::mem::size_of_val(v))
        });
        b.k.1 = v.len() as _;
        b
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> VertexBuffer<T> {
        let n = self.k.1;
        VertexBuffer(self.build_untyped(), n, PhantomData)
    }
}

#[derive(Clone, Debug)]
pub struct IndexBuffer<T>(pub(crate) Buffer, PhantomData<T>);

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

#[derive(Debug, Clone)]
pub struct IndexKind<T: IndexBufferFormat>(PhantomData<T>);

impl<T: IndexBufferFormat> Default for IndexKind<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub type IndexBufferBuilder<'a, T> = BufferBuilder<'a, IndexKind<T>>;

impl<'a, T: IndexBufferFormat> IndexBufferBuilder<'a, T> {
    #[must_use]
    #[inline]
    pub fn len(mut self, elms: u32) -> Self {
        self.size = (elms as usize * size_of::<T>()) as _;
        self
    }

    #[must_use]
    #[inline]
    pub fn init_data(self, v: &'a [T]) -> Self {
        self.init_bytes(unsafe {
            std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), std::mem::size_of_val(v))
        })
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> IndexBuffer<T> {
        IndexBuffer(self.build_untyped(), PhantomData)
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
