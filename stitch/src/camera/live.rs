use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    loader::{Loader, OwnedWriteBuffer},
    Error, Result,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub live_index: usize,
    pub mask_path: Option<PathBuf>,
    pub resolution: Option<[u32; 2]>,
    pub frame_rate: Option<u32>,
}

// impl Config {
//     #[must_use]
//     #[inline]
//     fn camera_format(&self) -> RequestedFormatType {
//         match (self.resolution, self.frame_rate) {
//             (Some([w, h]), Some(fr)) => RequestedFormatType::Closest(CameraFormat::new(
//                 Resolution::new(w, h),
//                 FrameFormat::MJPEG,
//                 fr,
//             )),
//             (Some([w, h]), None) => RequestedFormatType::HighestResolution(Resolution::new(w, h)),
//             (None, Some(fr)) => RequestedFormatType::HighestFrameRate(fr),
//             (None, None) => RequestedFormatType::AbsoluteHighestResolution,
//         }
//     }
// }

#[cfg(feature = "v4l")]
impl<B: OwnedWriteBuffer + 'static> TryFrom<Config> for Loader<B> {
    type Error = Error;

    fn try_from(spec: Config) -> Result<Self> {
        use v4l::{
            io::{mmap::Stream, traits::CaptureStream},
            video::Capture,
        };
        const CHANS: u32 = 4;

        let live_index = spec.live_index;
        let device = v4l::Device::new(live_index)
            .map_err(Error::io_ctx(format!("opening device {live_index}")))?;

        // let formats = device.enum_formats().map_err(Error::io_ctx(format!(
        //     "fetching camera {live_index} formats"
        // )))?;

        // device
        //     .set_format(&formats.first().ok_or_else(|| {
        //         Error::Other(format!("camera {live_index} has no known valid formats"))
        //     })?)
        //     .map_err(Error::io_ctx(format!(
        //         "updating camera {live_index} format"
        //     )))?;

        let mut params = device.params().map_err(Error::io_ctx(format!(
            "fetching camera {live_index} params"
        )))?;
        params.interval = v4l::Fraction::new(1, 30);
        device.set_params(&params).map_err(Error::io_ctx(format!(
            "updating camera {live_index} params"
        )))?;

        let mut stream = Stream::with_buffers(&device, v4l::buffer::Type::VideoCapture, 2)
            .map_err(Error::io_ctx(format!("create camera {live_index} stream")))?;
        stream.set_timeout(std::time::Duration::from_millis(1000));

        stream.next().map_err(Error::io_ctx(format!(
            "performing camera {live_index} initialization"
        )))?;

        let width = 1920;
        let height = 1080;

        Ok(Self::new_blocking(width, height, CHANS as _, move |dest| {
            stream
                .next()
                .map_err(Error::io_ctx("loading next frame"))
                .and_then(|(raw_frame, _)| {
                    mozjpeg::Decompress::new_mem(raw_frame)
                        .and_then(|d| d.rgba())
                        .map_err(Error::io_ctx("decompressing image"))
                })
                .and_then(|mut decomp| {
                    if dest.len() != decomp.min_flat_buffer_size() {
                        return Err(Error::Other("bad decoded buffer size".to_string()));
                    }

                    decomp
                        .read_scanlines_into::<u8>(dest)
                        .map_err(Error::io_ctx("reading scanlines"))?;

                    decomp.finish().map_err(Error::io_ctx("finishing decode"))?;

                    Ok(())
                })
                .unwrap_or_else(|err| {
                    tracing::warn!("failed to read from camera {}: {err}", live_index);
                });
        }))
    }
}

#[cfg(not(feature = "v4l"))]
impl<B: OwnedWriteBuffer + 'static> TryFrom<Config> for Loader<B> {
    type Error = Error;

    fn try_from(_spec: Config) -> Result<Self> {
        unimplemented!("non-linux video loading")
    }
}
