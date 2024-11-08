use std::{marker::PhantomData, ops::ControlFlow, time::Duration};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use stitch::{
    frame::{FrameBufferable, ToFrameBufferAsync},
    FrameBuffer,
};
use tokio::{sync::oneshot, time::Instant};
use zerocopy::{FromBytes, FromZeros, IntoBytes};

use super::App;

#[allow(dead_code)]
mod packet_kind {
    pub const NOP: u8 = 0;
    pub const SETTINGS_SYNC: u8 = 1;
    pub const UPDATE_FRAME: u8 = 2;
    pub const UPDATE_BOUNDS: u8 = 3;
}

pub struct VideoPacket<O: zerocopy::ByteOrder = zerocopy::LittleEndian>(Box<[u8]>, PhantomData<O>);

impl<O: zerocopy::ByteOrder> VideoPacket<O> {
    pub fn new(width: usize, height: usize, chans: usize) -> Self {
        let mut inner = <[u8]>::new_box_zeroed_with_elems(width * height * chans + 8).unwrap();
        inner[0] = packet_kind::UPDATE_FRAME;
        zerocopy::U16::<O>::new(width as u16)
            .write_to(&mut inner[1..3])
            .unwrap();
        zerocopy::U16::<O>::new(width as u16)
            .write_to(&mut inner[3..5])
            .unwrap();
        inner[5] = chans as u8;

        Self(inner, PhantomData)
    }

    pub fn take_message(&mut self) -> Message {
        let new_buf = Self::new(self.width(), self.height(), self.chans()).0;
        let old_buf = std::mem::replace(&mut self.0, new_buf);
        Message::Binary(old_buf.into_vec())
    }
}

impl<O: zerocopy::ByteOrder> FrameBufferable for VideoPacket<O> {}

impl<O: zerocopy::ByteOrder> FrameBuffer for VideoPacket<O> {
    fn width(&self) -> usize {
        zerocopy::U16::<O>::ref_from_bytes(&self.0[1..3])
            .unwrap()
            .get() as _
    }

    fn height(&self) -> usize {
        zerocopy::U16::<O>::ref_from_bytes(&self.0[3..5])
            .unwrap()
            .get() as _
    }

    fn chans(&self) -> usize {
        self.0[5] as usize
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0[8..]
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0[8..]
    }
}

pub async fn conn_state_machine(state: App, socket: WebSocket) {
    let (send_down, recv_down) = oneshot::channel::<()>();
    std::thread::spawn(move || {
        let rt: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let (sender, receiver) = socket.split();

        let local = tokio::task::LocalSet::new();

        let mut send_task = local.spawn_local(send_loop(state.clone(), sender));

        let mut recv_task = local.spawn_local(async move {
            receiver
                .try_take_while(|msg| {
                    let res = process_message(msg).is_continue();
                    async move { Ok(res) }
                })
                .count()
                .await
        });

        local.spawn_local(async move {
            tokio::select! {
                rv_a = (&mut send_task) => {
                    _ = rv_a.inspect_err(|e| println!("Error sending messages {e:?}"));
                    recv_task.abort();
                },
                rv_b = (&mut recv_task) => {
                    _ = rv_b.inspect_err(|e| println!("Error receiving messages {e:?}"));
                    send_task.abort();
                }
            }
        });

        rt.block_on(local);
        drop(send_down);
    });

    _ = recv_down.await;
}

async fn send_loop<S>(state: App, mut sender: S)
where
    S: SinkExt<Message> + std::marker::Unpin,
{
    const FPS: u64 = 15;
    loop {
        let frame_start_time = Instant::now();

        let proj = state.proj();
        let mut lock_buf = proj.buf.to_frame_async().await;

        let style = proj.ty.style;
        let forws = style.forward_proj(proj.spec, lock_buf.width(), lock_buf.height());
        forws.load_back(style, state.lock_cams().await, &mut lock_buf);

        let msg = lock_buf.take_message();

        if sender.send(msg).await.is_err() {
            return;
        }

        tokio::time::sleep_until(frame_start_time + Duration::from_millis(1000 / FPS)).await;

        // NOTE: Currently unnecessary, here for future reference
        // ----
        // If this fails, the connection has already closed anway.
        // _ = sender
        //     .send(Message::Close(Some(CloseFrame {
        //         code: axum::extract::ws::close_code::ABNORMAL,
        //         reason: Cow::from("Closing for unknown reason"),
        //     })))
        //     .await;
    }
}

fn process_message(msg: &Message) -> ControlFlow<()> {
    match msg {
        Message::Text(t) => {
            println!(">>> sent str: {t:?}");
        }
        Message::Binary(d) => {
            println!(">>> sent {} bytes: {:?}", d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> sent close with code {} and reason `{}`",
                    cf.code, cf.reason
                );
            } else {
                println!(">>> somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> sent pong with {v:?}");
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> sent ping with {v:?}");
        }
    }

    ControlFlow::Continue(())
}
