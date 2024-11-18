use std::{borrow::Cow, f32::consts::PI, marker::PhantomData, ops::ControlFlow};

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use stitch::frame::{FrameBuffer, FrameBufferMut, FrameSize};
use zerocopy::{FromBytes, FromZeros, IntoBytes};

use crate::util::time_fut;

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
        zerocopy::U16::<O>::new(height as u16)
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

impl<O: zerocopy::ByteOrder> FrameSize for VideoPacket<O> {
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
}

impl<O: zerocopy::ByteOrder> FrameBuffer for VideoPacket<O> {
    fn as_bytes(&self) -> &[u8] {
        &self.0[8..]
    }
}

impl<O: zerocopy::ByteOrder> FrameBufferMut for VideoPacket<O> {
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0[8..]
    }
}

pub async fn conn_state_machine(state: App, socket: WebSocket) {
    let (sender, receiver) = socket.split();

    let mut send_task = tokio::spawn(send_loop(state.clone(), sender));

    let mut recv_task = tokio::spawn(async move {
        receiver
            .try_take_while(|msg| {
                let res = process_message(msg).is_continue();
                async move { Ok(res) }
            })
            .count()
            .await
    });

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
}

async fn send_loop<S>(state: App, mut sender: S)
where
    S: SinkExt<Message> + std::marker::Unpin,
{
    while let Some(msg) = state.ws_frame().await {
        if time_fut("send frame".to_string(), sender.send(msg))
            .await
            .is_err()
        {
            break;
        }

        state.update_spec(|s| {
            s.azimuth += PI / 180.;
        });
    }

    // If this fails, the connection has already closed anyway.
    _ = sender
        .send(Message::Close(Some(CloseFrame {
            code: axum::extract::ws::close_code::AWAY,
            reason: Cow::from("No more frames"),
        })))
        .await;
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
        Message::Ping(v) => {
            println!(">>> sent ping with {v:?}");
        }
    }

    ControlFlow::Continue(())
}
