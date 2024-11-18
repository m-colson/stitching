use std::{future::Future, time::Instant};

use axum::{
    extract::{ws::WebSocket, FromRequest, State, WebSocketUpgrade},
    handler::Handler,
};

pub(crate) fn ws_upgrader<M, S: Send + Sync + Clone + 'static, Fut>(
    cb: impl FnOnce(S, WebSocket) -> Fut + Send + Clone + 'static,
) -> impl Handler<(M, State<S>, WebSocketUpgrade), S>
where
    WebSocketUpgrade: FromRequest<S, M>,
    Fut: Future<Output = ()> + Send + 'static,
{
    |State(state), ws: WebSocketUpgrade| async move { ws.on_upgrade(move |sock| cb(state, sock)) }
}

#[inline]
pub(crate) fn time_op<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    let took = format!("{}us", start.elapsed().as_micros());
    tracing::event!(tracing::Level::DEBUG, name, took, "timing");
    out
}

#[allow(dead_code)]
#[inline]
pub(crate) async fn time_fut<T>(name: String, f: impl Future<Output = T>) -> T {
    let start = Instant::now();
    let out = f.await;
    let took = format!("{}us", start.elapsed().as_micros());
    tracing::event!(tracing::Level::DEBUG, name, took, "timing");
    out
}
