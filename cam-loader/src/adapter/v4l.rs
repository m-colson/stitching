use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use v4l::{
    control::Value,
    io::{mmap::Stream, traits::CaptureStream},
    video::Capture,
};

use crate::{
    Error, Result,
    loader::{Loader, OwnedWriteBuffer},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub index: usize,
    #[serde(default)]
    pub controls: HashMap<String, i64>,
}

pub fn from_spec<B: OwnedWriteBuffer + 'static>(
    cam_spec: &super::Config,
    Config { index, controls }: Config,
) -> Result<Loader<B>> {
    const CHANS: u32 = 4;

    let device =
        v4l::Device::new(index).map_err(Error::io_ctx(format!("opening device {index}")))?;

    let mut params = device
        .params()
        .map_err(Error::io_ctx(format!("fetching camera {index} params")))?;
    params.interval = v4l::Fraction::new(1, 30);
    device
        .set_params(&params)
        .map_err(Error::io_ctx(format!("updating camera {index} params")))?;

    let ctrl_ids = device
        .query_controls()
        .map_err(Error::io_ctx(format!("fetching camera {index} controls")))?
        .into_iter()
        .map(|d| (d.name, d.id))
        .collect::<HashMap<_, _>>();

    let ctrls = controls
        .into_iter()
        .map(|(k, v)| {
            ctrl_ids
                .get(&k)
                .ok_or_else(|| {
                    Error::Other(format!("unknown control name {k:?} for camera {index}"))
                })
                .map(|&id| v4l::Control {
                    id,
                    value: Value::Integer(v),
                })
        })
        .collect::<Result<Vec<_>>>()?;

    if !ctrls.is_empty() {
        device
            .set_controls(ctrls)
            .map_err(Error::io_ctx(format!("updating camera {index} controls")))?;
    }

    let formats = device
        .enum_formats()
        .map_err(Error::io_ctx(format!("fetching camera {index} formats")))?;
    let frame_sizes = device
        .enum_framesizes(formats[0].fourcc)
        .map_err(Error::io_ctx(format!(
            "fetching camera {index} frame sizes"
        )))?;

    let (frame_width, frame_height, frame_format) = frame_sizes
        .into_iter()
        .filter_map(|s| {
            let size = s
                .size
                .to_discrete()
                .into_iter()
                .next()
                .expect("frame size had no sizes?");

            let [w, h] = cam_spec.resolution;
            (size.width == w && size.height == h).then(|| (size.width, size.height, s.fourcc))
        })
        .next()
        .ok_or_else(|| {
            Error::Other(format!(
                "camera {index} has no format of size {:?}",
                cam_spec.resolution
            ))
        })?;

    let format = v4l::Format::new(frame_width, frame_height, frame_format);

    device
        .set_format(&format)
        .map_err(Error::io_ctx(format!("updating camera {index} format")))?;

    let mut stream = Stream::with_buffers(&device, v4l::buffer::Type::VideoCapture, 2)
        .map_err(Error::io_ctx(format!("create camera {index} stream")))?;
    stream.set_timeout(std::time::Duration::from_millis(1000));

    stream.next().map_err(Error::io_ctx(format!(
        "performing camera {index} initialization"
    )))?;

    match &format.fourcc.repr {
        b"MJPG" => Ok(new_jpeg_loader(
            index,
            frame_width,
            frame_height,
            CHANS as _,
            stream,
        )),
        n => Err(Error::Other(format!(
            "unknown frame format for camera {}",
            std::str::from_utf8(n).unwrap()
        ))),
    }
}

#[inline]
fn new_jpeg_loader<B: OwnedWriteBuffer + 'static>(
    index: usize,
    width: u32,
    height: u32,
    chans: u32,
    mut stream: v4l::io::mmap::Stream<'static>,
) -> Loader<B> {
    Loader::new_blocking(width, height, chans, move |dest| {
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
                tracing::warn!("failed to read from camera {}: {err}", index);
            });
    })
}
