use std::{ffi, fmt::Debug, marker::PhantomData, time::Duration};

use argus_sys::{BayerTuple, Range, Status};
pub use argus_sys::{NvBufSurfaceColorFormat, NvBufSurfaceLayout};
use cpp_interop::container::CppVector;

mod uuid_enums;

pub use uuid_enums::*;

#[derive(Debug)]
pub struct Error(Status);

impl From<Status> for Error {
    fn from(value: Status) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "argus error: {:?}", self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

// fn status_result(s: argus_sys::Status) -> Result<()> {
//     match s {
//         argus_sys::Status::STATUS_OK => Ok(()),
//         err => Err(Error(err)),
//     }
// }

pub struct CameraProvider(*mut argus_sys::CameraProvider);

unsafe impl Send for CameraProvider {}
unsafe impl Sync for CameraProvider {}

impl CameraProvider {
    pub fn new() -> Result<Self> {
        let mut status = Status::STATUS_OK;
        let raw = unsafe { argus_sys::CameraProvider::create(&mut status) };
        status.ok()?;

        Ok(Self(raw))
    }

    pub fn as_interface(&self) -> ICameraProvider<'_> {
        ICameraProvider(unsafe { (*self.0).as_interface() }, PhantomData)
    }
}

impl Drop for CameraProvider {
    fn drop(&mut self) {
        unsafe { (*self.0)._base_1.destroy() };
    }
}

pub struct ICameraProvider<'a>(
    *mut argus_sys::ICameraProvider,
    PhantomData<&'a CameraProvider>,
);

impl<'a> ICameraProvider<'a> {
    pub fn get_camera_devices(&self) -> Result<Vec<CameraDevice>> {
        let mut out_vec = CppVector::new();
        unsafe { self.0.as_ref().unwrap().get_camera_devices(&mut out_vec) }.ok()?;
        Ok(out_vec
            .into_iter()
            .map(|p| unsafe { CameraDevice(*p) })
            .collect())
    }

    pub fn get_version(&self) -> ffi::CString {
        let version = unsafe { (*self.0.as_ref().unwrap().get_version()).c_str() };
        version.to_owned()
    }

    pub fn create_capture_session(&self, dev: CameraDevice) -> Result<CaptureSession<'a>> {
        let mut status = Status::STATUS_OK;
        let out = unsafe { (*self.0).create_capture_session(dev.0, &mut status) };
        status.ok()?;
        Ok(CaptureSession(out, PhantomData))
    }
}

#[derive(Clone, Copy)]
pub struct CameraDevice(*const argus_sys::CameraDevice);

impl CameraDevice {
    pub fn as_properties(&self) -> ICameraProperties<'_> {
        ICameraProperties(
            unsafe { self.0.as_ref().unwrap().as_properties() },
            PhantomData,
        )
    }
}

pub struct ICameraProperties<'a>(
    *const argus_sys::ICameraProperties,
    PhantomData<&'a CameraDevice>,
);

impl ICameraProperties<'_> {
    pub fn get_all_sensor_modes(&self) -> Result<Vec<SensorMode>> {
        let mut out_vec = CppVector::new();
        unsafe { self.0.as_ref().unwrap().get_all_sensor_modes(&mut out_vec) }.ok()?;
        Ok(out_vec
            .into_iter()
            .map(|p| unsafe { SensorMode(*p) })
            .collect())
    }

    pub fn get_basic_sensor_modes(&self) -> Result<Vec<SensorMode>> {
        let mut out_vec = CppVector::new();
        unsafe {
            self.0
                .as_ref()
                .unwrap()
                .get_basic_sensor_modes(&mut out_vec)
        }
        .ok()?;
        Ok(out_vec
            .into_iter()
            .map(|p| unsafe { SensorMode(*p) })
            .collect())
    }

    pub fn get_model_name(&self) -> ffi::CString {
        unsafe { (*self.0.as_ref().unwrap().get_model_name()).c_str() }.to_owned()
    }
}

#[derive(Clone, Copy)]
pub struct SensorMode(*const argus_sys::SensorMode);

impl SensorMode {
    pub fn as_interface(&self) -> ISensorMode<'_> {
        ISensorMode(
            unsafe { self.0.as_ref().unwrap().as_interface() },
            PhantomData,
        )
    }
}

pub struct ISensorMode<'a>(*const argus_sys::ISensorMode, PhantomData<&'a SensorMode>);

impl ISensorMode<'_> {
    pub fn get_resolution(&self) -> (u32, u32) {
        unsafe { self.0.as_ref().unwrap().get_resolution() }.into()
    }
}

pub struct CaptureSession<'a>(
    *mut argus_sys::CaptureSession,
    PhantomData<&'a CameraProvider>,
);

impl CaptureSession<'_> {
    pub fn as_interface(&mut self) -> ICaptureSession<'_> {
        ICaptureSession(unsafe { (*self.0).as_interface() }, PhantomData)
    }
}

impl Drop for CaptureSession<'_> {
    fn drop(&mut self) {
        unsafe {
            (*self.0)._base_1.destroy();
        }
    }
}

unsafe impl Send for CaptureSession<'_> {}

pub struct ICaptureSession<'a>(
    *mut argus_sys::ICaptureSession,
    PhantomData<&'a mut CaptureSession<'a>>,
);

impl<'a> ICaptureSession<'a> {
    pub fn create_output_stream_settings(
        &mut self,
        ty: StreamType,
    ) -> Result<OutputStreamSettings<'a>> {
        let mut status = Status::STATUS_OK;
        let out = unsafe {
            self.0
                .as_mut()
                .unwrap()
                .create_output_stream_settings(ty.0, &mut status)
        };
        status.ok()?;
        Ok(OutputStreamSettings(out, PhantomData))
    }

    pub fn create_output_stream(
        &mut self,
        settings: &OutputStreamSettings<'_>,
    ) -> Result<OutputStream<'a>> {
        let mut status = Status::STATUS_OK;
        let out = unsafe {
            self.0
                .as_mut()
                .unwrap()
                .create_output_stream(settings.0, &mut status)
        };
        status.ok()?;
        Ok(OutputStream(out, PhantomData))
    }

    pub fn create_request(&mut self, intent: CaptureIntent) -> Result<Request> {
        let mut status = Status::STATUS_OK;
        let out = unsafe {
            self.0
                .as_mut()
                .unwrap()
                .create_request(intent.0, &mut status)
        };
        status.ok()?;
        Ok(Request(out))
    }

    const INF_TIMEOUT: Duration = Duration::from_nanos(0xFFFFFFFFFFFFFFFF);

    #[inline]
    pub fn capture(&mut self, request: &Request) -> Result<u32> {
        self.capture_timeout(request, Self::INF_TIMEOUT)
    }

    pub fn capture_timeout(&mut self, request: &Request, duration: Duration) -> Result<u32> {
        let mut status = Status::STATUS_OK;
        let out = unsafe {
            self.0.as_mut().unwrap().capture(
                request.0,
                duration.as_nanos().try_into().unwrap(),
                &mut status,
            )
        };
        status.ok()?;
        Ok(out)
    }

    pub fn repeat(&mut self, request: &Request) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().repeat(request.0) }.ok()?)
    }

    pub fn stop_repeat(&mut self) {
        unsafe { self.0.as_mut().unwrap().stop_repeat() };
    }

    pub fn wait_for_idle(&mut self) -> Result<()> {
        Ok(unsafe {
            self.0
                .as_mut()
                .unwrap()
                .wait_for_idle(Self::INF_TIMEOUT.as_nanos() as _)
        }
        .ok()?)
    }
}

pub struct OutputStreamSettings<'a>(
    *mut argus_sys::OutputStreamSettings,
    PhantomData<&'a CaptureSession<'a>>,
);

impl OutputStreamSettings<'_> {
    pub fn as_egl_interface(&mut self) -> IEGLOutputStreamSettings<'_> {
        IEGLOutputStreamSettings(unsafe { (*self.0).as_egl_interface() }, PhantomData)
    }
}

impl Drop for OutputStreamSettings<'_> {
    fn drop(&mut self) {
        unsafe {
            (*self.0)._base_1.destroy();
        }
    }
}

pub struct IEGLOutputStreamSettings<'a>(
    *mut argus_sys::IEGLOutputStreamSettings,
    PhantomData<&'a mut OutputStreamSettings<'a>>,
);

impl IEGLOutputStreamSettings<'_> {
    pub fn set_pixel_format(&mut self, format: PixelFormat) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_pixel_format(format.0) }.ok()?)
    }

    pub fn set_resolution(&mut self, resolution: (u32, u32)) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_resolution(&resolution.into()) }.ok()?)
    }

    pub fn set_metadata_enable(&mut self, enabled: bool) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_metadata_enable(enabled) }.ok()?)
    }
}

pub struct OutputStream<'a>(
    *mut argus_sys::OutputStream,
    PhantomData<&'a CaptureSession<'a>>,
);

impl Drop for OutputStream<'_> {
    fn drop(&mut self) {
        unsafe {
            (*self.0)._base_1.destroy();
        }
    }
}

unsafe impl Send for OutputStream<'_> {}

pub struct FrameConsumer<'a>(
    *mut argus_sys::FrameConsumer,
    PhantomData<&'a OutputStream<'a>>,
);

impl<'a> FrameConsumer<'a> {
    pub fn from_stream(output_stream: &'a OutputStream) -> Result<Self> {
        let mut status = Status::STATUS_OK;
        let out = unsafe { argus_sys::FrameConsumer::create(output_stream.0, &mut status) };
        status.ok()?;
        Ok(Self(out, PhantomData))
    }

    pub fn as_interface(&mut self) -> IFrameConsumer {
        IFrameConsumer(unsafe { (*self.0).as_interface() }, PhantomData)
    }
}

impl Drop for FrameConsumer<'_> {
    fn drop(&mut self) {
        unsafe {
            (*self.0)._base_1.destroy();
        }
    }
}

unsafe impl Send for FrameConsumer<'_> {}

pub struct IFrameConsumer<'a>(
    *mut argus_sys::IFrameConsumer,
    PhantomData<&'a mut FrameConsumer<'a>>,
);

impl IFrameConsumer<'_> {
    pub fn acquire_frame(&mut self, timeout: Duration) -> Result<Frame> {
        let mut status = Status::STATUS_OK;
        let out = unsafe {
            self.0
                .as_mut()
                .unwrap()
                .acquire_frame(timeout.as_nanos().try_into().unwrap(), &mut status)
        };
        status.ok()?;
        Ok(Frame(out))
    }
}

unsafe impl Send for IFrameConsumer<'_> {}

pub struct Request(*mut argus_sys::Request);

impl Request {
    pub fn as_interface(&mut self) -> IRequest<'_> {
        IRequest(unsafe { (*self.0).as_interface() }, PhantomData)
    }

    pub fn as_source_settings(&mut self) -> ISourceSettings<'_> {
        ISourceSettings(unsafe { (*self.0).as_source_settings() }, PhantomData)
    }

    pub fn as_defog_settings(&mut self) -> Option<IDeFogSettings<'_>> {
        unsafe { (*self.0).as_defog_settings() }.map(|p| IDeFogSettings(p, PhantomData))
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        unsafe {
            (*self.0)._base_1.destroy();
        }
    }
}

pub struct IRequest<'a>(*mut argus_sys::IRequest, PhantomData<&'a mut Request>);

impl<'a> IRequest<'a> {
    pub fn set_pixel_format_type(&mut self, format: PixelFormatType) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_pixel_format_type(format.0) }.ok()?)
    }

    pub fn set_enable_isp_stage(&mut self, enable: bool) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_enable_isp_stage(enable) }.ok()?)
    }

    pub fn set_cv_output(&mut self, cv_output: CVOutput) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_cv_output(cv_output.0) }.ok()?)
    }

    pub fn enable_output_stream(&mut self, stream: &OutputStream) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().enable_output_stream(stream.0) }.ok()?)
    }

    pub fn get_ac_settings(&mut self) -> IAutoControlSettings<'a> {
        let raw = unsafe {
            argus_sys::IAutoControlSettings::from_provider(
                self.0.as_mut().unwrap().get_auto_control_settings(0),
            )
        };
        IAutoControlSettings(raw, PhantomData)
    }
}

pub struct ISourceSettings<'a>(
    *mut argus_sys::ISourceSettings,
    PhantomData<&'a mut Request>,
);

impl ISourceSettings<'_> {
    pub fn set_sensor_mode(&mut self, mode: SensorMode) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_sensor_mode(mode.0) }.ok()?)
    }

    pub fn set_frame_duration_range(&mut self, range: Range<u64>) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_frame_duration_range(&range) }.ok()?)
    }
}

pub struct IDeFogSettings<'a>(*mut argus_sys::IDeFogSettings, PhantomData<&'a mut Request>);

impl IDeFogSettings<'_> {
    pub fn set_defog_enable(&mut self, enable: bool) {
        unsafe { self.0.as_mut().unwrap().set_de_fog_enable(enable) }
    }
    pub fn set_defog_amount(&mut self, amount: f32) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_de_fog_amount(amount as _) }.ok()?)
    }
    pub fn set_defog_quality(&mut self, quality: f32) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_de_fog_quality(quality as _) }.ok()?)
    }
}

pub struct IAutoControlSettings<'a>(
    *mut argus_sys::IAutoControlSettings,
    PhantomData<&'a mut Request>,
);

impl IAutoControlSettings<'_> {
    pub fn set_ae_antibanding_mode(&mut self, mode: AeAntibandingMode) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_ae_antibanding_mode(mode.0) }.ok()?)
    }

    pub fn set_ae_mode(&mut self, mode: AeMode) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_ae_mode(mode.0) }.ok()?)
    }
    pub fn set_ae_lock(&mut self, lock: bool) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_ae_lock(lock) }.ok()?)
    }

    pub fn set_awb_mode(&mut self, mode: AwbMode) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_awb_mode(mode.0) }.ok()?)
    }

    pub fn set_wb_gains(&mut self, gains: [f32; 4]) -> Result<()> {
        if size_of::<BayerTuple<ffi::c_float>>() != size_of_val(&gains) {
            unimplemented!("c_float is not an f32");
        }

        unsafe {
            let acs = self.0.as_mut().unwrap();
            acs.set_awb_mode(&argus_sys::AwbMode::MANUAL).ok()?;
            acs.set_wb_gains(&gains).ok()?;
        }
        Ok(())
    }

    pub fn set_awb_lock(&mut self, lock: bool) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_awb_lock(lock) }.ok()?)
    }

    pub fn set_color_saturation(&mut self, saturation: f32) -> Result<()> {
        Ok(unsafe {
            self.0
                .as_mut()
                .unwrap()
                .set_color_saturation(saturation as _)
        }
        .ok()?)
    }

    pub fn set_color_saturation_enable(&mut self, enable: bool) -> Result<()> {
        Ok(unsafe { self.0.as_mut().unwrap().set_color_saturation_enable(enable) }.ok()?)
    }

    pub fn set_color_saturation_bias(&mut self, bias: f32) -> Result<()> {
        Ok(unsafe {
            self.0
                .as_mut()
                .unwrap()
                .set_color_saturation_bias(bias as _)
        }
        .ok()?)
    }
}

pub struct Frame(*mut argus_sys::Frame);

impl Frame {
    pub fn as_interface(&mut self) -> IFrame {
        IFrame(unsafe { (*self.0).as_interface() }, PhantomData)
    }

    pub fn get_metadata(&self) -> Option<CaptureMetadata<'_>> {
        unsafe { self.0.as_ref().unwrap().as_capture_metadata_interface() }
            .map(|p| CaptureMetadata(unsafe { p.as_ref().unwrap().get_metadata() }, PhantomData))
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            (*self.0)._base_1.destroy();
        }
    }
}

pub struct IFrame<'a>(*mut argus_sys::IFrame, PhantomData<&'a mut Frame>);

impl<'a> IFrame<'a> {
    pub fn get_number(&self) -> u64 {
        unsafe { self.0.as_ref().unwrap().get_number() }
    }

    pub fn get_time(&self) -> Duration {
        Duration::from_nanos(unsafe { self.0.as_ref().unwrap().get_number() })
    }

    pub fn get_image(&mut self) -> Image<'a> {
        Image(unsafe { self.0.as_mut().unwrap().get_image() }, PhantomData)
    }

    pub fn release_frame(self) {
        unsafe { self.0.as_mut().unwrap().release_frame() }
    }
}

pub struct Image<'a>(*mut argus_sys::Image, PhantomData<&'a Frame>);

impl Image<'_> {
    pub fn as_interface(&mut self) -> IImage<'_> {
        IImage(unsafe { (*self.0).as_interface() }, PhantomData)
    }

    pub fn as_2d_interface(&mut self) -> IImage2D<'_> {
        unsafe {
            let img_ref = self.0.as_mut().unwrap();
            IImage2D(
                img_ref.as_interface(),
                img_ref.as_2d_interface(),
                PhantomData,
            )
        }
    }

    pub fn as_native_buffer(&mut self) -> IImageNativeBuffer<'_> {
        unsafe {
            let img_ref = self.0.as_mut().unwrap();
            IImageNativeBuffer(img_ref.as_native_buffer(), PhantomData)
        }
    }

    pub fn write_jpeg(&mut self, path: &str) -> Result<()> {
        let cpath = ffi::CString::new(path).unwrap();
        Ok(unsafe {
            self.0
                .as_mut()
                .unwrap()
                .as_jpeg_interface()
                .as_ref()
                .unwrap()
                .write_jpeg(cpath.as_ptr())
        }
        .ok()?)
    }
}

pub struct IImage<'a>(*mut argus_sys::IImage, PhantomData<&'a mut Image<'a>>);

impl<'a> IImage<'a> {
    pub fn get_buffer_count(&self) -> u32 {
        unsafe { self.0.as_ref().unwrap().get_buffer_count() }
    }

    pub fn get_buffer_size(&self, index: u32) -> usize {
        unsafe { self.0.as_ref().unwrap().get_buffer_size(index) as _ }
    }

    pub fn map_buffer(&mut self, index: u32) -> Result<&'a [u8]> {
        let mut status = Status::STATUS_OK;
        let out = unsafe {
            let self_ref = self.0.as_mut().unwrap();
            let size = self_ref.get_buffer_size(index);
            let ptr = self_ref.map_buffer(index, &mut status);
            std::slice::from_raw_parts(ptr as *const _, size as _)
        };
        status.ok()?;
        Ok(out)
    }
}

pub struct IImage2D<'a>(
    *mut argus_sys::IImage,
    *mut argus_sys::IImage2D,
    PhantomData<&'a mut Image<'a>>,
);

impl<'a> IImage2D<'a> {
    pub fn get_buffer_count(&self) -> u32 {
        unsafe { self.0.as_ref().unwrap().get_buffer_count() }
    }

    pub fn get_buffer_size(&self, index: u32) -> usize {
        unsafe { self.0.as_ref().unwrap().get_buffer_size(index) as _ }
    }

    pub fn map_buffer(&mut self, index: u32) -> Result<&'a [u8]> {
        let mut status = Status::STATUS_OK;
        let out = unsafe {
            let self_ref = self.0.as_mut().unwrap();
            let size = self_ref.get_buffer_size(index);
            let ptr = self_ref.map_buffer(index, &mut status);
            std::slice::from_raw_parts(ptr as *const _, size as _)
        };
        status.ok()?;
        Ok(out)
    }

    pub fn get_size(&self, index: u32) -> (u32, u32) {
        unsafe { self.1.as_ref().unwrap().get_size(index).into() }
    }

    pub fn get_stride(&self, index: u32) -> usize {
        unsafe { self.1.as_ref().unwrap().get_stride(index) as _ }
    }
}

impl Debug for IImage2D<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num_bufs = self.get_buffer_count();
        let strides = (0..num_bufs)
            .map(|i| self.get_stride(i))
            .collect::<Vec<_>>();
        let sizes = (0..num_bufs).map(|i| self.get_size(i)).collect::<Vec<_>>();

        f.debug_struct("IImage2D")
            .field("sizes", &sizes)
            .field("strides", &strides)
            .finish()
    }
}

pub struct IImageNativeBuffer<'a>(
    *mut argus_sys::IImageNativeBuffer,
    PhantomData<&'a mut Image<'a>>,
);

impl IImageNativeBuffer<'_> {
    pub fn create_nv_buf(
        &self,
        size: (u32, u32),
        format: NvBufSurfaceColorFormat,
        layout: NvBufSurfaceLayout,
        rotation_90s: u32,
    ) -> Result<NvBufSurface> {
        if rotation_90s >= argus_sys::Rotation::ROTATION_COUNT as _ {
            return Err(Error(Status::STATUS_INVALID_PARAMS));
        }

        let mut status = Status::STATUS_OK;
        unsafe {
            let fd = self.0.as_ref().unwrap().create_nv_buffer(
                size.into(),
                format,
                layout,
                std::mem::transmute::<u32, argus_sys::Rotation>(rotation_90s),
                &mut status,
            );
            status.ok()?;
            if fd == -1 {
                todo!("handle dmabuf_fd == -1");
            }

            let mut raw = 0xdeadbeef as *mut ffi::c_void;
            if argus_sys::NvBufSurfaceFromFd(fd, &mut raw) != 0 {
                todo!("handle buf surface from fd failure");
            }
            Ok(NvBufSurface(raw as _))
        }
    }
}

pub struct CaptureMetadata<'a>(*const argus_sys::CaptureMetadata, PhantomData<&'a Frame>);

impl<'a> CaptureMetadata<'a> {
    pub fn as_inferface(&self) -> ICaptureMetadata<'a> {
        ICaptureMetadata(
            unsafe { self.0.as_ref().unwrap().as_interface() },
            PhantomData,
        )
    }
}

pub struct ICaptureMetadata<'a>(
    *const argus_sys::ICaptureMetadata,
    PhantomData<&'a CaptureMetadata<'a>>,
);

impl ICaptureMetadata<'_> {
    pub fn get_awb_wb_estimate(&self) -> Result<Vec<f32>> {
        let mut out_vec = CppVector::new();
        unsafe { self.0.as_ref().unwrap().get_awb_wb_estimate(&mut out_vec) }.ok()?;
        Ok(out_vec.into_iter().map(|p| unsafe { *p as _ }).collect())
    }
}

pub struct NvBufSurface(*mut argus_sys::NvBufSurface);

impl Drop for NvBufSurface {
    fn drop(&mut self) {
        unsafe { argus_sys::NvBufSurfaceDestroy(self.0) };
    }
}

impl NvBufSurface {
    pub fn copy_raw(&self, res: (u32, u32), out: &mut [u8]) {
        unsafe {
            if argus_sys::NvBufSurfaceMap(
                self.0,
                -1,
                -1,
                argus_sys::NvBufSurfaceMemMapFlags::NVBUF_MAP_READ,
            ) != 0
            {
                todo!("nv buf mapping failed");
            }

            if argus_sys::NvBufSurfaceSyncForCpu(self.0, -1, -1) != 0 {
                todo!("nv buf syncing failed");
            }

            let surfaces = (*self.0)
                .surfaceList
                .as_ref()
                .expect("no surface list exists");

            // println!("{:?}", surfaces);

            out.copy_from_slice(std::slice::from_raw_parts(
                surfaces.mappedAddr.addr[0] as *const u8,
                (res.0 * res.1 * 4) as _,
            ));
        }
    }
}
