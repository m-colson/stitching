use std::future::Future;

use axum::{
    extract::{ws::WebSocket, FromRequest, WebSocketUpgrade},
    handler::Handler,
};

pub(crate) fn ws_upgrader<M, S: Send + Sync + 'static, Fut>(
    cb: impl FnOnce(WebSocket) -> Fut + Send + Clone + 'static,
) -> impl Handler<(M, WebSocketUpgrade), S>
where
    WebSocketUpgrade: FromRequest<S, M>,
    Fut: Future<Output = ()> + Send + 'static,
{
    |ws: WebSocketUpgrade| async move { ws.on_upgrade(move |sock| cb(sock)) }
}
