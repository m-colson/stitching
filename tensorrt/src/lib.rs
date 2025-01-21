mod cuda;
mod log;
mod nets;
pub mod onnx;
mod rt;

use std::ffi::CString;

pub use cuda::*;
pub use log::*;
pub use nets::*;
pub use rt::*;

pub fn onnx_file_to_plan(filename: &str) -> SerializedNetwork {
    let builder = Builder::from_logger(DEFAULT_LOGGER);
    let mut network =
        builder.create_network_v2(1 << tensorrt_sys::NetworkDefinitionCreationFlag_kSTRONGLY_TYPED);

    let mut parser = onnx::Parser::new(&mut network, DEFAULT_LOGGER);
    parser.parse_from_file(
        &CString::new(filename).expect("illegal filename"),
        Severity::kWARNING,
    );

    let errs = parser.get_errors();
    if !errs.is_empty() {
        panic!("failed to build plan: {errs:#?}");
    }

    let mut config = builder.create_builder_config();
    config.set_memory_pool_limit(tensorrt_sys::MemoryPoolType::kWORKSPACE, 512 * 1024 * 1024);

    let serialized = builder.build_serialized_network(&network, &config);
    drop(parser); // For now, make sure the compiler doesn't shorten lifetime. FIX

    serialized
}

pub fn onnx_slice_to_plan(data: &[u8]) -> SerializedNetwork {
    let builder = Builder::from_logger(DEFAULT_LOGGER);
    let mut network =
        builder.create_network_v2(1 << tensorrt_sys::NetworkDefinitionCreationFlag_kSTRONGLY_TYPED);

    let mut parser = onnx::Parser::new(&mut network, DEFAULT_LOGGER);
    parser.parse_from_slice(data);

    let errs = parser.get_errors();
    if !errs.is_empty() {
        panic!("failed to build plan: {errs:#?}");
    }

    let mut config = builder.create_builder_config();
    config.set_memory_pool_limit(tensorrt_sys::MemoryPoolType::kWORKSPACE, 512 * 1024 * 1024);

    let serialized = builder.build_serialized_network(&network, &config);
    drop(parser); // For now, make sure the compiler doesn't shorten lifetime. FIX

    serialized
}
