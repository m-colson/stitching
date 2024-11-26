use std::path::{Path, PathBuf};

use nokhwa::{
    pixel_format::RgbAFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    FormatDecoder,
};
use zerocopy::FromBytes;

use serde::{Deserialize, Serialize};

use crate::{
    loader::{FrameLoader, OwnedWriteBuffer},
    Result,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveSpec {
    pub live_index: u32,
    pub mask_path: Option<PathBuf>,
}

pub fn live_camera_loader<B: OwnedWriteBuffer + 'static>(
    spec: LiveSpec,
    requested: RequestedFormatType,
) -> Result<FrameLoader<B>> {
    type Format = RgbAFormat;
    const CHANS: u32 = 4;

    let mut raw = nokhwa::Camera::new(
        CameraIndex::Index(spec.live_index),
        RequestedFormat::new::<Format>(requested),
    )?;

    raw.open_stream()?;
    let res = raw.resolution();
    let ff = raw.frame_format();

    let masker = spec
        .mask_path
        .and_then(|p| {
            Masker::open(&p)
                .inspect_err(|err| tracing::error!("failed to load mask {:?}: {err}", p))
                .ok()
        })
        .unwrap_or_default();

    Ok(FrameLoader::new_blocking(
        res.width() as _,
        res.height() as _,
        CHANS as _,
        move |buf| {
            _ = raw
                .frame_raw()
                .and_then(|raw_frame| Format::write_output_buffer(ff, res, &raw_frame, buf))
                .inspect_err(|err| {
                    tracing::warn!("failed to read from camera {}: {err}", spec.live_index)
                });

            masker.apply_rgba(buf);
        },
    ))
}

#[derive(Clone, Default)]
struct Masker {
    data: Option<Box<[u8]>>,
}

impl Masker {
    #[inline]
    pub fn open(p: &Path) -> Result<Self> {
        let img = image::open(p)?.to_luma8();
        Ok(Self {
            data: Some(
                img.into_raw()
                    .into_iter()
                    .map(|p| if p >= 128 { 1 } else { 0 })
                    .collect::<Vec<_>>()
                    .into(),
            ),
        })
    }

    #[inline]
    pub fn apply_rgba(&self, dest: &mut [u8]) {
        let Some(data) = self.data.as_deref() else {
            return;
        };
        if data.len() != (dest.len() >> 2) {
            panic!("mask is the wrong size");
        };
        let dest = <[u32]>::mut_from_bytes(dest).unwrap();

        for (cond, v) in data.iter().zip(dest) {
            if *cond == 0 {
                *v = 0;
            }
        }
    }
}
