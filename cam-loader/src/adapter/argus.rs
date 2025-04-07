use std::{
    sync::{Arc, LazyLock, Mutex, Weak},
    time::Duration,
};

use argus::{
    AeMode, AwbMode, CaptureIntent, FrameConsumer, NvBufSurfaceColorFormat, NvBufSurfaceLayout,
    PixelFormat, PixelFormatType, StreamType,
};
use serde::{Deserialize, Serialize};

use crate::{
    Error, Result,
    loader::{Loader, OwnedWriteBuffer},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub index: usize,
    pub mode: usize,
}

fn cam_provider() -> Arc<argus::CameraProvider> {
    static BACKING: LazyLock<Mutex<Weak<argus::CameraProvider>>> =
        LazyLock::new(|| Mutex::new(Weak::new()));

    let mut locked = BACKING.lock().unwrap();

    if let Some(p) = locked.upgrade() {
        return p;
    }

    let out = Arc::new(argus::CameraProvider::new().unwrap());
    *locked = Arc::downgrade(&out);
    out
}

pub fn from_spec<B: OwnedWriteBuffer + Send + 'static>(
    cam_spec: &super::Config,
    Config { index, mode }: Config,
) -> Loader<B> {
    let [w, h] = cam_spec.resolution;
    let fps = cam_spec.frame_rate.unwrap_or(30) as u64;

    let handler = move |req_recv: kanal::Receiver<(B, kanal::Sender<B>)>| -> Result<()> {
        let provider = cam_provider();
        let iprovider = provider.as_interface();

        let devices = iprovider.get_camera_devices()?;
        let Some(&device) = devices.get(index) else {
            return Err(Error::Other(format!("device {index} does not exist")));
        };

        let modes = device.as_properties().get_all_sensor_modes()?;
        let Some(&mode) = modes.get(mode) else {
            return Err(Error::Other(format!(
                "mode {mode} does not exist for device {index}"
            )));
        };

        let mut cap_session = iprovider.create_capture_session(device)?;
        let mut isession = cap_session.as_interface();

        let mut out_settings = isession.create_output_stream_settings(StreamType::EGL)?;
        let mut egl_out_settings = out_settings.as_egl_interface();
        egl_out_settings.set_pixel_format(PixelFormat::YCBCR_420_888)?;
        egl_out_settings.set_resolution(mode.as_interface().get_resolution())?;

        let out_stream = isession.create_output_stream(&out_settings)?;

        let mut frame_consumer = FrameConsumer::from_stream(&out_stream)?;
        let mut iframe_consumer = frame_consumer.as_interface();

        let mut request = isession.create_request(CaptureIntent::VIDEO_RECORD)?;
        let mut irequest = request.as_interface();
        irequest.set_pixel_format_type(PixelFormatType::YUV_ONLY)?;
        irequest.enable_output_stream(&out_stream)?;

        let mut ac_settings = irequest.get_ac_settings();
        ac_settings.set_ae_mode(AeMode::ON)?;
        ac_settings.set_awb_mode(AwbMode::AUTO)?;
        ac_settings.set_awb_lock(false)?;
        ac_settings.set_color_saturation_bias(1.)?;

        let mut src_settings = request.as_source_settings();
        src_settings.set_sensor_mode(mode)?;
        let frame_dur = 1e9 as u64 / fps;
        src_settings.set_frame_duration_range([frame_dur, frame_dur])?;

        isession.repeat(&request)?;
        loop {
            match req_recv.recv() {
                Ok((mut req, resp_send)) => {
                    if let Some(mut v) = req.owned_to_view() {
                        iframe_consumer
                            .acquire_frame(Duration::from_secs(1))
                            .and_then(|mut frame| {
                                let mut iframe = frame.as_interface();
                                let mut img = iframe.get_image();

                                let ibuf = img.as_native_buffer();
                                let buf = ibuf.create_nv_buf(
                                    (w, h),
                                    NvBufSurfaceColorFormat::NVBUF_COLOR_FORMAT_RGBA,
                                    NvBufSurfaceLayout::NVBUF_LAYOUT_PITCH,
                                    0,
                                )?;

                                buf.copy_raw((w, h), v.as_mut());

                                Ok(())
                            })
                            .unwrap_or_else(|err| {
                                tracing::warn!("failed to read from argus {index} : {err:?}")
                            })
                    } else {
                        tracing::warn!("attempted to copy zero bytes, ignoring...");
                    }

                    // if the receiver has been dropped, they don't want their buffer back!
                    _ = resp_send.send(req);
                }
                Err(err) => {
                    match err {
                        kanal::ReceiveError::SendClosed => {
                            tracing::warn!("argus loader exiting because all senders have dropped")
                        }
                        kanal::ReceiveError::Closed => {
                            tracing::warn!("argus loader exiting bacause it was closed")
                        }
                    }
                    break;
                }
            }
        }

        isession.stop_repeat();
        isession.wait_for_idle()?;

        Ok(())
    };

    Loader::<B>::new_blocking_manual_recv(w, h, 4, move |req_recv| {
        handler(req_recv).unwrap_or_else(|err| tracing::error!("argus loader failed: {err:?}"))
    })
}
