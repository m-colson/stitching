use std::path::{Path, PathBuf};

use image::ImageDecoder;
use serde::{Deserialize, Serialize};

use crate::{
    frame::{check_against_decoder, FrameBufferMut},
    Error, Result,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageSpec {
    pub path: PathBuf,
    pub mask_path: Option<PathBuf>,
}

impl ImageSpec {
    pub fn fix_paths(&mut self, rel_base: impl AsRef<Path>) {
        self.path = rel_base.as_ref().join(&self.path);

        if let Some(mask_path) = &mut self.mask_path {
            *mask_path = rel_base.as_ref().join(&mask_path);
        }
    }

    pub fn load_into<B: FrameBufferMut>(&self, buf: &mut B) -> Result<()> {
        let path = &self.path;

        let dec = image::ImageReader::open(path)
            .map_err(Error::io_ctx(format!("opening {path:?}")))?
            .into_decoder()?;
        check_against_decoder(buf, &dec)?;

        dec.read_image(buf.as_bytes_mut())?;
        Ok(())
    }
}
