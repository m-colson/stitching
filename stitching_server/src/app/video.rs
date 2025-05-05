//! This module contains function that handle clients connections.

use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use futures_util::{SinkExt, StreamExt};

use crate::util::{IntervalTimer, Metrics};

use super::{App, proto::RecvPacket};

/// Spawns tasks to handle the send and receive sides of the `socket` and aborts the tasks if the other fails.
/// Completes when connection is closed.
pub async fn conn_state_machine(state: App, socket: WebSocket) {
    let (sender, receiver) = socket.split();

    let mut send_task = tokio::spawn(send_loop(state.clone(), sender));
    let mut recv_task = tokio::spawn(recv_loop(state.clone(), receiver));

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
}

async fn send_loop<S>(state: App, mut sender: S)
where
    S: SinkExt<Message> + Unpin + Send,
{
    while let Some(msg) = state.ws_frame().await {
        let mut timer = IntervalTimer::new();
        let res = sender.send(msg).await;
        timer.mark("send-frame");

        if res.is_err() {
            break;
        }
    }

    // If this fails, the connection has already closed anyway.
    _ = sender
        .send(Message::Close(Some(CloseFrame {
            code: axum::extract::ws::close_code::AWAY,
            reason: Utf8Bytes::from_static("No more frames"),
        })))
        .await;
}

async fn recv_loop<R>(_state: App, mut receiver: R)
where
    R: StreamExt<Item = Result<Message, axum::Error>> + Unpin + Send,
{
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Binary(raw) = msg {
            let Some(p) = RecvPacket::from_raw(&raw) else {
                tracing::error!(
                    "failed to parse packet from client starting with {:?}",
                    &raw[..raw.len().min(64)]
                );
                continue;
            };

            match p {
                RecvPacket::Nop => {}
                RecvPacket::Timing(timing) => {
                    let (dur, delay) = timing.info_now();
                    Metrics::push("client-update", delay.as_secs_f64() * 1000.);
                    Metrics::push("client-decode", dur.as_secs_f64() * 1000.);
                }
            }
        }
    }
}
