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
    Option<Box<dyn FnOnce(wgpu::BufferView<'a>) + Send + 'a>>,
    kanal::Receiver<Result<(), wgpu::BufferAsyncError>>,
);

impl<'a> MappingCallback<'a> {
    pub fn new_read(
        b: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) + Send + 'a,
    ) -> Self {
        let (res_send, res_recv) = kanal::bounded(1);
        let bs = b.slice(..);
        bs.map_async(wgpu::MapMode::Read, move |v| res_send.send(v).unwrap());
        Self(b, bs, Some(Box::new(cb)), res_recv)
    }

    pub fn new_write(
        b: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) + Send + 'a,
    ) -> Self {
        let (res_send, res_recv) = kanal::bounded(1);
        let bs = b.slice(..);
        bs.map_async(wgpu::MapMode::Write, move |v| res_send.send(v).unwrap());
        Self(b, bs, Some(Box::new(cb)), res_recv)
    }
}

impl MappingCallback<'_> {
    async fn wait_async(mut self) {
        self.3.clone().to_async().recv().await.unwrap().unwrap();
        let data = self.1.get_mapped_range();
        self.2.take().expect("wait called twice")(data);
        self.0.unmap();
    }

    pub fn wait(mut self) {
        self.3.clone().recv().unwrap().unwrap();
        let data = self.1.get_mapped_range();
        self.2.take().expect("wait called twice")(data);
        self.0.unmap();
    }
}
