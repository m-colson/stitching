use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use futures_util::future::join_all;
use stitch::proj;
use util::Metrics;

mod app;
#[cfg(feature = "trt")]
mod infer_host;
mod log;
mod util;

#[tokio::main]
pub async fn main() {
    log::initialize(format!(
        "{}=debug,tower_http=debug,stitch=debug,smpgpu=debug,cam_loader=debug,tensorrt=info",
        env!("CARGO_CRATE_NAME")
    ));

    Args::parse().run().await.unwrap();
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "live.toml")]
    pub cfg_path: PathBuf,
    #[clap(subcommand)]
    pub cmd: ArgCommand,
}

impl Args {
    /// # Errors
    /// errors can occur if the [`App`] cannot be loaded, or the server fails.
    pub async fn run(self) -> Result<()> {
        match self.cmd {
            ArgCommand::Serve { timeout } => {
                let app = App::from_toml_cfg(self.cfg_path, 1280, 720).await?;

                let monitoring_handle = tokio::spawn(async {
                    loop {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        Metrics::with(|m| tracing::info!("timing {}", m));
                        if let Err(err) = Metrics::write_csv("metrics.csv") {
                            tracing::error!("error saving metrics: {err}");
                        }
                        Metrics::reset();
                    }
                });

                match timeout {
                    Some(n) => {
                        app.listen_and_serve_until(tokio::time::sleep(Duration::from_secs(n)))
                            .await?;
                    }
                    None => app.listen_and_serve().await?,
                };

                monitoring_handle.abort();
            }
            ArgCommand::CaptureLive { num_reads } => {
                let cfg = proj::Config::<cam_loader::Config>::open(self.cfg_path)?;

                let futs = cfg
                    .cameras
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| async move {
                        let [width, height] = c.meta.resolution;
                        let mut buf = vec![0u8; (width * height * 4) as usize];
                        let c = c.load::<Vec<u8>>().unwrap();
                        for _ in 0..num_reads {
                            let ticket = c.data.give(buf).unwrap();
                            buf = ticket.take().await.unwrap();
                        }
                        image::save_buffer(
                            format!("capture{i}.png"),
                            &buf,
                            width,
                            height,
                            image::ExtendedColorType::Rgba8,
                        )
                        .unwrap();
                    });

                _ = join_all(futs).await;
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
    },
    /// Capture a raw image from every configured cameras and save them as capture*.png
    CaptureLive {
        #[arg(short, default_value = "10")]
        num_reads: usize, // number of times to capture the image before saving it, allowing camera to stabilize settings.
    },
}
