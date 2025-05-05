//! This module contains the types and functions for using [`Loader`]s.
//!
//! A loader runs in the background, usually in a [`tokio::task::spawn_blocking`] task.
//! This allows you write loaders that iterface with otherwise synchronous APIs.
//! This struct contains the send side of a channel where the receive side is
//! owned by the loader task. With the [`Loader::give`] method, an owned buffer
//! and response sender is sent to the task over the channel who can then load
//! the buffer with data and return it with the response sender. A [`Ticket`] is
//! returned from the `give` method containing the receive side of the response
//! channel. The [`Ticket::take`] method will wait for the response and give
//! the buffer you originally sent back, with the data filled in by the loader.

use futures::future::join_all;

use crate::{Error, Result, buf::FrameSize, util::log_recv_err};

/// Can be implemented on types to signify that they own a writable buffer and
/// can provided a mutable view into that buffer when needed.
pub trait OwnedWriteBuffer {
    /// The type of view `Self` will return. Must implement [`AsMut`] for `[u8]`
    /// so it can written to.
    type View<'a>: AsMut<[u8]>
    where
        Self: 'a;

    /// Returns a mutable view into `self`.
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

/// Can be impleted on types to signify that they own something and can provided
/// it when needed. In this crate it is used as a more generic form of
/// [`OwnedWriteBuffer`] that allows you to write to multiple owned buffers
/// instead of only one.
pub trait OwnedWritable {
    /// The type of item that `Self` can give out.
    type Inner<'a>
    where
        Self: 'a;

    /// Returns the item that `self` can give out.
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

/// Contains information about the resulting image and the channel used to send
/// update requests. See [`crate::loader`] module docs.
#[derive(Clone, Debug)]
pub struct Loader<B> {
    req_send: kanal::Sender<(B, kanal::Sender<B>)>,
    width: u32,
    height: u32,
    chans: u32,
}

impl<B: OwnedWriteBuffer + Send + 'static> Loader<B> {
    /// Creates a new loader that will run in a blocking task. The callback will
    /// be called for every request to the loader.
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

    /// Creates a new loader that will run in a blocking task. The callback will
    /// be called once inside the created task and is expected to repeatedly wait for
    /// requests over the callback's receiver channel.
    pub fn new_blocking_manual_recv(
        width: u32,
        height: u32,
        chans: u32,
        cb: impl FnOnce(kanal::Receiver<(B, kanal::Sender<B>)>) + Send + 'static,
    ) -> Self {
        let (req_send, req_recv) = kanal::bounded::<(B, kanal::Sender<B>)>(4);

        tokio::task::spawn_blocking(move || cb(req_recv));

        Self {
            req_send,
            width,
            height,
            chans,
        }
    }

    /// Sends a request to `self` with the given buffer.
    pub fn give(&self, buf: B) -> Result<Ticket<B>> {
        let (buf_send, buf_recv) = kanal::bounded(1);
        self.req_send
            .send((buf, buf_send))
            .map_err(|_| Error::BufferLost)
            .map(|()| Ticket(buf_recv))
    }
}

impl<B1: OwnedWriteBuffer + 'static, B2: OwnedWriteBuffer + 'static> Loader<(B1, B2)> {
    /// Sends a request to `self` with the 2 given buffers.
    pub fn give2(&self, buf1: B1, buf2: B2) -> Result<Ticket<(B1, B2)>> {
        let (buf_send, buf_recv) = kanal::bounded(1);
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
    /// Creates a new loader that will run in a normal async task. The callback will
    /// be called for every request to the loader.
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

    /// Creates a new loader that will run in a normal async task. The callback will
    /// be called once inside the created task and is expected to repeatedly wait for
    /// requests over the callback's receiver channel.
    pub fn new_async_manual_recv<F>(
        width: u32,
        height: u32,
        chans: u32,
        cb: impl FnOnce(kanal::AsyncReceiver<(B, kanal::Sender<B>)>) -> F,
    ) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (req_send, req_recv) = kanal::bounded::<(B, kanal::Sender<B>)>(4);

        tokio::task::spawn(cb(req_recv.to_async()));

        Self {
            req_send,
            width,
            height,
            chans,
        }
    }
}

/// Calls the callback for every ticket with the corresponding item and waits
/// for all the futures to complete.
#[inline]
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

/// Calls [`Ticket::take`] for every ticket, waits for them to complete and gets
/// rid of the results.
#[inline]
pub async fn discard_tickets<B: OwnedWriteBuffer + Send>(tickets: Vec<Ticket<B>>) {
    join_all(tickets.into_iter().map(|ticket| async move {
        _ = ticket.take().await;
    }))
    .await;
}

/// Calls [`Ticket::block_take`] for every ticket, and gets rid of the results.
#[inline]
pub fn block_discard_tickets<B: OwnedWriteBuffer>(tickets: Vec<Ticket<B>>) {
    for ticket in tickets {
        _ = ticket.block_take();
    }
}

/// Represents a buffer response from a [`Loader`] that could happen.
pub struct Ticket<R>(kanal::Receiver<R>);

impl<R> Ticket<R> {
    /// Blocks while waiting for a response from the loader.
    /// # Errors
    /// Returns [`Error::BufferLost`] if it no longer possible for a response to
    /// happen, like if the loader exited.
    pub fn block_take(self) -> Result<R> {
        self.0.recv().map_err(|_| Error::BufferLost)
    }
}

impl<R: Send> Ticket<R> {
    /// Waits for a response from the loader.
    /// # Errors
    /// Returns [`Error::BufferLost`] if it no longer possible for a response to
    /// happen, like if the loader exited.
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
