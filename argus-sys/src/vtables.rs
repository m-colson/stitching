use core::ffi;

use cpp_interop::container::{CppString, CppVector};

use crate::*;

cpp_interop::make_vtable! {
    DestructableVTable for Destructable {
        fn destroy();
    }
}

// NOTE: devices are not listed as const pointers but they should be immutable.
cpp_interop::make_vtable! {
    ICameraProviderVTable for ICameraProvider {
        const fn get_version() -> *const CppString;
        const fn get_vendor() -> *const CppString;
        const fn supports_extension(extension: *const ExtensionName) -> bool;
        const fn get_camera_devices(devices: *mut CppVector<*mut CameraDevice>) -> Status;
        fn set_sync_sensor_sessions_count(dual_sensors: u32, single_sensor: u32) -> Status;
        fn create_capture_session(device: *const CameraDevice, status: *mut Status) -> *mut CaptureSession;
        fn create_capture_session_many(devices: *const CppVector<*const CameraDevice>, status: *mut Status) -> *mut CaptureSession;
    }
}

cpp_interop::make_vtable! {
    InterfaceProviderVTable for InterfaceProvider {
        fn get_interface(interface_id: *const InterfaceID) -> *mut Interface;
    }
}

// NOTE: sensor modes are not listed as const pointers but they should be immutable.
cpp_interop::make_vtable! {
    ICameraPropertiesVTable for ICameraProperties {
        const fn get_uuid() -> UUID;
        const fn get_sensor_placement() -> SensorPlacement;
        const fn get_max_ae_regions() -> u32;
        const fn get_min_ae_region_size() -> Size2D<u32>;
        const fn get_max_awb_regions() -> u32;
        const fn get_max_af_regions() -> u32;
        const fn get_basic_sensor_modes(modes: *mut CppVector<*const SensorMode>) -> Status;
        const fn get_all_sensor_modes(modes: *mut CppVector<*const SensorMode>) -> Status;
        const fn get_aperture_positions(modes: *mut CppVector<i32>) -> Status;
        const fn get_available_aperture_fnumbers(modes: *mut CppVector<ffi::c_float>) -> Status;
        const fn get_focus_position_range() -> Range<i32>;
        const fn get_aperture_position_range() -> Range<i32>;
        const fn get_aperture_motor_speed_range() -> Range<ffi::c_float>;
        const fn get_isp_digital_gain_range() -> Range<ffi::c_float>;
        const fn get_exposure_compensation_range() -> Range<ffi::c_float>;
        const fn get_model_name() -> *const CppString;
        const fn get_module_string() -> *const CppString;
    }
}

cpp_interop::make_vtable! {
    ISensorModeVTable for ISensorMode {
        const fn get_resolution() -> Size2D<u32>;
        const fn get_exposure_time_range() -> Range<u64>;
        const fn get_hdr_ratio_range() -> Range<ffi::c_float>;
        const fn get_frame_duration_range() -> Range<u64>;
        const fn get_analog_gain_range() -> Range<ffi::c_float>;
        const fn get_input_bit_depth() -> u32;
        const fn get_output_bit_depth() -> u32;
        const fn get_sensor_mode_type() -> SensorModeType;
        const fn get_bayer_phase() -> BayerPhase;
        const fn is_buffer_format_supported(buffer: *mut Buffer) -> bool;
    }
}

cpp_interop::make_vtable! {
    ICaptureSessionVTable for ICaptureSession {
        fn cancel_requests() -> Status;
        fn connect_all_request_input_streams(request: *const Request, num_requests: u32) -> Status;
        fn capture(request: *const Request, timeout: u64, status: *mut Status) -> u32;
        fn capture_burst(request_list: *const CppVector<*const Request>, timeout: u64, status: *mut Status) -> u32;
        const fn max_burst_requests() -> u32;
        fn create_request(intent: *const CaptureIntent, status: *mut Status) -> *mut Request;
        fn create_output_stream_settings(ty: *const StreamType, status: *mut Status) -> *mut OutputStreamSettings;
        fn create_output_stream(settings: *const OutputStreamSettings, status: *mut Status) -> *mut OutputStream;
        fn create_input_stream_settings(ty: *const StreamType, status: *mut Status) -> *mut InputStreamSettings;
        fn create_input_stream(settings: *const InputStreamSettings, status: *mut Status) -> *mut InputStream;
        const fn is_repeating() -> bool;
        fn repeat(request: *const Request) -> Status;
        fn repeat_burst(request_list: *const CppVector<*const Request>) -> Status;
        fn stop_repeat() -> Range<u32>;
        const fn wait_for_idle(timeout: u64) -> Status;
    }
}

cpp_interop::make_vtable! {
    IEGLOutputStreamSettingsVTable for IEGLOutputStreamSettings {
        fn set_pixel_format(format: *const PixelFormat) -> Status;
        const fn get_pixel_format() -> PixelFormat;
        fn set_resolution(resolution: *const Size2D<u32>) -> Status;
        const fn get_resolution() -> Size2D<u32>;
        fn set_exposure_count(exposure_count: u32) -> Status;
        const fn get_exposure_count() -> u32;
        fn set_egl_display(egl_display: EGLDisplay) -> Status;
        const fn get_egl_display() -> EGLDisplay;
        fn set_mode(mode: *const EGLStreamMode) -> Status;
        const fn get_mode() -> EGLStreamMode;
        fn set_fifo_length(fifo_length: u32) -> Status;
        const fn get_fifo_length() -> u32;
        fn set_metadata_enable(metadata_enable: bool) -> Status;
        const fn get_metadata_enable() -> bool;
        const fn supports_output_stream_format(sensor_mode: *const SensorMode, output_format: *const PixelFormat);
    }
}

cpp_interop::make_vtable! {
    IFrameConsumerVTable for IFrameConsumer {
        fn acquire_frame(timeout: u64, status: *mut Status) -> *mut Frame;
    }
}

cpp_interop::make_vtable! {
    IRequestVTable for IRequest {
        fn enable_output_stream(stream: *mut OutputStream) -> Status;
        fn disable_output_stream(stream: *mut OutputStream) -> Status;
        fn clear_output_streams() -> Status;
        fn enable_input_stream(stream: *mut InputStream, stream_setings: *mut InputStreamSettings) -> Status;
        fn disable_input_stream(stream: *mut InputStream, stream_settings: *mut InputStreamSettings) -> Status;
        fn clear_input_streams() -> Status;
        const fn get_output_streams(streams: *mut CppVector<*mut OutputStream>) -> Status;
        const fn get_input_streams(streams: *mut CppVector<*mut InputStream>) -> Status;
        fn get_stream_settings(stream: *const OutputStream) -> *mut InterfaceProvider;
        fn get_auto_control_settings(ac_id: AutoControlId) -> *mut InterfaceProvider;
        fn get_source_settings() -> *mut InterfaceProvider;
        fn set_client_data(data: u32) -> Status;
        const fn get_client_data() -> u32;
        fn set_pixel_format_type(format_type: *const PixelFormatType) -> Status;
        const fn get_pixel_format_type() -> PixelFormatType;
        fn set_cv_output(cv_output: *const CVOutput) -> Status;
        const fn get_cv_output() -> CVOutput;
        fn set_enable_isp_stage(enable: bool) -> Status;
        const fn get_enable_isp_stage() -> bool;
        fn set_reprocessing_enable(enable: bool) -> Status;
        fn set_msb_padding(enable: bool) -> Status;
        const fn get_msb_padding() -> bool;
    }
}

cpp_interop::make_vtable! {
    ISourceSettingsVTable for ISourceSettings {
        fn set_exposure_time_range(exposure_time_range: *const Range<u64>) -> Status;
        const fn get_exposure_time_range() -> Range<u64>;
        fn set_focus_position(position: i32) -> Status;
        const fn get_focus_position() -> i32;
        fn set_aperture_position(position: i32) -> Status;
        const fn get_aperture_position() -> i32;
        fn set_aperture_motor_speed(speed: ffi::c_float) -> Status;
        const fn get_aperture_motor_speed() -> ffi::c_float;
        fn set_aperture_fnumber(fnumber: ffi::c_float) -> Status;
        const fn get_aperture_fnumber() -> ffi::c_float;
        fn set_frame_duration_range(range: *const Range<u64>) -> Status;
        const fn get_frame_duration_range() -> Range<u64>;
        fn set_gain_range(range: *const Range<ffi::c_float>) -> Status;
        const fn get_gain_range() -> Range<ffi::c_float>;
        fn set_sensor_mode(mode: *const SensorMode) -> Status;
        const fn get_sensor_mode() -> *const SensorMode;
        fn set_optical_black(optical_black_levels: *const BayerTuple<ffi::c_float>) -> Status;
        const fn get_optical_black() -> BayerTuple<ffi::c_float>;
        fn set_optical_black_enable(enable: bool) -> Status;
        const fn get_optical_black_enable() -> bool;

    }
}

cpp_interop::make_vtable! {
    IDeFogSettingsVTable for IDeFogSettings {
        fn set_de_fog_enable(enable: bool);
        const fn get_de_fog_enable() -> bool;
        fn set_de_fog_amount(amount: ffi::c_float) -> Status;
        const fn get_de_fog_amount() -> ffi::c_float;
        fn set_de_fog_quality(quality: ffi::c_float) -> Status;
        const fn get_de_fog_quality() -> ffi::c_float;
    }
}

cpp_interop::make_vtable! {
    IAutoControlSettingsVTable for IAutoControlSettings {
        fn set_ae_antibanding_mode(mode: *const AeAntibandingMode) -> Status;
        const fn get_ae_antibanding_mode() -> AeAntibandingMode;
        fn set_ae_mode(mode: *const AeMode) -> Status;
        const fn get_ae_mode() -> AeMode;
        fn set_ae_lock(lock: bool) -> Status;
        const fn get_ae_lock() -> bool;
        fn set_ae_regions(regions: *const CppVector<AcRegion>) -> Status;
        const fn get_ae_regions(regions: *mut CppVector<AcRegion>) -> Status;
        fn set_bayer_histogram_region(region: *const Rectangle<u32>) -> Status;
        const fn get_bayer_histogram_region() -> Rectangle<u32>;
        fn set_awb_lock(lock: bool) -> Status;
        const fn get_awb_lock() -> bool;
        fn set_awb_mode(mode: *const AwbMode) -> Status;
        const fn get_awb_mode() -> AwbMode;
        fn set_awb_regions(regions: *const CppVector<AcRegion>) -> Status;
        const fn get_awb_regions(regions: *mut CppVector<AcRegion>) -> Status;
        fn set_af_mode(mode: *const AfMode) -> Status;
        const fn get_af_mode() -> AfMode;
        fn set_af_regions(regions: *const CppVector<AcRegion>) -> Status;
        const fn get_af_regions(regions: *mut CppVector<AcRegion>) -> Status;
        fn set_wb_gains(gains: *const BayerTuple<ffi::c_float>) -> Status;
        const fn get_wb_gains() -> BayerTuple<ffi::c_float>;
        const fn get_color_correction_matrix_size() -> Size2D<u32>;
        fn set_color_correction_matrix(matrix: *const CppVector<ffi::c_float>) -> Status;
        const fn get_color_correction_matrix(matrix: *mut CppVector<ffi::c_float>) -> Status;
        fn set_color_correction_matrix_enable(enable: bool) -> Status;
        const fn get_color_correction_matrix_enable() -> bool;
        fn set_color_saturation(saturation: ffi::c_float) -> Status;
        const fn get_color_saturation() -> ffi::c_float;
        fn set_color_saturation_enable(enable: bool) -> Status;
        const fn get_color_saturation_enable() -> bool;
        fn set_color_saturation_bias(bias: ffi::c_float) -> Status;
        const fn get_color_saturation_bias() -> ffi::c_float;
        fn set_exposure_compensation(ev: ffi::c_float) -> Status;
        const fn get_exposure_compensation() -> ffi::c_float;
        const fn get_tone_map_curve_size(channel: RGBChannel) -> u32;
        fn set_tone_map_curve(channel: RGBChannel, curve: *const CppVector<ffi::c_float>) -> Status;
        const fn get_tone_map_curve(channel: RGBChannel, curve: *mut CppVector<ffi::c_float>) -> Status;
        fn set_tone_map_curve_enable(enable: bool) -> Status;
        const fn get_tone_map_curve_enable() -> bool;
        fn set_isp_digital_gain_range(gain: *const Range<ffi::c_float>) -> Status;
        const fn get_isp_digital_gain_range() -> Range<ffi::c_float>;
    }
}

cpp_interop::make_vtable! {
    IFrameVTable for IFrame {
        const fn get_number() -> u64;
        const fn get_time() -> u64;
        fn get_image() -> *mut Image;
        fn release_frame();
    }
}

cpp_interop::make_vtable! {
    IImageVTable for IImage {
        const fn get_buffer_count() -> u32;
        const fn get_buffer_size(index: u32) -> u64;
        fn map_buffer(index: u32, status: *mut Status) -> *const ffi::c_void;
        fn map_buffer_first(status: *mut Status) -> *const ffi::c_void;
    }
}

cpp_interop::make_vtable! {
    IImage2DVTable for IImage2D {
        const fn get_size(index: u32) -> Size2D<u32>;
        const fn get_stride(index: u32) -> u32;
    }
}

cpp_interop::make_vtable! {
    IImageJPEGVTable for IImageJPEG {
        const fn write_jpeg(path: *const ffi::c_char) -> Status;
    }
}

cpp_interop::make_vtable! {
    IImageNativeBufferVTable for IImageNativeBuffer {
        const fn create_nv_buffer(size: Size2D<u32>, format: NvBufSurfaceColorFormat, layout: NvBufSurfaceLayout, rotation: Rotation, status: *mut Status) -> ffi::c_int;
        const fn copy_to_nv_buffer(fd: ffi::c_int, rotation: Rotation) -> Status;
    }
}

cpp_interop::make_vtable! {
    IArgusCaptureMetadataVTable for IArgusCaptureMetadata {
        const fn get_metadata() -> *const CaptureMetadata;
    }
}

cpp_interop::make_vtable! {
    ICaptureMetadataVTable for ICaptureMetadata {
        const fn get_capture_id() -> u32;
        const fn get_source_index() -> u32;
        const fn get_client_data() -> u32;
        const fn get_stream_metadata(stream: *const OutputStream) -> *const InterfaceProvider;
        const fn get_bayer_histogram() -> *const InterfaceProvider;
        const fn get_rgb_histogram() -> *const InterfaceProvider;
        const fn get_ae_locked() -> bool;
        const fn get_ae_mode() -> AeMode;
        const fn get_ae_regions(regions: *mut CppVector<AcRegion>) -> Status;
        const fn get_bayer_histogram_region() -> Rectangle<u32>;
        const fn get_ae_state() -> AeState;
        const fn get_flicker_state() -> AeFlickerState;
        const fn get_aperature_position() -> i32;
        const fn get_focuser_position() -> i32;
        const fn get_awb_cct() -> u32;
        const fn get_awb_gains() -> BayerTuple<ffi::c_float>;
        const fn get_awb_mode() -> AwbMode;
        const fn get_awb_regions(regions: *mut CppVector<AcRegion>) -> Status;
        const fn get_af_mode() -> AfMode;
        const fn get_af_regions(regions: *mut CppVector<AcRegion>) -> Status;
        const fn get_sharpness_score(values: *mut CppVector<ffi::c_float>) -> Status;
        const fn get_awb_state() -> AwbState;
        const fn get_awb_wb_estimate(estimate: *mut CppVector<ffi::c_float>) -> Status;
        const fn get_color_correction_matrix_enable() -> bool;
        const fn get_color_correction_matrix(cc_matrix: *mut CppVector<ffi::c_float>) -> Status;
        const fn get_color_saturation() -> ffi::c_float;
        const fn get_frame_duration() -> u64;
        const fn get_isp_digital_gain() -> ffi::c_float;
        const fn get_scene_lux() -> ffi::c_float;
        const fn get_sensor_analog_gain() -> ffi::c_float;
        const fn get_sensor_exposure_time() -> u64;
        const fn get_sensor_sensitivity() -> u32;
        const fn get_sensor_timestamp() -> u64;
        const fn get_tone_map_curve_enabled() -> bool;
        const fn get_tone_map_curve(channel: RGBChannel, curve: *mut CppVector<ffi::c_float>) -> Status;
    }
}
