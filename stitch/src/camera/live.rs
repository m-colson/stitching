use nokhwa::{
    pixel_format::RgbAFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    FormatDecoder,
};

use serde::{Deserialize, Serialize};

use crate::{
    loader::{FrameLoader, OwnedWriteBuffer},
    Result,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LiveSpec {
    pub live_index: u32,
}

pub fn live_camera_loader<B: OwnedWriteBuffer + 'static>(
    camera_index: u32,
    requested: RequestedFormatType,
) -> Result<FrameLoader<B>> {
    type Format = RgbAFormat;
    const CHANS: u32 = 4;

    let mut raw = nokhwa::Camera::new(
        CameraIndex::Index(camera_index),
        RequestedFormat::new::<Format>(requested),
    )?;

    raw.open_stream()?;
    let res = raw.resolution();
    let ff = raw.frame_format();

    Ok(FrameLoader::new_blocking(
        res.width() as _,
        res.height() as _,
        CHANS as _,
        move |buf| {
            let raw_frame = raw.frame_raw().unwrap();
            Format::write_output_buffer(ff, res, &raw_frame, buf).unwrap();
        },
    ))
}
