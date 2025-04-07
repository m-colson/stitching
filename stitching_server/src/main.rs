use std::{net::Ipv4Addr, time::Duration};

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use util::Metrics;

mod app;
#[cfg(feature = "trt")]
mod infer_host;
mod log;
mod util;

#[tokio::main]
pub async fn main() {
    log::initialize(format!(
        "{}=debug,tower_http=debug,stitch=debug,smpgpu=debug,cam_loader=debug",
        env!("CARGO_CRATE_NAME")
    ));

    Args::parse().run().await.unwrap();
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: ArgCommand,
}

impl Args {
    /// # Errors
    /// errors can occur if the [`App`] cannot be loaded, or the server fails.
    pub async fn run(self) -> Result<()> {
        match self.cmd {
            ArgCommand::Serve {
                timeout,
                host,
                port,
            } => {
                let app = App::from_toml_cfg("live.toml", 1280, 720).await?;

                let monitoring_handle = tokio::spawn(async {
                    loop {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        Metrics::with(|m| tracing::info!("timing {}", m));
                        Metrics::reset();
                    }
                });

                let listen = format!("{}:{}", host, port);
                match timeout {
                    Some(n) => {
                        app.listen_and_serve_until(
                            listen,
                            tokio::time::sleep(Duration::from_secs(n)),
                        )
                        .await?;

                        Metrics::save_csv("metrics.csv")?;
                    }
                    None => app.listen_and_serve(listen).await?,
                };

                monitoring_handle.abort();
            }
            #[cfg(feature = "capture")]
            ArgCommand::CaptureLive => {
                use stitch::{camera, proj};
                let cfg = proj::Config::<cam_loader::Config>::open("live.toml")?;
                for (i, c) in cfg.cameras.into_iter().enumerate() {
                    let [width, height] = c.meta.resolution;
                    let mut buf = vec![0u8; (width * height * 4) as usize];
                    let c = c.load::<Vec<u8>>()?;
                    for _ in 0..10 {
                        let ticket = c.data.give(buf)?;
                        buf = ticket.block_take()?;
                    }
                    image::save_buffer(
                        format!("capture{i}.png"),
                        &buf,
                        width,
                        height,
                        image::ExtendedColorType::Rgba8,
                    )?;
                }
            }
            #[cfg(not(feature = "capture"))]
            ArgCommand::CaptureLive => {
                anyhow::bail!(
                    "this binary was not compiled with the \"capture\" feature enabled, which is required for this subcommand"
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Subcommand)]
pub enum ArgCommand {
    /// Serve the http server
    Serve {
        #[arg(short, long)]
        timeout: Option<u64>,
        #[arg(long, default_value = "0.0.0.0")]
        host: Ipv4Addr,
        #[arg(short, long, default_value_t = 2780)]
        port: u16,
    },
    /// Capture a raw image from every configured cameras and save them as capture*.png
    CaptureLive,
    // SimMasks,
}
