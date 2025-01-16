use std::{
    num::NonZero,
    sync::{Arc, Weak},
};

use encase::{internal::WriteInto, ShaderType};

use crate::{Buffer, DirectWritableBufferView, Error, Result};

pub struct Context {
    dev: wgpu::Device,
    queue: wgpu::Queue,
    wake_poll: kanal::Sender<()>,
}

impl Context {
    #[must_use]
    #[inline]
    pub fn builder() -> ContextAdapterBuilder<'static, 'static> {
        ContextAdapterBuilder {
            inst: wgpu::Instance::default(),
            opts: wgpu::RequestAdapterOptions::default(),
        }
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
    ) -> DirectWritableBufferView<'a> {
        self.queue.write_buffer_with(buffer, offset, size).unwrap()
    }

    #[inline]
    pub fn write_uniform<T: ShaderType + WriteInto>(&self, buffer: &Buffer, v: &T) {
        let mut data = self.write_with(buffer, 0, buffer.size().try_into().unwrap());
        encase::UniformBuffer::new(data.as_mut()).write(v).unwrap();
    }

    #[inline]
    pub fn write_storage<T: ShaderType + WriteInto + ?Sized>(&self, buffer: &Buffer, v: &T) {
        let mut data = self.write_with(buffer, 0, buffer.size().try_into().unwrap());
        encase::StorageBuffer::new(data.as_mut()).write(v).unwrap();
    }

    #[must_use]
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

pub struct ContextAdapterBuilder<'a, 'b> {
    inst: wgpu::Instance,
    opts: wgpu::RequestAdapterOptions<'a, 'b>,
}

impl<'a, 'b> ContextAdapterBuilder<'a, 'b> {
    pub fn surface<'ap, 'bp>(
        self,
        s: impl Into<Option<&'ap wgpu::Surface<'bp>>>,
    ) -> ContextAdapterBuilder<'ap, 'bp> {
        ContextAdapterBuilder {
            inst: self.inst,
            opts: wgpu::RequestAdapterOptions {
                power_preference: self.opts.power_preference,
                force_fallback_adapter: self.opts.force_fallback_adapter,
                compatible_surface: s.into(),
            },
        }
    }

    pub async fn request_adapter(self) -> Result<ContextDeviceBuilder> {
        self.inst
            .request_adapter(&self.opts)
            .await
            .ok_or(Error::FailedToGetAdapater)
            .map(|adapter| ContextDeviceBuilder {
                adapter,
                features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
                limits: wgpu::Limits::downlevel_defaults(),
                hints: wgpu::MemoryHints::Performance,
            })
    }
}

pub struct ContextDeviceBuilder {
    adapter: wgpu::Adapter,
    features: wgpu::Features,
    limits: wgpu::Limits,
    hints: wgpu::MemoryHints,
}

impl ContextDeviceBuilder {
    pub async fn request_build(self) -> Result<Arc<Context>> {
        let (dev, queue) = self
            .adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: self.features,
                    required_limits: self.limits,
                    memory_hints: self.hints,
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

        spawn_poller(wake_recv, Arc::downgrade(&out));

        Ok(out)
    }
}

#[cfg(feature = "tokio_task_poller")]
fn spawn_poller(wake_recv: kanal::Receiver<()>, weak: Weak<Context>) {
    tokio::task::spawn_blocking(move || {
        while wake_recv.recv().is_ok() {
            let Some(ctx) = weak.upgrade() else { break };
            while ctx.block_poll_device() {}
        }
    });
}

#[cfg(not(feature = "tokio_task_poller"))]
fn spawn_poller(wake_recv: kanal::Receiver<()>, weak: Weak<Context>) {
    std::thread::spawn(move || {
        while wake_recv.recv().is_ok() {
            let Some(ctx) = weak.upgrade() else { break };
            while ctx.block_poll_device() {}
        }
    });
}
