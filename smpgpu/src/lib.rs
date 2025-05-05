//! A simple wgpu abstraction layer for builder-based data types.

pub use wgpu::vertex_attr_array;

mod bind;
mod buffer;
mod cmd;
mod ctx;
mod mem;
mod sampler;
mod shader;
mod texture;

pub use bind::{AsBinding, AutoVisBindable, Bindings, VisBindable};
pub use buffer::{
    Buffer, BufferBuilder,
    typed::{IndexBuffer, StorageBuffer, Uniform, VertexBuffer},
};
pub use cmd::{
    AsRenderItem, Checkpoint, ColorAttachment, CommandBuilder, ComputeCheckpoint, ComputeItem,
    CopyOp, DepthAttachment, FragTarget, Pass, RenderCheckpoint, RenderItem,
};
pub use ctx::Context;
pub mod global;
pub use mem::{AsyncMemMapper, MemMapper};
pub mod model;
pub use sampler::Sampler;
pub use shader::{RenderShader, Shader};
pub use texture::Texture;

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
