use std::{
    fmt::Debug,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Router,
    extract::{self, State, ws::Message},
    routing::{get, put},
};
use stitch::{
    camera::ViewParams,
    proj::{ProjectionStyle, ViewStyle},
};
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
    pub camera_views: Vec<ViewParams>,
}

impl App {
    pub fn into_router(self) -> Router {
        Router::new()
            .fallback_service(tower_http::services::ServeDir::new(PathBuf::from(
                "stitching_server/assets",
            )))
            .route("/video", get(ws_upgrader(video::conn_state_machine)))
            .route("/set/view/{id}", put(set_view_handler))
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

    pub fn update_proj_style(&self, f: impl FnOnce(&mut ProjectionStyle) + Send + 'static) {
        self.0.stitcher.update_proj_style(f);
    }

    #[allow(dead_code)]
    pub fn update_view_style(&self, f: impl FnOnce(&mut ViewStyle) + Send + 'static) {
        self.0.stitcher.update_view_style(f);
    }
}

async fn set_view_handler(State(app): State<App>, p: extract::Path<i32>) {
    let new_style = match p.0 {
        -1 => ViewStyle::Orbit {
            dist: 50.,
            z: 30.,
            theta: 0.,
            look_at: [0., 0., 10.],
            fov_y: 80f32.to_radians(),
            frame_per_rev: 500.,
        },
        0 => ViewStyle::Orthographic {
            pos: [0., 0., 100.],
            radius: 70.,
        },
        n if n as usize <= app.0.camera_views.len() => {
            let params = &app.0.camera_views[(n as usize) - 1];
            let look_at = [
                params.pos[0] + params.azimuth.sin(),
                params.pos[1] + params.azimuth.cos(),
                params.pos[2] - params.pitch.sin(),
            ];

            ViewStyle::Perspective {
                pos: params.pos,
                look_at,
                fov_y: 100f32.to_radians(),
            }
        }
        _ => return,
    };

    app.update_view_style(move |s| *s = new_style);
}

impl AppInner {
    pub async fn from_toml_cfg(
        p: impl AsRef<Path> + Send,
        proj_w: usize,
        proj_h: usize,
    ) -> stitch::Result<Self> {
        let cfg = stitch::proj::Config::open(&p)?;
        tracing::info!("opened config at {:?}", p.as_ref());

        let camera_views = cfg.cameras.iter().map(|c| c.view).collect();

        Ok(Self {
            stitcher: Sticher::from_cfg_gpu(cfg, proj_w, proj_h),
            camera_views,
        })
    }
}
