type MapperCallback<'a> = Box<dyn FnOnce(wgpu::BufferView<'a>) + 'a>;

#[derive(Default)]
pub struct MemMapper<'a> {
    slices: Vec<(&'a wgpu::Buffer, MapperCallback<'a>)>,
}

impl<'a> MemMapper<'a> {
    #[must_use]
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    #[inline]
    pub fn with_cb(
        mut self,
        buf: &'a wgpu::Buffer,
        cb: impl FnOnce(wgpu::BufferView<'a>) + 'a,
    ) -> Self {
        self.slices.push((buf, Box::new(cb)));
        self
    }

    #[cfg(feature = "tokio")]
    pub async fn run_all(self) {
        let chans = self.slices.into_iter().map(|(b, cb)| {
            let (res_send, res_recv) = kanal::bounded(1);
            let bs = b.slice(..);
            bs.map_async(wgpu::MapMode::Read, move |v| res_send.send(v).unwrap());
            (b, bs, cb, res_recv)
        });

        futures::future::join_all(chans.map(|(b, bs, cb, res_recv)| async move {
            res_recv.to_async().recv().await.unwrap().unwrap();
            let data = bs.get_mapped_range();
            cb(data);
            b.unmap();
        }))
        .await;
    }

    pub fn block_all(self) {
        let chans = self
            .slices
            .into_iter()
            .map(|(b, cb)| {
                let (res_send, res_recv) = kanal::bounded(1);
                let bs = b.slice(..);
                bs.map_async(wgpu::MapMode::Read, move |v| res_send.send(v).unwrap());
                (b, bs, cb, res_recv)
            })
            .collect::<Vec<_>>();

        for (b, bs, cb, res_recv) in chans {
            _ = res_recv.recv().unwrap();
            let data = bs.get_mapped_range();
            cb(data);
            b.unmap();
        }
    }
}
