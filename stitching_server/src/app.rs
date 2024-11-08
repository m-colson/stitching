use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{routing::get, Router};
use stitch::{
    camera::live::LiveBufferGaurd, Camera, CameraGroupAsync, LiveBuffer, LiveSpec, ProjSpec,
};
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    sync::Mutex,
};
use video::VideoPacket;

use crate::{log, util::ws_upgrader};

mod video;

#[derive(Clone)]
pub struct App(Arc<AppInner>);

struct AppInner {
    pub proj: Camera<Mutex<VideoPacket>, ProjSpec>,
    pub cams: Vec<Camera<LiveBuffer, LiveSpec>>,
}

impl App {
    pub fn into_router(self) -> Router {
        Router::new()
            .fallback_service(tower_http::services::ServeDir::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
            ))
            .route("/video", get(ws_upgrader(video::conn_state_machine)))
            .layer(log::http_trace_layer())
            .with_state(self)
    }

    pub async fn from_toml_cfg(
        p: impl AsRef<Path>,
        proj_w: usize,
        proj_h: usize,
    ) -> stitch::Result<Self> {
        AppInner::from_toml_cfg(p, proj_w, proj_h)
            .await
            .map(Arc::new)
            .map(Self)
    }

    pub async fn listen_and_serve(self, a: impl ToSocketAddrs + Debug) -> std::io::Result<()> {
        let bind = TcpListener::bind(&a).await?;
        tracing::info!("listening on {a:?}");

        axum::serve(bind, self.into_router()).await
    }

    pub fn proj(&self) -> &Camera<Mutex<VideoPacket>, ProjSpec> {
        &self.0.proj
    }

    pub async fn lock_cams(&self) -> Vec<Camera<LiveBufferGaurd, LiveSpec>> {
        self.0.cams.to_cams_async().await
    }
}

impl AppInner {
    pub async fn from_toml_cfg(
        p: impl AsRef<Path>,
        proj_w: usize,
        proj_h: usize,
    ) -> stitch::Result<Self> {
        let cfg = stitch::Config::open_live(&p)?;
        tracing::info!("opened config at {:?}", p.as_ref());

        let proj = Camera::new(
            cfg.proj.spec.with_dims(proj_w as f32, proj_h as f32),
            cfg.proj.ty,
            Mutex::new(VideoPacket::new(proj_w, proj_h, 4)),
        );

        let cams = futures::future::join_all(cfg.cameras.iter().map(
            |c: &stitch::CameraConfig<LiveSpec>| async move {
                tracing::info!("loading camera {:?}", c.ty.live_index);
                c.load_live().await
            },
        ))
        .await
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;

        tracing::info!("finished loading cameras");

        Ok(AppInner { proj, cams })
    }
}
