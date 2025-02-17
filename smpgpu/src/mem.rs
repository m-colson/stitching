use std::{future::Future, pin::Pin};

#[derive(Default)]
pub struct MemMapper<'a> {
    chans: Vec<MappingCallback<'a>>,
}

impl<'a> MemMapper<'a> {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    #[inline]
    pub fn read_from(
        mut self,
        buf: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) + Send + 'a,
    ) -> Self {
        self.chans.push(MappingCallback::new_read(buf, cb));
        self
    }

    #[must_use]
    #[inline]
    pub fn write_to(
        mut self,
        buf: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) + Send + 'a,
    ) -> Self {
        self.chans.push(MappingCallback::new_write(buf, cb));
        self
    }

    #[must_use]
    #[inline]
    pub fn copy(self, src: &'a wgpu::Buffer, dst: &'a mut [u8]) -> Self {
        self.read_from(src, |view| dst.copy_from_slice(&view))
    }

    #[inline]
    pub async fn run_all(self) {
        futures::future::join_all(self.chans.into_iter().map(MappingCallback::wait_async)).await;
    }

    #[inline]
    pub fn block_all(self) {
        for mc in self.chans {
            mc.wait();
        }
    }
}

struct MappingCallback<'a>(
    &'a wgpu::Buffer,
    wgpu::BufferSlice<'a>,
    Box<dyn FnOnce(wgpu::BufferView<'a>) + Send + 'a>,
    kanal::OneshotReceiver<Result<(), wgpu::BufferAsyncError>>,
);

impl<'a> MappingCallback<'a> {
    pub fn new_read(
        b: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) + Send + 'a,
    ) -> Self {
        let (res_send, res_recv) = kanal::oneshot();
        let bs = b.slice(..);
        bs.map_async(wgpu::MapMode::Read, move |v| {
            // if this send fails, the user must have dropped the callback,
            // so they don't care about the result
            _ = res_send.send(v);
        });
        Self(b, bs, Box::new(cb), res_recv)
    }

    pub fn new_write(
        b: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) + Send + 'a,
    ) -> Self {
        let (res_send, res_recv) = kanal::oneshot();
        let bs = b.slice(..);
        bs.map_async(wgpu::MapMode::Write, move |v| {
            // if this send fails, the user must have dropped the callback,
            // so they don't care about the result
            _ = res_send.send(v);
        });
        Self(b, bs, Box::new(cb), res_recv)
    }
}

impl MappingCallback<'_> {
    async fn wait_async(self) {
        if mapping_failed(self.3.to_async().recv().await) {
            return;
        }

        let data = self.1.get_mapped_range();
        self.2(data);
        self.0.unmap();
    }

    pub fn wait(self) {
        if mapping_failed(self.3.recv()) {
            return;
        }

        let data = self.1.get_mapped_range();
        self.2(data);
        self.0.unmap();
    }
}

#[derive(Default)]
pub struct AsyncMemMapper<'a> {
    chans: Vec<AsyncMappingCallback<'a>>,
}

impl<'a> AsyncMemMapper<'a> {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn fold_from<T>(
        data: impl IntoIterator<Item = T>,
        cb: impl FnMut(Self, T) -> Self,
    ) -> Self {
        data.into_iter().fold(Self::new(), cb)
    }

    #[must_use]
    #[inline]
    pub fn read_from<F: Future<Output = ()> + Send + 'a>(
        mut self,
        buf: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) -> F + Send + 'a,
    ) -> Self {
        self.chans.push(AsyncMappingCallback::new_read(buf, cb));
        self
    }

    #[must_use]
    #[inline]
    pub fn write_to<F: Future<Output = ()> + Send + 'a>(
        mut self,
        buf: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) -> F + Send + 'a,
    ) -> Self {
        self.chans.push(AsyncMappingCallback::new_write(buf, cb));
        self
    }

    #[must_use]
    #[inline]
    pub fn copy(self, src: &'a wgpu::Buffer, dst: &'a mut [u8]) -> Self {
        self.read_from(src, |view| async move { dst.copy_from_slice(&view) })
    }

    #[inline]
    pub async fn run(self) {
        futures::future::join_all(self.chans.into_iter().map(AsyncMappingCallback::wait)).await;
    }
}

struct AsyncMappingCallback<'a>(
    &'a wgpu::Buffer,
    wgpu::BufferSlice<'a>,
    Box<
        dyn FnOnce(wgpu::BufferView<'a>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
            + Send
            + 'a,
    >,
    kanal::OneshotReceiver<Result<(), wgpu::BufferAsyncError>>,
);

impl<'a> AsyncMappingCallback<'a> {
    pub fn new_read<F: Future<Output = ()> + Send + 'a>(
        b: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) -> F + Send + 'a,
    ) -> Self {
        let (res_send, res_recv) = kanal::oneshot();
        let bs = b.slice(..);
        bs.map_async(wgpu::MapMode::Read, move |v| {
            // if this send fails, the user must have dropped the callback,
            // so they don't care about the result
            _ = res_send.send(v);
        });
        Self(b, bs, Box::new(move |v| Box::pin(cb(v))), res_recv)
    }

    pub fn new_write<F: Future<Output = ()> + Send + 'a>(
        b: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) -> F + Send + 'a,
    ) -> Self {
        let (res_send, res_recv) = kanal::oneshot();
        let bs = b.slice(..);
        bs.map_async(wgpu::MapMode::Write, move |v| {
            // if this send fails, the user must have dropped the callback,
            // so they don't care about the result
            _ = res_send.send(v);
        });
        Self(b, bs, Box::new(move |v| Box::pin(cb(v))), res_recv)
    }
}

impl AsyncMappingCallback<'_> {
    async fn wait(self) {
        if mapping_failed(self.3.to_async().recv().await) {
            return;
        }

        let data = self.1.get_mapped_range();
        self.2(data).await;
        self.0.unmap();
    }
}

fn mapping_failed(
    res: Result<Result<(), wgpu::BufferAsyncError>, kanal::OneshotReceiveError>,
) -> bool {
    let Ok(res) = res else {
        #[cfg(feature = "tracing")]
        tracing::error!("failed to receive confirmation of mapping operation");
        return true;
    };
    #[allow(unused_variables)]
    if let Err(err) = res {
        #[cfg(feature = "tracing")]
        tracing::error!("mapping operation failed: {err}");
        return true;
    }
    false
}
