use crate::{Error, Result, buf::FrameSize, util::log_recv_err};

pub trait OwnedWriteBuffer {
    type View<'a>: AsMut<[u8]>
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Option<Self::View<'_>>;
}

impl OwnedWriteBuffer for Vec<u8> {
    type View<'a>
        = &'a mut [u8]
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Option<Self::View<'_>> {
        Some(self)
    }
}

pub trait OwnedWritable {
    type Inner<'a>
    where
        Self: 'a;

    fn owned_to_inner(&mut self) -> Option<Self::Inner<'_>>;
}

impl<B: OwnedWriteBuffer + 'static> OwnedWritable for B {
    type Inner<'a> = B::View<'a>;

    fn owned_to_inner(&mut self) -> Option<Self::Inner<'_>> {
        self.owned_to_view()
    }
}

impl<B1: OwnedWritable, B2: OwnedWritable> OwnedWritable for (B1, B2) {
    type Inner<'a>
        = (B1::Inner<'a>, B2::Inner<'a>)
    where
        Self: 'a;
    fn owned_to_inner(&mut self) -> Option<Self::Inner<'_>> {
        Some((self.0.owned_to_inner()?, self.1.owned_to_inner()?))
    }
}

#[derive(Clone, Debug)]
pub struct Loader<B> {
    req_send: kanal::Sender<(B, kanal::OneshotSender<B>)>,
    width: u32,
    height: u32,
    chans: u32,
}

impl<B: OwnedWriteBuffer + 'static> Loader<B> {
    pub fn new_blocking(
        width: u32,
        height: u32,
        chans: u32,
        mut cb: impl FnMut(&mut [u8]) + Send + 'static,
    ) -> Self {
        Self::new_blocking_manual_recv(width, height, chans, move |req_recv| {
            while let Ok((mut req, resp_send)) = req_recv.recv().inspect_err(log_recv_err) {
                if let Some(mut v) = req.owned_to_view() {
                    cb(v.as_mut());
                }
                // if the receiver has been dropped, they don't want their buffer back!
                _ = resp_send.send(req);
            }
        })
    }

    pub fn new_blocking_manual_recv(
        width: u32,
        height: u32,
        chans: u32,
        cb: impl FnOnce(kanal::Receiver<(B, kanal::OneshotSender<B>)>) + Send + 'static,
    ) -> Self {
        let (req_send, req_recv) = kanal::bounded::<(B, kanal::OneshotSender<B>)>(4);

        tokio::task::spawn_blocking(move || cb(req_recv));

        Self {
            req_send,
            width,
            height,
            chans,
        }
    }

    /// # Errors
    /// loader doesn't exist anymore
    pub fn give(&self, buf: B) -> Result<Ticket<B>> {
        let (buf_send, buf_recv) = kanal::oneshot();
        self.req_send
            .send((buf, buf_send))
            .map_err(|_| Error::BufferLost)
            .map(|()| Ticket(buf_recv))
    }
}

impl<B1: OwnedWriteBuffer + 'static, B2: OwnedWriteBuffer + 'static> Loader<(B1, B2)> {
    pub fn give2(&self, buf1: B1, buf2: B2) -> Result<Ticket<(B1, B2)>> {
        let (buf_send, buf_recv) = kanal::oneshot();
        self.req_send
            .send(((buf1, buf2), buf_send))
            .map_err(|_| Error::BufferLost)
            .map(|()| Ticket(buf_recv))
    }
}

impl<B: OwnedWritable + Send + 'static> Loader<B>
where
    for<'a> B::Inner<'a>: Send,
{
    pub fn new_async<F>(
        width: u32,
        height: u32,
        chans: u32,
        mut cb: impl FnMut(B::Inner<'_>) -> F + Send + 'static,
    ) -> Self
    where
        F: Future<Output = ()> + Send,
    {
        Self::new_async_manual_recv(width, height, chans, |req_recv| async move {
            while let Ok((mut req, resp_send)) = req_recv.recv().await.inspect_err(log_recv_err) {
                if let Some(v) = req.owned_to_inner() {
                    cb(v).await;
                }
                // if the receiver has been dropped, they don't want their buffer back!
                _ = resp_send.send(req);
            }
        })
    }

    pub fn new_async_manual_recv<F>(
        width: u32,
        height: u32,
        chans: u32,
        cb: impl FnOnce(kanal::AsyncReceiver<(B, kanal::OneshotSender<B>)>) -> F,
    ) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (req_send, req_recv) = kanal::bounded::<(B, kanal::OneshotSender<B>)>(4);

        tokio::task::spawn(cb(req_recv.to_async()));

        Self {
            req_send,
            width,
            height,
            chans,
        }
    }
}

pub async fn collect_mapped_tickets<B: OwnedWriteBuffer + Send, K: Sync, T: FrameSize + Sync, O>(
    tickets: Vec<Ticket<B>>,
    items: impl IntoIterator<Item = T>,
    f: impl AsyncFn(Ticket<B>, T) -> O + 'static,
) -> Vec<O> {
    futures::future::join_all(
        items
            .into_iter()
            .zip(tickets)
            .map(|(c, ticket)| f(ticket, c)),
    )
    .await
    // c.with_map_fut(|b| async {
    //     let _ = ticket.take().await;
    //     b.as_empty_view()
    // })
}

#[inline]
pub async fn discard_tickets<B: OwnedWriteBuffer + Send>(tickets: Vec<Ticket<B>>) {
    for ticket in tickets {
        _ = ticket.take().await;
    }
}

#[inline]
pub fn block_discard_tickets<B: OwnedWriteBuffer>(tickets: Vec<Ticket<B>>) {
    for ticket in tickets {
        _ = ticket.block_take();
    }
}

pub struct Ticket<R>(kanal::OneshotReceiver<R>);

impl<R> Ticket<R> {
    /// # Errors
    /// loading thread exited
    pub fn block_take(self) -> Result<R> {
        self.0.recv().map_err(|_| Error::BufferLost)
    }
}

impl<R: Send> Ticket<R> {
    /// # Errors
    /// loading thread exited
    pub async fn take(self) -> Result<R> {
        self.0
            .to_async()
            .recv()
            .await
            .map_err(|_| Error::BufferLost)
    }
}

impl<B: OwnedWriteBuffer> FrameSize for Loader<B> {
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
