macro_rules! define_enumw {
    ($name:ident, $($variants:ident),*) => {
        #[derive(Clone, Copy)]
        #[allow(dead_code)]
        pub struct $name(pub(crate) *const ::argus_sys::$name);

        impl $name {
            $(
                pub const $variants: Self = Self(&::argus_sys::$name::$variants);
            )*
        }
    };
}

define_enumw!(AeAntibandingMode, OFF, AUTO, F50HZ, F60HZ);
define_enumw!(AeMode, OFF, ON);
define_enumw!(AeFlickerState, NONE, F50HZ, F60HZ);
define_enumw!(
    AeState,
    INACTIVE,
    SEARCHING,
    CONVERGED,
    FLASH_REQUIRED,
    TIMEOUT
);
define_enumw!(
    AwbMode,
    OFF,
    AUTO,
    INCANDESCENT,
    FLUORESCENT,
    WARM_FLUORESCENT,
    DAYLIGHT,
    CLOUDY_DAYLIGHT,
    TWILIGHT,
    SHADE,
    MANUAL
);

define_enumw!(StreamType, EGL);
define_enumw!(
    PixelFormat,
    UNKNOWN,
    Y8,
    Y16,
    YCBCR_420_888,
    YCBCR_422_888,
    YCBCR_444_888,
    JPEG_BLOB,
    RAW16,
    P016,
    RGBA
);
define_enumw!(
    CaptureIntent,
    MANUAL,
    PREVIEW,
    STILL_CAPTURE,
    VIDEO_RECORD,
    VIDEO_SNAPSHOT
);
define_enumw!(PixelFormatType, NONE, YUV_ONLY, RGB_ONLY, BOTH);
define_enumw!(CVOutput, NONE, LINEAR, NON_LINEAR);
