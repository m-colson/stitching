use std::path::PathBuf;

// #[cfg(target_os = "windows")]
// const CUDA_ROOT: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/";
// #[cfg(target_os = "linux")]
// const CUDA_ROOT: &str = "/usr/local/cuda-12.6/";

#[cfg(target_os = "windows")]
const CUDA_BIN: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/bin";
#[cfg(target_os = "linux")]
const CUDA_BIN: &str = "/usr/local/cuda-12.6/bin";

#[cfg(target_os = "windows")]
const CUDA_LIB64: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/lib/x64";
#[cfg(target_os = "linux")]
const CUDA_LIB64: &str = "/usr/local/cuda-12.6/lib64";

#[cfg(target_os = "windows")]
const CUDA_INCLUDE: &str = "C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.6/include";
#[cfg(target_os = "linux")]
const CUDA_INCLUDE: &str = "/usr/local/cuda-12.6/include";

fn configuration() {
    println!("cargo:rustc-link-search={CUDA_BIN}");

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=dylib=nvinfer_10");
        println!("cargo:rustc-link-lib=dylib=nvonnxparser_10");
    }

    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dylib=nvinfer");
        println!("cargo:rustc-link-lib=dylib=nvonnxparser");
    }

    cuda_configuration();
}

fn cuda_configuration() {
    println!("cargo:rustc-link-search={CUDA_LIB64}");
    println!("cargo:rustc-link-lib=dylib=cudart");
}

fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .impl_debug(true)
        .blocklist_type(".*EnumMaxImpl")
        .allowlist_item("nvinfer1::.*")
        .allowlist_item("nvonnxparser::.*")
        .rustified_enum(".*Severity.*")
        .rustified_enum(".*nvonnxparser::ErrorCode")
        .rustified_enum(".*nvinfer1::BuilderFlag")
        .rustified_enum(".*nvinfer1::MemoryPoolType")
        .rustified_enum(".*nvinfer1::ExecutionContextAllocationStrategy")
        .rustified_enum(".*nvinfer1::DataType")
        .rustified_enum(".*cudaError")
        .rustified_enum(".*cudaMemcpyKind")
        .opaque_type(".*SubGraph_t.*")
        .blocklist_function(".*std::.*")
        .blocklist_type(".*std::.*iterator.*")
        .blocklist_type(".*std::.*Iter.*")
        .blocklist_type(".*std::vector.*")
        .blocklist_type(".*std::__allocator_base")
        .blocklist_item(".*__gnu_cxx::\\w+.*")
        .blocklist_item(".*__gnu_cxx::_.*")
        .allowlist_function(".*create.*_INTERNAL")
        .allowlist_function(".*cuda.*")
        .use_core()
        .enable_cxx_namespaces()
        .merge_extern_blocks(true)
        .generate_comments(false)
        .generate_block(true)
        .size_t_is_usize(true)
        .clang_args([&format!("-I{CUDA_INCLUDE}")])
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("failed to generate bindings");

    bindings
        .write_to_file(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("failed to write bindings");

    configuration();
}
