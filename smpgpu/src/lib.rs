pub use wgpu::vertex_attr_array;

mod bind;
pub use bind::{Bindable, Bindings};

mod buffer;
pub use buffer::{Buffer, BufferBuilder};

mod cmd;
pub use cmd::{ComputeCheckpoint, RenderCheckpoint};

pub mod ctx;
pub use ctx::Context;

mod mem;
pub use mem::MemMapper;

mod sampler;
pub use sampler::{Sampler, SamplerBuilder};

mod shader;
pub use shader::{RenderShader, Shader};

mod texture;
pub use texture::{Texture, TextureBuilder};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to get adapter")]
    FailedToGetAdapater,
    #[error(transparent)]
    RequestDeviceError(#[from] wgpu::RequestDeviceError),
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub type DirectWritableBufferView<'a> = wgpu::QueueWriteBufferView<'a>;

pub trait OntoDevice<T> {
    fn onto_device(self, dev: &wgpu::Device) -> T;
}

pub mod reexport {
    pub use wgpu::include_wgsl;
}
