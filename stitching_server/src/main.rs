use std::{fmt::Debug, path::Path, sync::Arc};

use stitch::{ConfigError, RenderState, StaticFrameBuffer};
use tokio::net::{TcpListener, ToSocketAddrs};

mod routes;
mod util;

mod log;

#[tokio::main]
pub async fn main() {
    log::initialize(format!(
        "{}=debug,tower_http=debug",
        env!("CARGO_CRATE_NAME")
    ));

    AppState::<1080, 720>::from_toml_cfg("cams.toml")
        .unwrap()
        .listen_and_serve("0.0.0.0:2780")
        .await
        .unwrap();
}

#[derive(Clone, Debug)]
pub struct AppState<const W: usize, const H: usize> {
    pub rend: Arc<RenderState<Box<StaticFrameBuffer<W, H>>>>,
}

impl<const W: usize, const H: usize> AppState<W, H> {
    pub fn from_toml_cfg(p: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let cfg = stitch::Config::open(p)?;

        let proj = cfg.proj.load_heaped()?;

        let cams = cfg
            .cameras
            .iter()
            .map(|c| c.clone().load_sized())
            .collect::<Result<Vec<_>, _>>()?;

        // SAFETY: all fields of `RenderState` are initialized if we get here.
        Ok(Self {
            rend: Arc::new(RenderState { proj, cams }),
        })
    }

    pub async fn listen_and_serve(self, a: impl ToSocketAddrs + Debug) -> std::io::Result<()> {
        let bind = TcpListener::bind(&a).await?;
        tracing::info!("listening on {a:?}");

        axum::serve(bind, routes::router(self)).await
    }
}
