use std::path::PathBuf;

use nokhwa::{
    pixel_format::RgbAFormat,
    utils::{
        CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution,
    },
    FormatDecoder,
};

use serde::{Deserialize, Serialize};

use crate::{
    loader::{Loader, OwnedWriteBuffer},
    Error, Result,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub live_index: u32,
    pub mask_path: Option<PathBuf>,
    pub resolution: Option<[u32; 2]>,
    pub frame_rate: Option<u32>,
}

impl Config {
    #[must_use]
    #[inline]
    fn camera_format(&self) -> RequestedFormatType {
        match (self.resolution, self.frame_rate) {
            (Some([w, h]), Some(fr)) => RequestedFormatType::Closest(CameraFormat::new(
                Resolution::new(w, h),
                FrameFormat::MJPEG,
                fr,
            )),
            (Some([w, h]), None) => RequestedFormatType::HighestResolution(Resolution::new(w, h)),
            (None, Some(fr)) => RequestedFormatType::HighestFrameRate(fr),
            (None, None) => RequestedFormatType::AbsoluteHighestResolution,
        }
    }
}

impl<B: OwnedWriteBuffer + 'static> TryFrom<Config> for Loader<B> {
    type Error = Error;

    fn try_from(spec: Config) -> Result<Self> {
        type Format = RgbAFormat;
        const CHANS: u32 = 4;

        let live_index = spec.live_index;
        let mut raw = nokhwa::Camera::new(
            CameraIndex::Index(live_index),
            RequestedFormat::new::<Format>(spec.camera_format()),
        )?;

        raw.open_stream()?;
        let res = raw.resolution();
        let ff = raw.frame_format();

        Ok(Self::new_blocking(
            res.width(),
            res.height(),
            CHANS as _,
            move |buf| {
                _ = raw
                    .frame_raw()
                    .and_then(|raw_frame| Format::write_output_buffer(ff, res, &raw_frame, buf))
                    .inspect_err(|err| {
                        tracing::warn!("failed to read from camera {}: {err}", live_index);
                    });
            },
        ))
    }
}
