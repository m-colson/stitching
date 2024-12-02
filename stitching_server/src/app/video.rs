use std::borrow::Cow;

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};

use crate::util::{IntervalTimer, Metrics};

use super::{proto::RecvPacket, App};

pub async fn conn_state_machine(state: App, socket: WebSocket) {
    let (sender, receiver) = socket.split();

    let mut send_task = tokio::spawn(send_loop(state.clone(), sender));
    let mut recv_task = tokio::spawn(recv_loop(state.clone(), receiver));

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
            reason: Cow::from("No more frames"),
        })))
        .await;
}

async fn recv_loop<R>(state: App, mut receiver: R)
where
    R: StreamExt<Item = Result<Message, axum::Error>> + Unpin + Send,
{
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Binary(raw) = msg {
            let Some(p) = RecvPacket::from_raw(&raw) else {
                tracing::error!(
                    "failed to parse packet from client starting with {:?}",
                    &raw[..raw.len().min(8)]
                );
                continue;
            };

            match p {
                RecvPacket::Nop => {}
                RecvPacket::SettingsSync(sp) => {
                    state.update_style(move |proj_spec| {
                        *proj_spec = sp.view_type(proj_spec.radius());
                    });
                }
                RecvPacket::Timing(timing) => {
                    let (took, delay) = timing.info_now();
                    Metrics::push("client-update", delay.as_secs_f64() * 1000.);

                    let took = format!("{took:.1?}");
                    let delay = format!("{delay:.1?}");
                    tracing::info!(delay, took, "client update");
                }
            }
        }
    }
}
