use std::{ffi::CStr, marker::PhantomData};

use tensorrt_sys::{create_infer_runtime, DataType};

use crate::{
    cuda::{CudaBuffer, CudaStream},
    log::Logger,
};

pub struct RuntimeEngineContext<'a>(
    #[allow(dead_code)] *mut tensorrt_sys::IRuntime,
    #[allow(dead_code)] *mut tensorrt_sys::ICudaEngine,
    *mut tensorrt_sys::IExecutionContext,
    PhantomData<&'a Logger>,
);

impl RuntimeEngineContext<'_> {
    pub fn new_engine_slice(plan: &[u8]) -> Self {
        let rt = Runtime::new();
        let eng = rt.cuda_engine_from_slice(plan);
        let ctx =
            eng.create_execution_context(tensorrt_sys::ExecutionContextAllocationStrategy::kSTATIC);
        Self(rt.0, eng.0, ctx.0, PhantomData)
    }

    pub fn as_ctx(&self) -> ExecutionContext<'_> {
        ExecutionContext(self.2, PhantomData)
    }
}

unsafe impl Send for RuntimeEngineContext<'_> {}

pub struct Runtime<'a>(*mut tensorrt_sys::IRuntime, PhantomData<&'a Logger>);

impl<'a> Runtime<'a> {
    pub fn new() -> Self {
        Self::from_logger(crate::DEFAULT_LOGGER)
    }

    pub fn from_logger(logger: &'a Logger) -> Self {
        let raw = unsafe { create_infer_runtime(logger.as_ffi()) };
        Runtime(raw, PhantomData)
    }

    pub fn cuda_engine_from_slice<'b>(&'b self, plan: &[u8]) -> CudaEngine<'b> {
        let raw = unsafe { (*self.0).deserialize_slice_to_cuda_engine(plan) };
        CudaEngine(raw, PhantomData)
    }
}

impl Default for Runtime<'_> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CudaEngine<'a>(*mut tensorrt_sys::ICudaEngine, PhantomData<&'a Runtime<'a>>);

impl<'a> CudaEngine<'a> {
    pub fn create_execution_context(
        &self,
        strategy: ExecutionContextAllocationStrategy,
    ) -> ExecutionContext<'a> {
        let raw = unsafe { (*self.0).create_execution_context(strategy) };
        ExecutionContext(raw, PhantomData)
    }

    pub fn io_tensor_names(&self) -> Vec<&CStr> {
        let count = unsafe { (*self.0).get_nb_iotensors() };
        (0..count)
            .map(|i| unsafe { (*self.0).get_iotensor_name(i) })
            .collect()
    }

    pub fn tensor_shape(&self, name: &CStr) -> Vec<i64> {
        let raw = unsafe { (*self.0).get_tensor_shape(name) };
        raw.d[..raw.nbDims as usize].to_vec()
    }

    pub fn tensor_type(&self, name: &CStr) -> DataType {
        unsafe { (*self.0).get_tensor_type(name) }
    }
}

pub type ExecutionContextAllocationStrategy = tensorrt_sys::ExecutionContextAllocationStrategy;

pub struct ExecutionContext<'a>(
    *mut tensorrt_sys::IExecutionContext,
    PhantomData<&'a CudaEngine<'a>>,
);

impl ExecutionContext<'_> {
    pub fn enqueue(&self, stream: &CudaStream) -> bool {
        unsafe { (*self.0).enqueue_v3(stream.as_ffi()) }
    }

    pub fn set_input_tensor(&self, name: &CStr, data: &CudaBuffer) -> bool {
        unsafe { (*self.0).set_input_tensor_address(name, data.as_ffi()) }
    }

    pub fn set_output_tensor(&self, name: &CStr, data: &mut CudaBuffer) -> bool {
        unsafe { (*self.0).set_output_tensor_address(name, data.as_ffi()) }
    }
}
