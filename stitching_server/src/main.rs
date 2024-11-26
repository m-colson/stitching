use std::time::Duration;

use app::App;
use clap::{Parser, Subcommand};
use stitch::Config;

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
            ArgCommand::Serve { timeout } => {
                let app = App::from_toml_cfg("live.toml", 1280, 720, 1920, 1080)
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
            #[cfg(feature = "capture")]
            ArgCommand::CaptureLive => {
                let width = 1920;
                let height = 1080;

                let cfg = Config::open_live("live.toml").unwrap();
                let mut buf = vec![0u8; 1920 * 1080 * 4].into_boxed_slice();
                for (i, c) in cfg.cameras.into_iter().enumerate() {
                    let c = c.load::<Box<[u8]>>(width, height).unwrap();
                    let ticket = c.buf.give(buf);
                    buf = ticket.block_take();
                    image::save_buffer(
                        format!("capture{i}.png"),
                        &buf,
                        width,
                        height,
                        image::ExtendedColorType::Rgba8,
                    )
                    .unwrap();
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
    ListLive,
    #[cfg(feature = "capture")]
    CaptureLive,
}
