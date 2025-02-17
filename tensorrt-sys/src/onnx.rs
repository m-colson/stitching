use std::ffi::{c_char, c_int};

use cpp_interop::DestructorVEntry;

pub use crate::nvonnxparser::*;
use crate::{root::SubGraphCollection_t, ITensor};

pub struct IParserVTable {
    pub parse: unsafe extern "C" fn(
        this: *mut IParser,
        serialized_onnx_model: *const core::ffi::c_void,
        serialized_onnx_model_size: usize,
        model_path: *const core::ffi::c_char,
    ) -> bool,
    pub parse_from_file: unsafe extern "C" fn(
        this: *mut IParser,
        onnxModelFile: *const core::ffi::c_char,
        verbosity: core::ffi::c_int,
    ) -> bool,
    pub supports_model: unsafe extern "C" fn(
        this: *mut IParser,
        serialized_onnx_model: *const core::ffi::c_void,
        serialized_onnx_model_size: usize,
        sub_graph_collection: *mut SubGraphCollection_t,
        model_path: *const core::ffi::c_char,
    ) -> bool,
    pub parse_with_weight_descriptors: unsafe extern "C" fn(
        this: *mut IParser,
        serialized_onnx_model: *const core::ffi::c_void,
        serialized_onnx_model_size: usize,
    ) -> bool,
    pub supports_operator:
        unsafe extern "C" fn(this: *mut IParser, op_name: *const core::ffi::c_char) -> bool,
    pub get_nb_errors: unsafe extern "C" fn(this: *mut IParser) -> core::ffi::c_int,
    pub get_error:
        unsafe extern "C" fn(this: *mut IParser, index: core::ffi::c_int) -> *const IParserError,
    pub clear_errors: unsafe extern "C" fn(this: *mut IParser),
    pub destruct: DestructorVEntry<IParser>,
    pub get_used_vcplugin_libraries: unsafe extern "C" fn(
        this: *mut IParser,
        nbPluginLibs: *mut i64,
    ) -> *const *const core::ffi::c_char,
    pub set_flags: unsafe extern "C" fn(this: *mut IParser, onnxParserFlags: OnnxParserFlags),
    pub get_flags: unsafe extern "C" fn(this: *mut IParser) -> OnnxParserFlags,
    pub clear_flag: unsafe extern "C" fn(this: *mut IParser, onnxParserFlag: OnnxParserFlag),
    pub set_flag: unsafe extern "C" fn(this: *mut IParser, onnxParserFlag: OnnxParserFlag),
    pub get_flag: unsafe extern "C" fn(this: *mut IParser, onnxParserFlag: OnnxParserFlag) -> bool,
    pub get_layer_output_tensor: unsafe extern "C" fn(
        this: *mut IParser,
        name: *const core::ffi::c_char,
        i: i64,
    ) -> *const ITensor,
    pub supports_model_v2: unsafe extern "C" fn(
        this: *mut IParser,
        serializedOnnxModel: *const core::ffi::c_void,
        serializedOnnxModelSize: usize,
        modelPath: *const core::ffi::c_char,
    ) -> bool,
    pub get_nb_subgraphs: unsafe extern "C" fn(this: *mut IParser) -> i64,
    pub is_subgraph_supported: unsafe extern "C" fn(this: *mut IParser, index: i64) -> bool,
    pub get_subgraph_nodes:
        unsafe extern "C" fn(this: *mut IParser, index: i64, subgraphLength: *mut i64) -> *mut i64,
}

pub struct IParserErrorVTable {
    //     virtual ErrorCode code() const = 0;
    pub code: unsafe extern "C" fn(this: *const IParserError) -> ErrorCode,
    //     virtual char const* desc() const = 0;
    pub desc: unsafe extern "C" fn(this: *const IParserError) -> *const c_char,
    //     virtual char const* file() const = 0;
    pub file: unsafe extern "C" fn(this: *const IParserError) -> *const c_char,
    //     virtual int line() const = 0;
    pub line: unsafe extern "C" fn(this: *const IParserError) -> c_int,
    //     virtual char const* func() const = 0;
    pub func: unsafe extern "C" fn(this: *const IParserError) -> *const c_char,
    //     virtual int node() const = 0;
    pub node: unsafe extern "C" fn(this: *const IParserError) -> c_int,
    //     virtual char const* nodeName() const = 0;
    pub node_name: unsafe extern "C" fn(this: *const IParserError) -> *const c_char,
    //     virtual char const* nodeOperator() const = 0;
    pub node_operator: unsafe extern "C" fn(this: *const IParserError) -> *const c_char,
    //     virtual char const* const* localFunctionStack() const = 0;
    pub local_function_stack:
        unsafe extern "C" fn(this: *const IParserError) -> *const *const c_char,
    //     virtual int32_t localFunctionStackSize() const = 0;
    pub local_function_stack_size: unsafe extern "C" fn(this: *const IParserError) -> i32,
    // protected:
    //     virtual ~IParserError() {}
    // pub destruct: DestructorVEntry<IParserError>,
}
