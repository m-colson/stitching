use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use notify::Watcher;
use serde::{Deserialize, Serialize};

use crate::{
    camera::{Camera, CameraError, CameraFov, ProjectionStyle},
    RenderState,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub proj: CameraConfig,
    pub cameras: Vec<CameraConfig>,
}

#[allow(dead_code)]
impl Config {
    pub fn open(p: impl AsRef<Path>) -> Result<Self, ConfigError> {
        Ok(toml::from_str(&std::fs::read_to_string(p)?)?)
    }

    pub fn load_state(&self) -> Result<RenderState, CameraError> {
        let cams = self
            .cameras
            .iter()
            .map(|c| c.load())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RenderState {
            proj: self.proj.load()?,
            cams,
        })
    }

    pub fn open_state(p: impl AsRef<Path>) -> Result<RenderState, ConfigError> {
        Ok(Self::open(p)?.load_state()?)
    }

    pub fn open_state_watch(
        p: impl AsRef<Path>,
    ) -> Result<(Arc<Mutex<RenderState>>, impl notify::Watcher), ConfigError> {
        let cams = Arc::new(Mutex::new(Self::open_state(p.as_ref())?));

        let watch_cams = cams.clone();
        let watch_p = p.as_ref().to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: Result<_, _>| {
            match res
                .map_err(ConfigError::WatchErr)
                .and_then(|_| Ok(Self::open(watch_p.clone())?.load_state()?))
            {
                Ok(cs) => {
                    println!("reloading");
                    *watch_cams.lock().unwrap() = cs;
                    println!("reload done");
                }
                Err(e) => println!("watch err {:?}", e),
            }
        })?;

        watcher.watch(p.as_ref(), notify::RecursiveMode::Recursive)?;

        Ok((cams, watcher))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraConfig {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub pitch: f32,
    pub azimuth: f32,
    #[serde(default)]
    pub roll: f32,
    pub fov: CameraFov,
    pub ty: CameraTypeConfig,
}

impl CameraConfig {
    pub fn load(&self) -> Result<Camera, CameraError> {
        let mut out = Camera::new(
            (self.x, self.y, self.z),
            self.pitch,
            self.azimuth,
            self.roll,
            self.fov,
        );

        match &self.ty {
            CameraTypeConfig::Image { path, mask_path } => {
                out.with_image(path, mask_path.as_deref())
            }
            CameraTypeConfig::Projection { style, avg_colors } => {
                out.project_settings(style.clone(), *avg_colors);
                Ok(out)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CameraTypeConfig {
    Image {
        path: PathBuf,
        mask_path: Option<PathBuf>,
    },
    Projection {
        style: ProjectionStyle,
        avg_colors: bool,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("read error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("decode error: {0}")]
    DecodeError(#[from] toml::de::Error),
    #[error("{0}")]
    CameraError(#[from] CameraError),
    #[error("watch err {0}")]
    WatchErr(#[from] notify::Error),
}
