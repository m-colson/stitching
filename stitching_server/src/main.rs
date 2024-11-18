use std::time::Duration;

use app::App;
use clap::{Parser, Subcommand};

mod app;
mod util;

mod log;

#[tokio::main]
pub async fn main() {
    log::initialize(format!(
        "{}=debug,tower_http=debug,stitch=debug,smpgpu=debug",
        env!("CARGO_CRATE_NAME")
    ));

    Args::parse().run().await
}

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: ArgCommand,
}

impl Args {
    pub async fn run(self) {
        match self.cmd {
            ArgCommand::Serve { timeout: _ } => {
                App::from_toml_cfg("live.toml", 1280, 720)
                    .await
                    .unwrap()
                    .listen_and_serve("0.0.0.0:2780")
                    .await
                    .unwrap();
            }
            ArgCommand::ServeGpu { timeout } => {
                let app = App::from_toml_cfg_gpu("live.toml", 1280, 720, 1280, 720)
                    .await
                    .unwrap();

                match timeout {
                    Some(n) => app
                        .listen_and_serve_until(
                            "0.0.0.0:2780",
                            tokio::time::sleep(Duration::from_secs(n)),
                        )
                        .await
                        .unwrap(),
                    None => app.listen_and_serve("0.0.0.0:2780").await.unwrap(),
                };
            }
            ArgCommand::ListLive => {
                let cams = nokhwa::query(nokhwa::native_api_backend().unwrap()).unwrap();
                for c in cams {
                    println!(
                        "{} -> {:?} ({:?})",
                        c.index(),
                        c.human_name(),
                        c.description()
                    );
                }
            }
        }
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum ArgCommand {
    Serve {
        #[arg(short, long)]
        timeout: Option<u64>,
    },
    ServeGpu {
        #[arg(short, long)]
        timeout: Option<u64>,
    },
    ListLive,
}
