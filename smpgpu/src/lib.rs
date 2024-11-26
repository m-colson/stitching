use std::{num::NonZero, sync::Arc};
pub use wgpu::{include_wgsl, vertex_attr_array, QueueWriteBufferView};

mod bind;
pub use bind::{Bindable, Bindings};

mod buffer;
pub use buffer::{Buffer, BufferBuilder};

mod cmd;
pub use cmd::{ComputeCheckpoint, RenderCheckpoint};

mod mem;
use encase::{internal::WriteInto, ShaderType};
pub use mem::MemMapper;

mod sampler;
pub use sampler::{Sampler, SamplerBuilder};

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

pub struct Context {
    pub dev: wgpu::Device,
    queue: wgpu::Queue,
    wake_poll: kanal::Sender<()>,
}

impl Context {
    #[inline]
    fn features() -> wgpu::Features {
        wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
    }

    #[inline]
    fn limits() -> wgpu::Limits {
        let mut limits = wgpu::Limits::downlevel_defaults();
        limits.max_texture_dimension_3d = 2048;
        limits
    }

    pub async fn new() -> Result<Arc<Self>> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or(Error::FailedToGetAdapater)?;

        let (dev, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: Self::features(),
                    required_limits: Self::limits(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(Error::from)?;

        let (wake_poll, wake_recv) = kanal::unbounded();

        let out = Arc::new(Context {
            dev,
            queue,
            wake_poll,
        });

        let weak = Arc::downgrade(&out);
        tokio::task::spawn_blocking(move || {
            while wake_recv.recv().is_ok() {
                let Some(ctx) = weak.upgrade() else { break };
                while ctx.block_poll_device() {}
            }
        });

        Ok(out)
    }

    #[inline]
    pub fn signal_wake(&self) {
        self.wake_poll.send(()).expect("poller has died");
    }

    #[inline]
    pub fn submit(&self, buf: impl IntoIterator<Item = wgpu::CommandBuffer>) {
        self.queue.submit(buf);
    }

    #[inline]
    pub fn write_with<'a>(
        &'a self,
        buffer: &'a Buffer,
        offset: u64,
        size: NonZero<u64>,
    ) -> QueueWriteBufferView<'a> {
        self.queue.write_buffer_with(buffer, offset, size).unwrap()
    }

    #[inline]
    pub fn write_uniform<T: ShaderType + WriteInto>(&self, buffer: &Buffer, v: &T) {
        let mut data = self.write_with(buffer, 0, buffer.size().try_into().unwrap());
        encase::UniformBuffer::new(data.as_mut()).write(v).unwrap()
    }

    #[inline]
    pub fn write_storage<T: ShaderType + WriteInto>(&self, buffer: &Buffer, v: &T) {
        let mut data = self.write_with(buffer, 0, buffer.size().try_into().unwrap());
        encase::StorageBuffer::new(data.as_mut()).write(v).unwrap();
    }

    #[inline]
    pub fn block_poll_device(&self) -> bool {
        !self.dev.poll(wgpu::Maintain::wait()).is_queue_empty()
    }
}

impl AsRef<wgpu::Device> for Context {
    #[inline]
    fn as_ref(&self) -> &wgpu::Device {
        &self.dev
    }
}
