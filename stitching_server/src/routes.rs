use std::path::PathBuf;

use axum::{routing::get, Router};

use crate::{log, util::ws_upgrader};

pub fn router<S: Clone + Send + Sync + 'static>(state: S) -> Router {
    Router::new()
        .fallback_service(
            tower_http::services::ServeDir::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
            )
            .append_index_html_on_directories(true),
        )
        .route("/video", get(ws_upgrader(video::conn_state_machine)))
        .layer(log::http_trace_layer())
        .with_state(state)
}

mod video {
    use std::{borrow::Cow, net::SocketAddr, ops::ControlFlow, time::Duration};

    use axum::extract::ws::{CloseFrame, Message, WebSocket};
    use futures_util::{SinkExt, StreamExt, TryStreamExt};

    // EXAMPLE CODE FOR WEBSOCKETS
    pub async fn conn_state_machine(mut socket: WebSocket) {
        let who = SocketAddr::new(std::net::IpAddr::V4([127, 0, 0, 1].into()), 0);
        // send a ping (unsupported by some browsers) just to kick things off and get a response
        if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
            println!("Pinged {who}...");
        } else {
            println!("Could not send ping {who}!");
            // no Error here since the only thing we can do is to close the connection.
            // If we can not send messages, there is no way to salvage the statemachine anyway.
            return;
        }

        if let Some(msg) = socket.recv().await {
            if let Ok(msg) = msg {
                if process_message(&msg, who).is_break() {
                    return;
                }
            } else {
                println!("client {who} abruptly disconnected");
                return;
            }
        }

        // Since each client gets individual statemachine, we can pause handling
        // when necessary to wait for some external event (in this case illustrated by sleeping).
        // Waiting for this client to finish getting its greetings does not prevent other clients from
        // connecting to server and receiving their greetings.
        for i in 1..5 {
            if socket
                .send(Message::Text(format!("Hi {i} times!")))
                .await
                .is_err()
            {
                println!("client {who} abruptly disconnected");
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // By splitting socket we can send and receive at the same time. In this example we will send
        // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
        let (mut sender, receiver) = socket.split();

        // Spawn a task that will push several messages to the client (does not matter what client does)
        let mut send_task = tokio::spawn(async move {
            let n_msg = 20;
            for i in 0..n_msg {
                // In case of any websocket error, we exit.
                if sender
                    .send(Message::Text(format!("Server message {i} ...")))
                    .await
                    .is_err()
                {
                    return i;
                }

                tokio::time::sleep(Duration::from_millis(1000)).await;
            }

            println!("Sending close to {who}...");
            if let Err(e) = sender
                .send(Message::Close(Some(CloseFrame {
                    code: axum::extract::ws::close_code::NORMAL,
                    reason: Cow::from("Goodbye"),
                })))
                .await
            {
                println!("Could not send Close due to {e}, probably it is ok?");
            }
            n_msg
        });

        // This second task will receive messages from client and print them on server console
        let mut recv_task = tokio::spawn(async move {
            receiver
                .try_take_while(|msg| {
                    let res = process_message(msg, who).is_continue();
                    async move { Ok(res) }
                })
                .count()
                .await
        });

        // If any one of the tasks exit, abort the other.
        tokio::select! {
            rv_a = (&mut send_task) => {
                match rv_a {
                    Ok(a) => println!("{a} messages sent to {who}"),
                    Err(a) => println!("Error sending messages {a:?}")
                }
                recv_task.abort();
            },
            rv_b = (&mut recv_task) => {
                match rv_b {
                    Ok(b) => println!("Received {b} messages"),
                    Err(b) => println!("Error receiving messages {b:?}")
                }
                send_task.abort();
            }
        }

        // returning from the handler closes the websocket connection
        println!("Websocket context {who} destroyed");
    }

    fn process_message(msg: &Message, who: SocketAddr) -> ControlFlow<()> {
        match msg {
            Message::Text(t) => {
                println!(">>> {who} sent str: {t:?}");
            }
            Message::Binary(d) => {
                println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
            }
            Message::Close(c) => {
                if let Some(cf) = c {
                    println!(
                        ">>> {} sent close with code {} and reason `{}`",
                        who, cf.code, cf.reason
                    );
                } else {
                    println!(">>> {who} somehow sent close message without CloseFrame");
                }
                return ControlFlow::Break(());
            }

            Message::Pong(v) => {
                println!(">>> {who} sent pong with {v:?}");
            }
            // You should never need to manually handle Message::Ping, as axum's websocket library
            // will do so for you automagically by replying with Pong and copying the v according to
            // spec. But if you need the contents of the pings you can see them here.
            Message::Ping(v) => {
                println!(">>> {who} sent ping with {v:?}");
            }
        }

        ControlFlow::Continue(())
    }
}
