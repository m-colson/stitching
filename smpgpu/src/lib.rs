//! A simple wgpu abstraction layer for builder-based data types.

pub use wgpu::vertex_attr_array;

mod bind;
pub use bind::{AsBinding, AutoVisBindable, Bindings, VisBindable};

mod buffer;
pub use buffer::{Buffer, BufferBuilder};

mod cmd;
pub use cmd::{Checkpoint, ComputeCheckpoint, ComputeItem, Pass, RenderCheckpoint, RenderItem};

mod ctx;
pub use ctx::Context;

pub mod global;

mod mem;
pub use mem::MemMapper;

pub mod model;

mod sampler;
pub use sampler::Sampler;

mod shader;
pub use shader::{RenderShader, Shader};

mod texture;
pub use texture::Texture;

mod typed_buffer;
pub use typed_buffer::{IndexBuffer, StorageBuffer, Uniform, VertexBuffer};

/// The error type for operations in this crate
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The [`wgpu::Instance`] was unable to find a [`wgpu::Adapter`].
    #[error("failed to get adapter")]
    FailedToGetAdapater,
    /// There was a problem requesting a device from the [`wgpu::Adapter`]. See [`wgpu::RequestDeviceError`].
    #[error(transparent)]
    RequestDeviceError(#[from] wgpu::RequestDeviceError),
}

/// A specialized [`std::result::Result`] type for operations in this crate
pub type Result<T> = ::std::result::Result<T, Error>;

/// A writable view into a staging buffer. See [`wgpu::QueueWriteBufferView`].
pub type DirectWritableBufferView<'a> = wgpu::QueueWriteBufferView<'a>;

/// The `OntoDevice` trait allows for certain types to be turned
/// into their gpu equivalent automatically.
pub trait OntoDevice<T> {
    /// Take ownership of `self` and uses the provided device to create `T`
    fn onto_device(self, dev: &wgpu::Device) -> T;
}
