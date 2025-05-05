//! This module contains top-level function and types for the application.

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
use serde::{Deserialize, Serialize};
use stitch::{
    camera::ViewParams,
    proj::{MaskLoaderConfig, ViewStyle},
};
use tokio::net::TcpListener;

use crate::{log, util::ws_upgrader};

pub mod proto;
pub mod stitcher;
pub mod video;

use stitcher::Stitcher;

/// Contains an instance of [`AppInner`] that can be shared between threads and server requests.
#[derive(Clone)]
pub struct App(Arc<AppInner>);

/// Contains the configuration and state of the application.
struct AppInner {
    pub stitcher: Stitcher,
    /// list of camera view configurations in the same order as the config file.
    pub camera_views: Vec<ViewParams>,
    pub server_cfg: ServerConfig,
}

#[derive(Serialize, Deserialize)]
struct AppConfig {
    pub server: ServerConfig,
    #[serde(flatten)]
    pub proj: stitch::proj::Config<MaskLoaderConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ServerConfig {
    /// the host name/address to listen on.
    pub host: String,
    /// the port to listen on.
    pub port: u16,
    /// the directory containing the frontend asset files.
    pub asset_dir: PathBuf,
}

impl App {
    /// Constructs a [`Router`] with `self` as the application state.
    pub fn into_router(self) -> Router {
        let router = Router::new()
            .fallback_service(tower_http::services::ServeDir::new(
                &self.0.server_cfg.asset_dir,
            ))
            .route("/video", get(ws_upgrader(video::conn_state_machine)))
            .route("/settings/view/{id}", put(set_view_handler));
        #[cfg(feature = "trt")]
        let router = router
            .route("/settings/min-iou/{id}", put(set_min_iou_handler))
            .route("/settings/min-score/{id}", put(set_min_score_handler));

        router.layer(log::http_trace_layer()).with_state(self)
    }

    /// Creates an app from a config file path and the primary view size.
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

    /// Creates and binds a [`TcpListener`] to the app's current [`ServerConfig`] hostname and port.
    async fn create_tcp_listener(&self) -> stitch::Result<TcpListener> {
        let server_cfg = &self.0.server_cfg;
        let addr = (&*server_cfg.host, server_cfg.port);

        let bind = TcpListener::bind(addr).await?;
        tracing::info!("listening on {}:{}", addr.0, addr.1);

        Ok(bind)
    }

    /// Serves the application on the app's current [`ServerConfig`] hostname and port
    /// and blocks until the server fails.
    pub async fn listen_and_serve(self) -> stitch::Result<()> {
        let bind = self.create_tcp_listener().await?;
        axum::serve(bind, self.into_router())
            .await
            .map_err(From::from)
    }

    /// Serves the application on the app's current [`ServerConfig`] hostname and port
    /// and exits when the `signal` future completes.
    pub async fn listen_and_serve_until(
        self,
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> stitch::Result<()> {
        let bind = self.create_tcp_listener().await?;
        axum::serve(bind, self.into_router())
            .with_graceful_shutdown(signal)
            .await
            .map_err(From::from)
    }

    /// Forwards message request to [`Stitcher::next_frame_msg`]
    async fn ws_frame(&self) -> Option<Message> {
        self.0.stitcher.next_frame_msg().await
    }

    /// Forwards update request to [`Stitcher::update_view_style`].
    pub fn update_view_style(&self, f: impl FnOnce(&mut ViewStyle) + Send + 'static) {
        self.0.stitcher.update_view_style(f);
    }

    /// Forwards set request to [`Stitcher::set_min_iou`].
    #[cfg(feature = "trt")]
    pub fn set_min_iou(&self, v: f32) {
        self.0.stitcher.set_min_iou(v);
    }

    /// Forwards set request to [`Stitcher::set_min_score`].
    #[cfg(feature = "trt")]
    pub fn set_min_score(&self, v: f32) {
        self.0.stitcher.set_min_score(v);
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

#[cfg(feature = "trt")]
async fn set_min_iou_handler(State(app): State<App>, p: extract::Path<i32>) {
    app.set_min_iou((p.0 as f32) / 100.);
}

#[cfg(feature = "trt")]
async fn set_min_score_handler(State(app): State<App>, p: extract::Path<i32>) {
    app.set_min_score((p.0 as f32) / 100.);
}

impl AppInner {
    /// Loads the [`AppConfig`] file at `p` and constructs the stitcher with [`Stitcher::from_cfg_gpu`].
    pub async fn from_toml_cfg(
        p: impl AsRef<Path> + Send,
        proj_w: usize,
        proj_h: usize,
    ) -> stitch::Result<Self> {
        let cfg = toml::from_str::<AppConfig>(&std::fs::read_to_string(&p)?)?;

        tracing::info!("opened config at {:?}", p.as_ref());

        let camera_views = cfg.proj.cameras.iter().map(|c| c.view).collect();

        Ok(Self {
            stitcher: Stitcher::from_cfg_gpu(cfg.proj, proj_w, proj_h),
            camera_views,
            server_cfg: cfg.server,
        })
    }
}
