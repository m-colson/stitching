use std::time::Instant;

use crate::{
    frame::{FrameBufferView, ToFrameBuffer},
    FrameSize,
};

use zerocopy::FromZeros;

pub trait OwnedWriteBuffer {
    type View<'a>: AsMut<[u8]>
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Self::View<'_>;
}

impl<T: std::ops::DerefMut<Target = [u8]>> OwnedWriteBuffer for T {
    type View<'a> = &'a mut [u8] where Self: 'a;

    fn owned_to_view(&mut self) -> Self::View<'_> {
        self
    }
}

pub struct FrameLoader<B: OwnedWriteBuffer> {
    req_send: kanal::Sender<(B, kanal::OneshotSender<B>)>,
    width: u16,
    height: u16,
    chans: u16,
}

impl<B: OwnedWriteBuffer + 'static> FrameLoader<B> {
    pub fn new_blocking(
        width: usize,
        height: usize,
        chans: usize,
        mut cb: impl FnMut(&mut [u8]) + Send + 'static,
    ) -> Self {
        let (req_send, req_recv) = kanal::bounded::<(B, kanal::OneshotSender<B>)>(4);

        tokio::task::spawn_blocking(move || {
            while let Ok((mut req, resp_send)) = req_recv.recv() {
                let start_time = Instant::now();
                cb(req.owned_to_view().as_mut());

                let elapsed = format!("{}us", start_time.elapsed().as_micros());
                tracing::debug!(took = elapsed, "indiv load");

                // if the receiver has been dropped, they don't want their buffer back!
                _ = resp_send.send(req);
            }
        });

        Self {
            req_send,
            width: width as _,
            height: height as _,
            chans: chans as _,
        }
    }

    pub fn give(&self, buf: B) -> FrameLoaderTicket<B> {
        let (buf_send, buf_recv) = kanal::oneshot();
        self.req_send.send((buf, buf_send)).unwrap();
        FrameLoaderTicket(buf_recv)
    }
}

impl<B: OwnedWriteBuffer> FrameLoader<B> {
    pub fn with_buffer<T>(self, buf: T) -> LoadingBuffer<T, B> {
        LoadingBuffer::new(self, buf)
    }
}

impl<T: From<Box<[u8]>>, B: OwnedWriteBuffer> From<FrameLoader<B>> for LoadingBuffer<T, B> {
    fn from(value: FrameLoader<B>) -> Self {
        let buf = <[u8]>::new_box_zeroed_with_elems(value.num_bytes())
            .unwrap()
            .into();
        value.with_buffer(buf)
    }
}

pub struct FrameLoaderTicket<R>(kanal::OneshotReceiver<R>);

impl<R> FrameLoaderTicket<R> {
    pub fn block_take(self) -> R {
        self.0.recv().unwrap()
    }

    pub async fn take(self) -> R {
        self.0.to_async().recv().await.unwrap()
    }
}

impl<B: OwnedWriteBuffer> FrameSize for FrameLoader<B> {
    fn width(&self) -> usize {
        self.width as _
    }

    fn height(&self) -> usize {
        self.height as _
    }

    fn chans(&self) -> usize {
        self.chans as _
    }
}

pub struct LoadingBuffer<T, B: OwnedWriteBuffer> {
    loader: FrameLoader<B>,
    inner: Option<T>,
}

impl<T, B: OwnedWriteBuffer> LoadingBuffer<T, B> {
    #[inline]
    pub fn new(loader: FrameLoader<B>, inner: T) -> Self {
        Self {
            loader,
            inner: Some(inner),
        }
    }
}

impl<B: OwnedWriteBuffer + 'static> LoadingBuffer<(), B> {
    #[inline]
    pub fn new_none(loader: FrameLoader<B>) -> Self {
        Self::new(loader, ())
    }

    #[inline]
    pub fn begin_load_with(&self, buf: B) -> FrameLoaderTicket<B> {
        self.loader.give(buf)
    }
}

impl<B: OwnedWriteBuffer + 'static> LoadingBuffer<B, B> {
    #[inline]
    pub fn begin_load(&mut self) -> Option<FrameLoaderTicket<B>> {
        self.inner.take().map(|buf| self.loader.give(buf))
    }

    #[inline]
    pub fn block_attach(&mut self, ticket: FrameLoaderTicket<B>) {
        self.inner.replace(ticket.block_take());
    }

    #[inline]
    pub async fn attach(&mut self, ticket: FrameLoaderTicket<B>) {
        self.inner.replace(ticket.take().await);
    }
}

impl<T: AsRef<[u8]>, B: OwnedWriteBuffer> LoadingBuffer<T, B>
where
    Self: FrameSize,
{
    pub fn as_view(&self) -> FrameBufferView<'_> {
        FrameBufferView::new(self.frame_size(), self.inner.as_ref().unwrap().as_ref())
    }
}

impl<T, B: OwnedWriteBuffer> LoadingBuffer<T, B>
where
    Self: FrameSize,
{
    pub fn as_empty_view(&self) -> FrameBufferView<'static> {
        FrameBufferView::new(self.frame_size(), &[])
    }
}

impl<T, B: OwnedWriteBuffer> FrameSize for LoadingBuffer<T, B> {
    fn width(&self) -> usize {
        self.loader.width as _
    }

    fn height(&self) -> usize {
        self.loader.height as _
    }

    fn chans(&self) -> usize {
        self.loader.chans as _
    }
}

impl<'a, T: AsRef<[u8]>, B: OwnedWriteBuffer> ToFrameBuffer<'a> for LoadingBuffer<T, B> {
    type Output = FrameBufferView<'a>;

    fn to_frame_buf(&'a self) -> Self::Output {
        self.as_view()
    }
}
