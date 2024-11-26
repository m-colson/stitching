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

pub(crate) struct IntervalTimer {
    base_time: Instant,
    mark_time: Instant,
}

impl IntervalTimer {
    #[inline]
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            base_time: now,
            mark_time: now,
        }
    }

    #[inline]
    pub fn start(&mut self) {
        let now = Instant::now();
        self.base_time = now;
        self.mark_time = now;
    }

    #[inline]
    pub fn mark(&mut self, name: &str) {
        let now = Instant::now();

        let took = format!("{:.1?}", now - self.mark_time);
        tracing::event!(tracing::Level::DEBUG, took, "{}", name);

        self.mark_time = now;
    }

    #[inline]
    pub fn log_iters_per_sec(&self, name: &str) {
        let diff = self.base_time.elapsed();
        let fps = format!("{:.1}", 1. / diff.as_secs_f32());
        let took = format!("{:.1?}", diff);
        tracing::info!(fps, took, "{}", name);
    }
}
