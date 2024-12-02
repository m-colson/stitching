use std::time::Duration;

use anyhow::{anyhow, Result};
use app::App;
use clap::{Parser, Subcommand};
use util::Metrics;

mod app;
mod util;

mod log;

#[tokio::main]
pub async fn main() {
    log::initialize(format!(
        "{}=debug,tower_http=debug,stitch=debug,smpgpu=debug",
        env!("CARGO_CRATE_NAME")
    ));

    Args::try_parse().unwrap().run().await.unwrap();
}

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: ArgCommand,
}

impl Args {
    /// # Errors
    /// errors can occur if the [App] cannot be loaded, or the server fails.
    pub async fn run(self) -> Result<()> {
        match self.cmd {
            ArgCommand::Serve { timeout } => {
                let app = App::from_toml_cfg("live.toml", 1280, 720).await?;

                match timeout {
                    Some(n) => {
                        app.listen_and_serve_until(
                            "0.0.0.0:2780",
                            tokio::time::sleep(Duration::from_secs(n)),
                        )
                        .await?;

                        Metrics::save_csv("metrics.csv")?;
                    }
                    None => app.listen_and_serve("0.0.0.0:2780").await?,
                };
            }
            ArgCommand::ListLive => {
                let cams = nokhwa::query(
                    nokhwa::native_api_backend()
                        .ok_or_else(|| anyhow!("no camera backend found"))?,
                )?;
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

                let cfg = stitch::proj::Config::open("live.toml")?;
                let mut buf = vec![0u8; (width * height * 4) as usize].into_boxed_slice();
                for (i, c) in cfg.cameras.into_iter().enumerate() {
                    let c = c.load::<Box<[u8]>>()?;
                    let ticket = c.data.give(buf)?;
                    buf = ticket.block_take()?;
                    image::save_buffer(
                        format!("capture{i}.png"),
                        &buf,
                        width,
                        height,
                        image::ExtendedColorType::Rgba8,
                    )?;
                }
            }
        }
        Ok(())
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
