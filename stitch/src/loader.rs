use crate::{
    buf::{FrameBufferView, FrameSize},
    camera::Camera,
    Error, Result,
};
pub trait OwnedWriteBuffer {
    type View<'a>: AsMut<[u8]>
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Self::View<'_>;
}

impl<T: std::ops::DerefMut<Target = [u8]>> OwnedWriteBuffer for T {
    type View<'a>
        = &'a mut [u8]
    where
        Self: 'a;

    fn owned_to_view(&mut self) -> Self::View<'_> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct Loader<B: OwnedWriteBuffer> {
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
        let (req_send, req_recv) = kanal::bounded::<(B, kanal::OneshotSender<B>)>(4);

        tokio::task::spawn_blocking(move || {
            loop {
                match req_recv.recv() {
                    Ok((mut req, resp_send)) => {
                        cb(req.owned_to_view().as_mut());
                        // if the receiver has been dropped, they don't want their buffer back!
                        _ = resp_send.send(req);
                    }
                    Err(err) => {
                        match err {
                            kanal::ReceiveError::SendClosed => {
                                tracing::warn!("loader exiting because all senders have dropped")
                            }
                            kanal::ReceiveError::Closed => {
                                tracing::warn!("loader exiting bacause it was closed")
                            }
                        }
                        break;
                    }
                }
            }
        });

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

pub async fn collect_empty_camera_tickets<
    B: OwnedWriteBuffer + Send,
    K: Sync,
    T: FrameSize + Sync,
>(
    tickets: Vec<Ticket<B>>,
    cams: &[Camera<T>],
) -> Vec<Camera<FrameBufferView<'static>>> {
    futures::future::join_all(cams.iter().zip(tickets).map(|(c, ticket)| {
        c.with_map_fut(|b| async {
            let _ = ticket.take().await;
            b.as_empty_view()
        })
    }))
    .await
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
