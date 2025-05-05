//! This crate contains the functionality for the server that interfaces with [`stitch`].
#![warn(missing_docs)]

use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use futures_util::future::join_all;
use stitch::proj::{self, MaskLoaderConfig};
use util::Metrics;

pub mod app;
#[cfg(feature = "trt")]
pub mod infer_host;
pub mod log;
pub mod util;

#[tokio::main]
/// Entrypoint to the binary. Initializes logging, parses arguments and calls [`Args::run`].
pub async fn main() {
    log::initialize(format!(
        "{}=debug,tower_http=debug,stitch=debug,smpgpu=debug,cam_loader=debug,tensorrt=info",
        env!("CARGO_CRATE_NAME")
    ));

    Args::parse().run().await.unwrap();
}

/// Represents arguments that could be passed to the cli.
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// path to the config file.
    #[arg(short, long, default_value = "live.toml")]
    pub cfg_path: PathBuf,
    /// cli subcommand.
    #[clap(subcommand)]
    pub cmd: ArgCommand,
}

impl Args {
    /// Runs the subcommand that was parsed.
    pub async fn run(self) -> Result<()> {
        match self.cmd {
            ArgCommand::Serve {
                timeout,
                log_delta,
                metrics_file,
            } => {
                let app = App::from_toml_cfg(self.cfg_path, 1280, 720).await?;

                let monitoring_handle = (log_delta >= 0).then(|| {
                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(Duration::from_secs(log_delta as _)).await;
                            Metrics::with(|m| tracing::info!("timing {}", m));
                            if let Some(p) = &metrics_file {
                                if let Err(err) = Metrics::write_csv(p) {
                                    tracing::error!("error saving metrics: {err}");
                                }
                            }
                            Metrics::reset();
                        }
                    })
                });

                match timeout {
                    Some(n) => {
                        app.listen_and_serve_until(tokio::time::sleep(Duration::from_secs(n)))
                            .await?;
                    }
                    None => app.listen_and_serve().await?,
                };

                if let Some(h) = monitoring_handle {
                    h.abort();
                }
            }
            ArgCommand::CaptureLive { num_reads } => {
                let cfg = proj::Config::<MaskLoaderConfig>::open(self.cfg_path)?;

                let futs = cfg
                    .cameras
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| async move {
                        let [width, height] = c.meta.loader.resolution;
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

/// Represents cli subcommands.
#[derive(Clone, Debug, Subcommand)]
pub enum ArgCommand {
    /// Serve the http server
    Serve {
        /// how long to run the server for in seconds
        #[arg(short, long)]
        timeout: Option<u64>,
        /// how often to log the server metrics in seconds
        #[arg(short = 'd', long, default_value = "10")]
        log_delta: i64,
        /// path to save the metrics csv file.
        #[arg(short = 'm', long)]
        metrics_file: Option<String>,
    },
    /// Capture a raw image from every configured cameras and save them as capture*.png
    CaptureLive {
        /// number of times to capture the image before saving it, allowing camera to stabilize settings.
        #[arg(short, default_value = "10")]
        num_reads: usize,
    },
}
