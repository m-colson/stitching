use std::future::Future;

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

// pub(crate) fn ws_upgrader_not_send<M, S: Send + Sync + Clone + 'static, Fut>(
//     cb: impl FnOnce(S, WebSocket) -> Fut + Clone + 'static,
// ) -> impl Handler<(M, State<S>, WebSocketUpgrade), S>
// where
//     WebSocketUpgrade: FromRequest<S, M>,
//     Fut: Future<Output = ()> + 'static,
// {
//     |State(state), ws: WebSocketUpgrade| async move {
//         ws.on_upgrade(move |sock| tokio::task::LocalSet::new().run_until(cb(S,)))
//     }
// }
