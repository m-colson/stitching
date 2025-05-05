//! Unsafe bindings to
//! [TensorRT](https://docs.nvidia.com/deeplearning/tensorrt/latest/_static/c-api/index.html)
//! based on the C++ headers.
//!
//! It will use the pre-generated bindings included in this crate unless
//! the `build` feature flag is enabled.

pub(crate) mod bind {
    #![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]

    #[cfg(feature = "build")]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    #[cfg(not(feature = "build"))]
    pub(crate) mod generated;
    #[cfg(not(feature = "build"))]
    pub use generated::*;

    pub use root::nvinfer1::*;
    pub use root::nvonnxparser;

    /// # Safety
    pub unsafe fn create_infer_builder(logger: *mut ILogger) -> *mut IBuilder {
        root::createInferBuilder_INTERNAL(logger as _, TENSORRT_VERSION) as _
    }

    /// # Safety
    pub unsafe fn create_infer_runtime(logger: *mut ILogger) -> *mut IRuntime {
        root::createInferRuntime_INTERNAL(logger as _, TENSORRT_VERSION) as _
    }

    /// # Safety
    pub unsafe fn create_onnx_parser(
        network: *mut INetworkDefinition,
        logger: *mut ILogger,
    ) -> *mut nvonnxparser::IParser {
        root::createNvOnnxParser_INTERNAL(network as _, logger as _, ONNX_PARSER_VERSION as _) as _
    }
}

pub mod onnx;
mod vtables;
use std::ffi::CStr;

use root::cudaStream_t;
pub use vtables::*;

pub use bind::*;

#[derive(Debug)]
pub struct CudaError(root::cudaError);

impl core::fmt::Display for CudaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl core::error::Error for CudaError {}

pub type CudaResult<T> = ::std::result::Result<T, CudaError>;

impl From<root::cudaError> for CudaResult<()> {
    fn from(err: root::cudaError) -> Self {
        match err {
            root::cudaError::cudaSuccess => Ok(()),
            err => Err(CudaError(err)),
        }
    }
}

impl IBuilder {
    /// # Safety
    /// `IBuilder` should have been initialized by TensorRT itself
    pub unsafe fn funcs(v: *const Self) -> *const VBuilderVTable {
        (*(*v).mImpl)._base.vtable_ as _
    }
}

impl IBuilderConfig {
    /// # Safety
    /// `IBuilderConfig` should have been initialized by TensorRT itself
    pub unsafe fn funcs(v: *const Self) -> *const VBuilderConfigVTable {
        (*(*v).mImpl)._base.vtable_ as _
    }
}

impl INetworkDefinition {
    /// # Safety
    /// `INetworkDefinition` should have been initialized by TensorRT itself
    pub unsafe fn funcs(v: *const Self) -> *const VNetworkDefinitionVTable {
        (*(*v).mImpl)._base.vtable_ as _
    }
}

impl IHostMemory {
    /// # Safety
    /// `this` should have been created by TensorRT
    pub unsafe fn data(this: *const Self) -> *mut core::ffi::c_void {
        let v = (*this).mImpl;
        ((*((*v)._base.vtable_ as *const VHostMemoryVTable)).data)(v)
    }

    /// # Safety
    /// `this` should have been created by TensorRT
    pub unsafe fn size(this: *const Self) -> usize {
        let v = (*this).mImpl;
        ((*((*v)._base.vtable_ as *const VHostMemoryVTable)).size)(v)
    }

    /// # Safety
    /// self` should have been created by TensorRT
    pub unsafe fn as_bytes(&self) -> &[u8] {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VHostMemoryVTable;
        let data = ((*impl_funcs).data)(impl_ptr);
        let len = ((*impl_funcs).size)(impl_ptr);
        std::slice::from_raw_parts(data as _, len)
    }

    /// # Safety
    /// `this` should have been created by TensorRT
    pub unsafe fn data_type(this: *const Self) -> DataType {
        let v = (*this).mImpl;
        ((*((*v)._base.vtable_ as *const VHostMemoryVTable)).data_type)(v)
    }
}

impl IRuntime {
    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn deserialize_slice_to_cuda_engine(&self, data: &[u8]) -> *mut ICudaEngine {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VRuntimeVTable;
        ((*impl_funcs).deserialize_cuda_engine_blob)(impl_ptr, data.as_ptr() as _, data.len())
    }

    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn get_logger(&self) -> *mut ILogger {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VRuntimeVTable;
        ((*impl_funcs).get_logger)(impl_ptr)
    }
}

impl ICudaEngine {
    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn create_execution_context(
        &self,
        strategy: ExecutionContextAllocationStrategy,
    ) -> *mut IExecutionContext {
        println!("{:?}", self as *const _);
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VCudaEngineVTable;
        ((*impl_funcs).create_execution_context)(impl_ptr, strategy)
    }

    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn get_nb_iotensors(&self) -> i32 {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VCudaEngineVTable;
        ((*impl_funcs).get_nb_iotensors)(impl_ptr)
    }

    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn get_iotensor_name(&self, index: i32) -> &CStr {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VCudaEngineVTable;
        CStr::from_ptr(((*impl_funcs).get_iotensor_name)(impl_ptr, index))
    }

    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn get_tensor_shape(&self, name: &CStr) -> Dims64 {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VCudaEngineVTable;
        ((*impl_funcs).get_tensor_shape)(impl_ptr, name.as_ptr())
    }

    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn get_tensor_type(&self, name: &CStr) -> DataType {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VCudaEngineVTable;
        ((*impl_funcs).get_tensor_data_type)(impl_ptr, name.as_ptr())
    }
}

impl IExecutionContext {
    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn enqueue_v3(&self, queue: cudaStream_t) -> bool {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VExecutionContextVTable;
        ((*impl_funcs).enqueue_v3)(impl_ptr, queue)
    }

    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn set_input_tensor_address(
        &self,
        name: &CStr,
        data: *const core::ffi::c_void,
    ) -> bool {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VExecutionContextVTable;
        ((*impl_funcs).set_input_tensor_address)(impl_ptr, name.as_ptr(), data)
    }

    /// # Safety
    /// `self` should have been created by TensorRT
    pub unsafe fn set_output_tensor_address(
        &self,
        name: &CStr,
        data: *mut core::ffi::c_void,
    ) -> bool {
        let impl_ptr = self.mImpl;
        let impl_funcs = (*impl_ptr)._base.vtable_ as *const VExecutionContextVTable;
        ((*impl_funcs).set_output_tensor_address)(impl_ptr, name.as_ptr(), data)
    }
}
