use std::path::Path;

use image::ImageDecoder;
use nokhwa::utils::Resolution;
use serde::{Deserialize, Serialize};

use crate::{
    camera::{Camera, CameraSpec, ImageSpec},
    frame::{FrameBuffer, FrameSize, SizedFrameBuffer},
    loader::{FrameLoader, OwnedWriteBuffer},
    proj::ProjSpec,
    Error, FrameBufferMut, RenderState, Result,
};

#[cfg(feature = "live")]
use crate::camera::live::LiveSpec;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config<Cam = ImageSpec> {
    pub proj: CameraConfig<ProjSpec>,
    pub cameras: Vec<CameraConfig<Cam>>,
}

#[allow(dead_code)]
impl Config {
    #[cfg(feature = "toml-cfg")]
    pub fn open(p: impl AsRef<Path>) -> Result<Self> {
        use crate::Error;

        let mut out = toml::from_str::<Self>(
            &std::fs::read_to_string(&p)
                .map_err(Error::io_ctx(format!("reading {:?}", p.as_ref())))?,
        )?;

        let rel_base = p
            .as_ref()
            .canonicalize()
            .map_err(Error::io_ctx(format!("canonicalizing {:?}", p.as_ref())))?;
        let rel_base = rel_base.parent().unwrap();

        for c in &mut out.cameras {
            c.meta.fix_paths(rel_base);
        }

        Ok(out)
    }

    pub fn load_state<P: FrameBuffer + Default>(
        &self,
        proj_width: usize,
        proj_height: usize,
    ) -> Result<RenderState<P>> {
        let cams = self
            .cameras
            .iter()
            .map(|c| c.clone().load_sized())
            .collect::<Result<Vec<_>>>()?;

        Ok(RenderState {
            proj: self
                .proj
                .with_dims(proj_width as f32, proj_height as f32)
                .into(),
            cams,
        })
    }

    #[cfg(feature = "toml-cfg")]
    pub fn open_state<P: FrameBuffer + Default>(
        p: impl AsRef<Path>,
        proj_width: usize,
        proj_height: usize,
    ) -> Result<RenderState<P>> {
        Self::open(p)?.load_state(proj_width, proj_height)
    }

    #[cfg(feature = "watch")]
    pub fn open_state_watch<P: FrameBuffer + Default + 'static + std::marker::Send>(
        p: impl AsRef<Path>,
        proj_width: usize,
        proj_height: usize,
    ) -> Result<(
        std::sync::Arc<std::sync::Mutex<RenderState<P>>>,
        impl notify::Watcher,
    )> {
        use notify::Watcher;
        use std::sync::{Arc, Mutex};

        let cams = Arc::new(Mutex::new(Self::open_state(
            p.as_ref(),
            proj_width,
            proj_height,
        )?));

        let watch_cams = cams.clone();
        let watch_p = p.as_ref().to_path_buf();
        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<_, _>| {
            match res
                .map_err(Error::WatchErr)
                .and_then(|_| Self::open(watch_p.clone())?.load_state(proj_width, proj_height))
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

#[cfg(feature = "live")]
impl Config<LiveSpec> {
    #[cfg(feature = "toml-cfg")]
    pub fn open_live(p: impl AsRef<Path>) -> Result<Self> {
        toml::from_str::<Self>(
            &std::fs::read_to_string(&p)
                .map_err(Error::io_ctx(format!("reading {:?}", p.as_ref())))?,
        )
        .map_err(From::from)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CameraConfig<K> {
    #[serde(flatten)]
    pub spec: CameraSpec,
    #[serde(flatten)]
    pub meta: K,
}

impl<K> CameraConfig<K> {
    pub fn with_dims(mut self, w: f32, h: f32) -> Self {
        self.spec = self.spec.with_dims(w, h);
        self
    }

    pub fn with_buffer<B>(self, buf: B) -> Camera<B, K> {
        Camera::new(self.spec, self.meta, buf)
    }
}

impl CameraConfig<ImageSpec> {
    pub fn load<B: FrameBufferMut + Default>(self) -> Result<Camera<B, ImageSpec>> {
        let mut out = Camera::from(self);
        out.meta.load_into(&mut out.buf)?;
        out.spec.set_dims(out.width() as f32, out.height() as f32);
        Ok(out)
    }

    pub fn load_with<B: FrameBufferMut>(self, buf: B) -> Result<Camera<B, ImageSpec>> {
        let mut out = Camera::new(self.spec, self.meta, buf);
        out.meta.load_into(&mut out.buf)?;
        out.spec.set_dims(out.width() as f32, out.height() as f32);
        Ok(out)
    }

    // pub fn load_heaped<B: FrameBufferMut>(self) -> Result<Camera<Box<B>, ImageSpec>> {
    //     let mut uninit_buf = Box::<B>::new_uninit();
    //     self.meta
    //         .load_into(unsafe { uninit_buf.as_mut_ptr().as_mut().unwrap() })?;
    //     let buf = unsafe { uninit_buf.assume_init() };

    //     Ok(Camera::new(
    //         self.spec.with_dims(buf.width() as f32, buf.height() as f32),
    //         self.meta,
    //         buf,
    //     ))
    // }

    pub fn load_sized(self) -> Result<Camera<SizedFrameBuffer, ImageSpec>> {
        let path = &self.meta.path;
        let dec = image::ImageReader::open(path)
            .map_err(Error::io_ctx(format!("opening {path:?}")))?
            .into_decoder()?;
        let (img_width, img_height) = dec.dimensions();
        let img_chans = dec.color_type().channel_count();

        let mut buf =
            SizedFrameBuffer::new(img_width as usize, img_height as usize, img_chans as usize);
        dec.read_image(buf.as_bytes_mut())?;

        Ok(Camera::new(
            self.spec.with_dims(img_width as f32, img_height as f32),
            self.meta,
            buf,
        ))
    }
}

#[cfg(feature = "live")]
impl CameraConfig<LiveSpec> {
    pub fn load<B: OwnedWriteBuffer + 'static>(
        self,
        target_x: u32,
        target_y: u32,
    ) -> Result<Camera<FrameLoader<B>>> {
        let buf = crate::camera::live_camera_loader(
            self.meta,
            nokhwa::utils::RequestedFormatType::HighestResolution(Resolution::new(
                target_x, target_y,
            )),
        )?;
        let (w, h, _) = buf.frame_size();

        Ok(Camera::new(
            self.spec.with_dims(w as f32, h as f32),
            (),
            buf,
        ))
    }
}
