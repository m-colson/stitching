use app::App;
use clap::{Parser, Subcommand};

mod app;
mod util;

mod log;

#[tokio::main]
pub async fn main() {
    log::initialize(format!(
        "{}=debug,tower_http=debug",
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
            ArgCommand::Serve => {
                App::from_toml_cfg("live.toml", 1280, 720)
                    .await
                    .unwrap()
                    .listen_and_serve("0.0.0.0:2780")
                    .await
                    .unwrap();
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
    Serve,
    ListLive,
}
