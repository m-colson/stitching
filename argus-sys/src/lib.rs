mod autobind {
    #![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::ffi;

pub use autobind::root::{
    Argus::{
        AcRegion, AeAntibandingMode, AeFlickerState, AeMode, AeState, AfMode, AutoControlId,
        AwbMode, AwbState, BayerPhase, Buffer, BufferSettings, BufferType, CVOutput, CameraDevice,
        CameraProvider, CameraProvider_create, CaptureIntent, CaptureMetadata, CaptureSession,
        DenoiseMode, Destructable, EGLStreamMode, EdgeEnhanceMode, Ext::IDeFogSettings,
        ExtensionName, IAutoControlSettings, ICameraProperties, ICameraProvider, ICaptureMetadata,
        ICaptureMetadata_NUM_AWB_WB_ESTIMATE_ELEMENTS,
        ICaptureMetadata_NUM_COLOR_CORRECTION_ELEMENTS, ICaptureSession, IEGLOutputStreamSettings,
        IRequest, ISensorMode, ISourceSettings, InputStream, InputStreamSettings, Interface,
        InterfaceID, InterfaceProvider, NamedUUID, OutputStream, OutputStreamSettings, PixelFormat,
        PixelFormatType, RGBChannel, Request, SensorMode, SensorModeType, SensorPlacement, Status,
        StreamType, UUID,
    },
    EGLDisplay,
    EGLStream::{
        Frame, FrameConsumer, IArgusCaptureMetadata, IFrame, IFrameConsumer, IImage, IImage2D,
        IImageJPEG, Image,
        NV::{IImageNativeBuffer, Rotation},
    },
    NvBufSurface, NvBufSurfaceColorFormat, NvBufSurfaceDestroy, NvBufSurfaceFromFd,
    NvBufSurfaceLayout, NvBufSurfaceMap, NvBufSurfaceMemMapFlags, NvBufSurfaceSyncForCpu,
};

mod uuid_enums;
pub mod vtables;

pub type Range<T> = [T; 2];
pub type Size2D<T> = [T; 2];
pub type BayerTuple<T> = [T; 4];
pub type Rectangle<T> = [T; 4];

impl Status {
    pub fn ok(self) -> Result<(), Self> {
        match self {
            Status::STATUS_OK => Ok(()),
            err => Err(err),
        }
    }
}

impl NamedUUID {
    pub const fn new(
        time_low: u32,
        time_mid: u16,
        time_hi_and_version: u16,
        clock_seq: u16,
        node: [u8; 6],
        m_name: [ffi::c_char; 32],
    ) -> Self {
        Self {
            _base: UUID {
                time_low,
                time_mid,
                time_hi_and_version,
                clock_seq,
                node,
            },
            m_name,
        }
    }

    pub const fn into_interface(self) -> InterfaceID {
        InterfaceID { _base: self }
    }
}

#[macro_export]
macro_rules! define_uuid {
    ($ty:path,
        $name:ident,
        $l:literal,
        $s0:literal,
        $s1:literal,
        $s2:literal,
        $c0:literal,
        $c1:literal,
        $c2:literal,
        $c3:literal,
        $c4:literal,
        $c5:literal) => {
        pub const $name: $ty = {
            $ty {
                _base: $crate::NamedUUID::new(
                    $l,
                    $s0,
                    $s1,
                    $s2,
                    [$c0, $c1, $c2, $c3, $c4, $c5],
                    [0; 32],
                ),
            }
        };
    };
    ($name:ident,
        $l:literal,
        $s0:literal,
        $s1:literal,
        $s2:literal,
        $c0:literal,
        $c1:literal,
        $c2:literal,
        $c3:literal,
        $c4:literal,
        $c5:literal) => {
        $crate::define_uuid!(Self, $name, $l, $s0, $s1, $s2, $c0, $c1, $c2, $c3, $c4, $c5);
    };
}

impl Interface {
    fn check_valid<T>(p: *mut Self) -> *mut T {
        if p.is_null() {
            panic!("unknown interface, likely implementation bug")
        }
        p as _
    }

    fn check_valid_opt<T>(p: *mut Self) -> Option<*mut T> {
        (!p.is_null()).then_some(p as _)
    }

    fn check_valid_const<T>(p: *const Self) -> *const T {
        if p.is_null() {
            panic!("unknown interface, likely implementation bug")
        }
        p as _
    }

    fn check_valid_const_opt<T>(p: *const Self) -> Option<*const T> {
        (!p.is_null()).then_some(p as _)
    }
}

impl InterfaceProvider {
    pub(crate) unsafe fn get_interface_const(
        &self,
        interface_id: *const InterfaceID,
    ) -> *const Interface {
        (self as *const Self as *mut Self)
            .as_mut()
            .unwrap()
            .get_interface(interface_id)
    }
}

impl CameraProvider {
    pub unsafe fn as_interface(&mut self) -> *mut ICameraProvider {
        const IID_CAMERA_PROVIDER: InterfaceID = NamedUUID::new(
            0xa00f33d7,
            0x8564,
            0x4226,
            0x955c,
            [0x2d, 0x1b, 0xcd, 0xaf, 0xa3, 0x5f],
            [0; 32],
        )
        .into_interface();

        Interface::check_valid(self._base.get_interface(&IID_CAMERA_PROVIDER))
    }
}

impl CameraDevice {
    pub unsafe fn as_properties(&self) -> *const ICameraProperties {
        const IID_CAMERA_PROPERTIES: InterfaceID = NamedUUID::new(
            0x436d2a73,
            0xc85b,
            0x4a29,
            0xbce5,
            [0x15, 0x60, 0x6e, 0x35, 0x86, 0x91],
            [0; 32],
        )
        .into_interface();

        Interface::check_valid_const(self._base.get_interface_const(&IID_CAMERA_PROPERTIES))
    }
}

impl SensorMode {
    pub unsafe fn as_interface(&self) -> *const ISensorMode {
        const IID_SENSOR_MODE: InterfaceID = NamedUUID::new(
            0xe69015e0,
            0xdb2a,
            0x11e5,
            0xa837,
            [0x18, 0x00, 0x20, 0x0c, 0x9a, 0x66],
            [0; 32],
        )
        .into_interface();
        Interface::check_valid_const(self._base.get_interface_const(&IID_SENSOR_MODE))
    }
}

impl CaptureSession {
    pub unsafe fn as_interface(&mut self) -> *mut ICaptureSession {
        const IID_CAPTURE_SESSION: InterfaceID = NamedUUID::new(
            0x813644f5,
            0xbc21,
            0x4013,
            0xaf44,
            [0xdd, 0xda, 0xb5, 0x7a, 0x9d, 0x13],
            [0; 32],
        )
        .into_interface();

        Interface::check_valid(self._base.get_interface(&IID_CAPTURE_SESSION))
    }
}

impl OutputStreamSettings {
    pub unsafe fn as_egl_interface(&mut self) -> *mut IEGLOutputStreamSettings {
        const IID_EGL_OUTPUT_STREAM_SETTINGS: InterfaceID = NamedUUID::new(
            0x3a659361,
            0x5231,
            0x11e7,
            0x9598,
            [0x18, 0x00, 0x20, 0x0c, 0x9a, 0x66],
            [0; 32],
        )
        .into_interface();

        Interface::check_valid(self._base.get_interface(&IID_EGL_OUTPUT_STREAM_SETTINGS))
    }
}

impl FrameConsumer {
    pub unsafe fn as_interface(&mut self) -> *mut IFrameConsumer {
        const IID_FRAME_CONSUMER: InterfaceID = NamedUUID::new(
            0xb94a7bd1,
            0xc3c8,
            0x11e5,
            0xa837,
            [0x08, 0x00, 0x20, 0x0c, 0x9a, 0x66],
            [0; 32],
        )
        .into_interface();
        Interface::check_valid(self._base.get_interface(&IID_FRAME_CONSUMER))
    }
}

impl Request {
    pub unsafe fn as_interface(&mut self) -> *mut IRequest {
        const IID_REQUEST: InterfaceID = NamedUUID::new(
            0xeb9b3750,
            0xfc8d,
            0x455f,
            0x8e0f,
            [0x91, 0xb3, 0x3b, 0xd9, 0x4e, 0xc5],
            [0; 32],
        )
        .into_interface();
        Interface::check_valid(self._base.get_interface(&IID_REQUEST))
    }

    pub unsafe fn as_source_settings(&mut self) -> *mut ISourceSettings {
        define_uuid!(
            InterfaceID,
            IID_SOURCE_SETTINGS,
            0xeb7ae38c,
            0x3c62,
            0x4161,
            0xa92a,
            0xa6,
            0x4f,
            0xba,
            0xc6,
            0x38,
            0x83
        );
        Interface::check_valid(self._base.get_interface(&IID_SOURCE_SETTINGS))
    }

    pub unsafe fn as_defog_settings(&mut self) -> Option<*mut IDeFogSettings> {
        define_uuid!(
            InterfaceID,
            IID_DE_FOG_SETTINGS,
            0x9cf05bd1,
            0x1d99,
            0x4be8,
            0x8732,
            0x75,
            0x99,
            0x55,
            0x7f,
            0xed,
            0x3a
        );
        Interface::check_valid_opt(self._base.get_interface(&IID_DE_FOG_SETTINGS))
    }
}

impl IAutoControlSettings {
    pub unsafe fn from_provider(p: *mut InterfaceProvider) -> *mut Self {
        define_uuid!(
            InterfaceID,
            IID_AUTO_CONTROL_SETTINGS,
            0x1f2ad1c6,
            0xcb13,
            0x440b,
            0xbc95,
            0x3f,
            0xfd,
            0x0d,
            0x19,
            0x91,
            0xdb
        );
        Interface::check_valid(
            p.as_mut()
                .unwrap()
                .get_interface(&IID_AUTO_CONTROL_SETTINGS),
        )
    }
}

impl Frame {
    pub unsafe fn as_interface(&mut self) -> *mut IFrame {
        const IID_FRAME: InterfaceID = NamedUUID::new(
            0x546F4520,
            0x87EF,
            0x11E5,
            0xA837,
            [0x08, 0x00, 0x20, 0x0C, 0x9A, 0x66],
            [0; 32],
        )
        .into_interface();
        Interface::check_valid(self._base.get_interface(&IID_FRAME))
    }

    pub unsafe fn as_capture_metadata_interface(&self) -> Option<*const IArgusCaptureMetadata> {
        define_uuid!(
            InterfaceID,
            IID_ARGUS_CAPTURE_METADATA,
            0xb94aa2e0,
            0xc3c8,
            0x11e5,
            0xa837,
            0x08,
            0x00,
            0x20,
            0x0c,
            0x9a,
            0x66
        );
        Interface::check_valid_const_opt(
            self._base.get_interface_const(&IID_ARGUS_CAPTURE_METADATA),
        )
    }
}

impl Image {
    pub unsafe fn as_interface(&mut self) -> *mut IImage {
        const IID_IMAGE: InterfaceID = NamedUUID::new(
            0x546F4522,
            0x87EF,
            0x11E5,
            0xA837,
            [0x08, 0x00, 0x20, 0x0C, 0x9A, 0x66],
            [0; 32],
        )
        .into_interface();
        Interface::check_valid(self._base.get_interface(&IID_IMAGE))
    }

    pub unsafe fn as_2d_interface(&mut self) -> *mut IImage2D {
        define_uuid!(
            InterfaceID,
            IID_IMAGE_2D,
            0x546F4525,
            0x87EF,
            0x11E5,
            0xA837,
            0x08,
            0x00,
            0x20,
            0x0C,
            0x9A,
            0x66
        );
        Interface::check_valid(self._base.get_interface(&IID_IMAGE_2D))
    }

    pub unsafe fn as_jpeg_interface(&mut self) -> *mut IImageJPEG {
        define_uuid!(
            InterfaceID,
            IID_IMAGE_JPEG,
            0x48aeddc9,
            0xc8d8,
            0x11e5,
            0xa837,
            0x08,
            0x00,
            0x20,
            0x0c,
            0x9a,
            0x66
        );
        Interface::check_valid(self._base.get_interface(&IID_IMAGE_JPEG))
    }

    pub unsafe fn as_native_buffer(&mut self) -> *mut IImageNativeBuffer {
        define_uuid!(
            InterfaceID,
            IID_IMAGE_NATIVE_BUFFER,
            0x2f410340,
            0x1793,
            0x11e6,
            0xbdf4,
            0x08,
            0x00,
            0x20,
            0x0c,
            0x9a,
            0x66
        );
        Interface::check_valid(self._base.get_interface(&IID_IMAGE_NATIVE_BUFFER))
    }
}

impl CaptureMetadata {
    pub unsafe fn as_interface(&self) -> *const ICaptureMetadata {
        define_uuid!(
            InterfaceID,
            IID_CAPTURE_METADATA,
            0x5f6ac5d4,
            0x59e8,
            0x45d0,
            0x8bac,
            0x38,
            0x09,
            0x1f,
            0xf8,
            0x74,
            0xa9
        );
        Interface::check_valid_const(self._base.get_interface_const(&IID_CAPTURE_METADATA))
    }
}
