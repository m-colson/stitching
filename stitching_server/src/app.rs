use std::{
    fmt::Debug,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{extract::ws::Message, routing::get, Router};
use stitch::proj::ProjectionStyle;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::{log, util::ws_upgrader};

mod stitcher;
use stitcher::Sticher;

mod proto;
mod video;

#[derive(Clone)]
pub struct App(Arc<AppInner>);

struct AppInner {
    pub stitcher: Sticher,
}

impl App {
    pub fn into_router(self) -> Router {
        Router::new()
            .fallback_service(tower_http::services::ServeDir::new(PathBuf::from(
                "stitching_server/assets",
            )))
            .route("/video", get(ws_upgrader(video::conn_state_machine)))
            .layer(log::http_trace_layer())
            .with_state(self)
    }

    pub async fn from_toml_cfg(
        p: impl AsRef<Path> + Send,
        proj_w: usize,
        proj_h: usize,
    ) -> stitch::Result<Self> {
        AppInner::from_toml_cfg(p, proj_w, proj_h)
            .await
            .map(Arc::new)
            .map(Self)
    }

    pub async fn listen_and_serve(
        self,
        a: impl ToSocketAddrs + Debug + Send + Sync,
    ) -> std::io::Result<()> {
        let bind = TcpListener::bind(&a).await?;
        tracing::info!("listening on {a:?}");

        axum::serve(bind, self.into_router()).await
    }

    pub async fn listen_and_serve_until(
        self,
        a: impl ToSocketAddrs + Debug + Send + Sync,
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> std::io::Result<()> {
        let bind = TcpListener::bind(&a).await?;
        tracing::info!("listening on {a:?}");

        axum::serve(bind, self.into_router())
            .with_graceful_shutdown(signal)
            .await
    }

    pub async fn ws_frame(&self) -> Option<Message> {
        self.0.stitcher.next_frame_msg().await
    }

    pub fn update_style<F: FnOnce(&mut ProjectionStyle) + Send + 'static>(&self, f: F) {
        self.0.stitcher.update_style(f);
    }
}

impl AppInner {
    pub async fn from_toml_cfg(
        p: impl AsRef<Path> + Send,
        proj_w: usize,
        proj_h: usize,
    ) -> stitch::Result<Self> {
        let cfg = stitch::proj::Config::open(&p)?;
        tracing::info!("opened config at {:?}", p.as_ref());

        Ok(Self {
            stitcher: Sticher::from_cfg_gpu(cfg, proj_w, proj_h).await,
        })
    }
}
