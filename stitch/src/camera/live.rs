use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType, Resolution},
};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, MutexGuard};
use zerocopy::FromZeros;

use crate::{
    frame::{FrameBufferable, ToFrameBufferAsync},
    FrameBuffer, Result,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LiveSpec {
    pub live_index: u32,
}

type RawLiveCam = nokhwa::Camera;

pub struct LiveBuffer {
    raw: Mutex<RawLiveCam>,
    data: Mutex<Box<[u8]>>,
}

impl LiveBuffer {
    pub fn new(camera_index: u32) -> Result<Self> {
        let width = 1280;
        let height = 720;

        let mut raw = RawLiveCam::new(
            CameraIndex::Index(camera_index),
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::HighestResolution(
                Resolution::new(width, height),
            )),
        )?;

        raw.open_stream()?;
        let res: Resolution = raw.resolution();
        let data =
            <[u8]>::new_box_zeroed_with_elems((res.width() * res.height() * 3) as usize).unwrap();

        Ok(Self {
            raw: Mutex::new(raw),
            data: Mutex::new(data),
        })
    }

    pub async fn cam_info(&self) -> (usize, usize, usize) {
        let locked = self.raw.lock().await;
        let res = locked.resolution();
        (res.width() as _, res.height() as _, 3)
    }
}

impl FrameBufferable for LiveBuffer {}

impl<'a> ToFrameBufferAsync<'a> for LiveBuffer {
    type Output = LiveBufferGaurd<'a>;

    async fn to_frame_async(&'a self) -> Self::Output {
        let mut raw = self.raw.lock().await;
        let mut data = self.data.lock().await;
        raw.write_frame_to_buffer::<RgbFormat>(&mut data).unwrap();

        LiveBufferGaurd { raw, data }
    }
}

pub struct LiveBufferGaurd<'a> {
    raw: MutexGuard<'a, RawLiveCam>,
    data: MutexGuard<'a, Box<[u8]>>,
}

impl<'a> FrameBufferable for LiveBufferGaurd<'a> {}

impl<'a> FrameBuffer for LiveBufferGaurd<'a> {
    fn width(&self) -> usize {
        self.raw.resolution().width() as _
    }

    fn height(&self) -> usize {
        self.raw.resolution().height() as _
    }

    fn chans(&self) -> usize {
        3
    }

    fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}
