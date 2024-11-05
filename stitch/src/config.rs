use std::path::{Path, PathBuf};

use image::ImageDecoder;
use serde::{Deserialize, Serialize};

use crate::{
    camera::{Camera, CameraFov, ProjectionStyle},
    frame::{FrameBuffer, SizedFrameBuffer},
    RenderState,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub proj: CameraConfig,
    pub cameras: Vec<CameraConfig>,
}

#[allow(dead_code)]
impl Config {
    #[cfg(feature = "toml-cfg")]
    pub fn open(p: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let mut out = toml::from_str::<Self>(
            &std::fs::read_to_string(&p)
                .map_err(ConfigError::io_ctx(format!("reading {:?}", p.as_ref())))?,
        )?;

        let rel_base = p
            .as_ref()
            .canonicalize()
            .map_err(ConfigError::io_ctx(format!(
                "canonicalizing {:?}",
                p.as_ref()
            )))?;
        let rel_base = rel_base.parent().unwrap();

        out.proj.fix_paths(rel_base);
        for c in &mut out.cameras {
            c.fix_paths(rel_base);
        }

        Ok(out)
    }

    pub fn load_state<P: FrameBuffer + Default>(
        &self,
        proj_width: usize,
        proj_height: usize,
    ) -> Result<RenderState<P>, ConfigError> {
        let cams: Vec<Camera<SizedFrameBuffer>> = self
            .cameras
            .iter()
            .map(|c| c.clone().load_sized())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RenderState {
            proj: self
                .proj
                .clone()
                .with_dims(proj_width as f32, proj_height as f32)
                .load()?,
            cams,
        })
    }

    pub fn open_state<P: FrameBuffer + Default>(
        p: impl AsRef<Path>,
        proj_width: usize,
        proj_height: usize,
    ) -> Result<RenderState<P>, ConfigError> {
        Ok(Self::open(p)?.load_state(proj_width, proj_height)?)
    }

    #[cfg(feature = "watch")]
    pub fn open_state_watch<P: FrameBuffer + Default + 'static + std::marker::Send>(
        p: impl AsRef<Path>,
        proj_width: usize,
        proj_height: usize,
    ) -> Result<
        (
            std::sync::Arc<std::sync::Mutex<RenderState<P>>>,
            impl notify::Watcher,
        ),
        ConfigError,
    > {
        use notify::Watcher;
        use std::sync::{Arc, Mutex};

        let cams = Arc::new(Mutex::new(Self::open_state(
            p.as_ref(),
            proj_width,
            proj_height,
        )?));

        let watch_cams = cams.clone();
        let watch_p = p.as_ref().to_path_buf();
        let mut watcher = notify::recommended_watcher(move |res: Result<_, _>| {
            match res
                .map_err(ConfigError::WatchErr)
                .and_then(|_| Ok(Self::open(watch_p.clone())?.load_state(proj_width, proj_height)?))
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
    pub ty: CameraType,
}

impl CameraConfig {
    pub fn fix_paths(&mut self, rel_base: impl AsRef<Path>) {
        match &mut self.ty {
            CameraType::Image { path, mask_path } => {
                *path = rel_base.as_ref().join(&path);

                if let Some(mask_path) = mask_path {
                    *mask_path = rel_base.as_ref().join(&mask_path);
                }
            }
            CameraType::Projection { .. } => {}
        }
    }

    pub fn update_and_init_buf<B: FrameBuffer>(&mut self, buf: &mut B) -> Result<(), ConfigError> {
        match &self.ty {
            CameraType::Image { path, mask_path: _ } => {
                let dec = image::ImageReader::open(path)
                    .map_err(ConfigError::io_ctx(format!("opening {path:?}")))?
                    .into_decoder()?;
                buf.check_decoder(&dec)?;

                dec.read_image(buf.as_bytes_mut())?;
                self.set_dims(buf.width() as f32, buf.height() as f32);
            }
            CameraType::Projection { .. } => {}
        }

        Ok(())
    }

    pub fn load<B: FrameBuffer + Default>(self) -> Result<Camera<B>, ConfigError> {
        let mut out = Camera::<B>::new(self);
        out.cfg.update_and_init_buf(&mut out.buf).map(|_| out)
    }

    pub fn load_heaped<B: FrameBuffer>(mut self) -> Result<Camera<Box<B>>, ConfigError> {
        let mut uninit_buf = Box::<B>::new_uninit();
        self.update_and_init_buf(unsafe { uninit_buf.as_mut_ptr().as_mut().unwrap() })?;

        Ok(Camera {
            cfg: self,
            buf: unsafe { uninit_buf.assume_init() },
        })
    }

    pub fn load_sized(self) -> Result<Camera<SizedFrameBuffer>, ConfigError> {
        match &self.ty {
            CameraType::Image { path, mask_path: _ } => {
                let dec = image::ImageReader::open(path)
                    .map_err(ConfigError::io_ctx(format!("opening {path:?}")))?
                    .into_decoder()?;
                let (img_width, img_height) = dec.dimensions();
                let img_chans = dec.color_type().channel_count();

                let mut buf = SizedFrameBuffer::new(
                    img_width as usize,
                    img_height as usize,
                    img_chans as usize,
                );
                dec.read_image(buf.as_bytes_mut())?;

                Ok(Camera {
                    cfg: self.clone().with_dims(img_width as f32, img_height as f32),
                    buf,
                })
            }
            CameraType::Projection { .. } => Ok(Camera {
                cfg: self.clone(),
                buf: SizedFrameBuffer::default(),
            }),
        }
    }

    pub fn set_dims(&mut self, w: f32, h: f32) {
        self.fov = self.fov.with_aspect(w, h);
    }

    pub fn with_dims(mut self, w: f32, h: f32) -> Self {
        self.set_dims(w, h);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CameraType {
    Image {
        path: PathBuf,
        mask_path: Option<PathBuf>,
    },
    Projection {
        style: ProjectionStyle,
        avg_colors: bool,
    },
}

#[derive(thiserror::Error)]
pub enum ConfigError {
    #[error("io error while {1}: {0}")]
    IO(std::io::Error, String),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("image cast failed")]
    ImageCastFailure,

    #[error("{0}")]
    Dims(#[from] DimError),

    #[cfg(feature = "toml-cfg")]
    #[error("decode error: {0}")]
    DecodeError(#[from] toml::de::Error),

    #[cfg(feature = "watch")]
    #[error("watch err {0}")]
    WatchErr(#[from] notify::Error),
}

impl ConfigError {
    pub fn io_ctx(msg: String) -> impl FnOnce(std::io::Error) -> ConfigError {
        move |err| ConfigError::IO(err, msg)
    }
}

impl std::fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("{kind} mismatch: {exp} != {got}")]
pub struct DimError {
    pub kind: DimErrorKind,
    pub exp: usize,
    pub got: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum DimErrorKind {
    Width,
    Height,
    Channel,
}

impl std::fmt::Display for DimErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Width => write!(f, "width"),
            Self::Height => write!(f, "height"),
            Self::Channel => write!(f, "channel"),
        }
    }
}

impl DimErrorKind {
    pub fn err(self, exp: usize, got: usize) -> DimError {
        DimError {
            kind: self,
            exp,
            got,
        }
    }

    pub fn check(self, exp: usize, got: usize) -> Result<(), DimError> {
        (exp == got).then_some(()).ok_or(self.err(exp, got))
    }
}
