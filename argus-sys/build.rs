use std::path::PathBuf;

// #[cfg(target_os = "windows")]
// const CUDA_ROOT: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/";
// #[cfg(target_os = "linux")]
// const CUDA_ROOT: &str = "/usr/local/cuda-12.6/";

// #[cfg(target_os = "windows")]
// const CUDA_BIN: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/bin";
// #[cfg(target_os = "linux")]
// const CUDA_BIN: &str = "/usr/local/cuda-12.6/bin";

// #[cfg(target_os = "windows")]
// const CUDA_LIB64: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/lib/x64";
// #[cfg(target_os = "linux")]
// const CUDA_LIB64: &str = "/usr/local/cuda-12.6/lib64";

// #[cfg(target_os = "windows")]
// const CUDA_INCLUDE: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/include";
// #[cfg(target_os = "linux")]
// const CUDA_INCLUDE: &str = "/usr/local/cuda-12.6/include";

fn configuration() {
    // println!("cargo:rustc-link-search={CUDA_BIN}");

    println!("cargo:rustc-link-search=/usr/lib/aarch64-linux-gnu/nvidia");
    println!("cargo:rustc-link-lib=dylib=nvbufsurface");
    println!("cargo:rustc-link-lib=dylib=nvargus_socketclient");

    // cuda_configuration();
}

// fn cuda_configuration() {
//     println!("cargo:rustc-link-search={CUDA_LIB64}");
//     println!("cargo:rustc-link-lib=dylib=cudart");
// }

fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .allowlist_recursively(false)
        .allowlist_item("Argus::CameraProvider.*")
        .allowlist_item("Argus::ICameraProvider.*")
        .allowlist_item("Argus::CameraDevice.*")
        .allowlist_item("Argus::SensorMode.*")
        .allowlist_item("Argus::ISensorMode.*")
        .allowlist_item("Argus::CaptureSession.*")
        .allowlist_item("Argus::ICaptureSession.*")
        .allowlist_item("Argus::OutputStreamSettings.*")
        .allowlist_item("Argus::IEGLOutputStreamSettings.*")
        .allowlist_item("Argus::OutputStream.*")
        .allowlist_item("Argus::InputStream.*")
        .allowlist_item("Argus::Request.*")
        .allowlist_item("Argus::IRequest.*")
        .allowlist_item("Argus::ISourceSettings.*")
        .allowlist_item("Argus::IAutoControlSettings.*")
        .allowlist_item("Argus::Status.*")
        .allowlist_item("Argus::InterfaceID.*")
        .allowlist_item("Argus::ExtensionName.*")
        .allowlist_item("Argus::InterfaceProvider.*")
        .allowlist_item("Argus::Destructable.*")
        .allowlist_item("Argus::NamedUUID.*")
        .allowlist_item("Argus::UUID.*")
        .allowlist_item("Argus::Interface.*")
        .allowlist_item("Argus::ICameraProperties.*")
        .allowlist_item("Argus::SensorPlacement.*")
        .allowlist_item("Argus::CaptureIntent.*")
        .allowlist_item("Argus::BayerPhase.*")
        .allowlist_item("Argus::Buffer.*")
        .allowlist_item("Argus::StreamType.*")
        .allowlist_item("Argus::PixelFormat")
        .allowlist_item("Argus::PixelFormatType")
        .allowlist_item("Argus::EGLStreamMode")
        .allowlist_item("Argus::IRequest")
        .allowlist_item("Argus::AutoControlId")
        .allowlist_item("Argus::CVOutput")
        .allowlist_item("Argus::AeAntibandingMode")
        .allowlist_item("Argus::AeMode")
        .allowlist_item("Argus::AeFlickerState")
        .allowlist_item("Argus::AeState")
        .allowlist_item("Argus::AwbMode")
        .allowlist_item("Argus::AwbState")
        .allowlist_item("Argus::AfMode")
        .allowlist_item("Argus::DenoiseMode")
        .allowlist_item("Argus::EdgeEnhanceMode")
        .allowlist_item("Argus::AcRegion")
        .allowlist_item("Argus::RGBChannel")
        .allowlist_item("Argus::CaptureMetadata")
        .allowlist_item("Argus::ICaptureMetadata")
        .rustified_enum("Argus::Status")
        .rustified_enum("Argus::RGBChannel")
        .allowlist_item("Argus::Ext::IDeFogSettings")
        .allowlist_item("EGLStream::FrameConsumer.*")
        .allowlist_item("EGLStream::IFrameConsumer.*")
        .allowlist_item("EGLStream::Frame")
        .allowlist_item("EGLStream::IFrame")
        .allowlist_item("EGLStream::Image")
        .allowlist_item("EGLStream::IImage")
        .allowlist_item("EGLStream::IImage2D")
        .allowlist_item("EGLStream::IImageJPEG")
        .allowlist_item("EGLStream::IArgusCaptureMetadata")
        .allowlist_item("EGLStream::NV::IImageNativeBuffer")
        .allowlist_type("EGLStream::NV::Rotation")
        .rustified_enum("EGLStream::NV::Rotation")
        .allowlist_item("EGLDisplay")
        .allowlist_item("EGLStreamKHR")
        .allowlist_item("NvBufSurface")
        .allowlist_item("NvBufSurfaceFromFd")
        .allowlist_item("NvBufSurfaceDestroy")
        .allowlist_item("NvBufSurfaceMap")
        // .allowlist_item("NvBufSurface2Raw")
        .allowlist_item("NvBufSurfaceSyncForCpu")
        .allowlist_item("NvBufSurfaceMemType")
        .allowlist_item("NvBufSurfaceMemMapFlags")
        .allowlist_item("NvBufSurfaceParams")
        .allowlist_item("NvBufSurfaceColorFormat")
        .allowlist_item("NvBufSurfacePlaneParams")
        .allowlist_item("NvBufSurfaceMappedAddr")
        .allowlist_item("NvBufSurfaceParamsEx")
        .allowlist_item("NvBufSurfaceChromaSubsamplingParams")
        .allowlist_item("NvBufSurfacePlaneParamsEx")
        .allowlist_item("NvBufSurfaceDisplayScanFormat")
        .allowlist_item("NvBufSurfaceLayout")
        .rustified_enum("NvBufSurfaceColorFormat")
        .rustified_enum("NvBufSurfaceLayout")
        .rustified_enum("NvBufSurfaceMemType")
        .rustified_enum("NvBufSurfaceDisplayScanFormat")
        .rustified_enum("NvBufSurfaceMemMapFlags")
        .use_core()
        .enable_cxx_namespaces()
        .merge_extern_blocks(true)
        .generate_comments(false)
        .generate_block(true)
        .size_t_is_usize(true)
        .clang_args([
            "-I/usr/src/jetson_multimedia_api/argus/include",
            "-I/usr/src/jetson_multimedia_api/include",
        ])
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("failed to generate bindings");

    bindings
        .write_to_file(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("failed to write bindings");

    configuration();
}
