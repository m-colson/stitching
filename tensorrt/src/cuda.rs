use std::ffi::c_void;

use tensorrt_sys::root::cudaMemcpyKind;

pub use tensorrt_sys::{CudaError, CudaResult};

pub struct CudaStream(tensorrt_sys::root::cudaStream_t);

impl CudaStream {
    pub fn new() -> CudaResult<Self> {
        let mut raw = std::ptr::null_mut();
        CudaResult::from(unsafe { tensorrt_sys::root::cudaStreamCreate(&mut raw) })
            .map(|_| Self(raw))
    }

    pub fn synchronize(&self) -> CudaResult<()> {
        unsafe { tensorrt_sys::root::cudaStreamSynchronize(self.0) }.into()
    }

    #[inline]
    pub(crate) fn as_ffi(&self) -> tensorrt_sys::root::cudaStream_t {
        self.0
    }
}

unsafe impl Send for CudaStream {}
unsafe impl Sync for CudaStream {}

pub struct CudaBuffer(*mut core::ffi::c_void, usize);

impl Drop for CudaBuffer {
    fn drop(&mut self) {
        unsafe { tensorrt_sys::root::cudaFree(self.0) };
    }
}

unsafe impl Send for CudaBuffer {}

impl CudaBuffer {
    pub fn new(bytes: usize) -> CudaResult<Self> {
        let mut raw = std::ptr::null_mut();
        CudaResult::from(unsafe { tensorrt_sys::root::cudaMalloc(&mut raw, bytes) })
            .map(|_| Self(raw, bytes))
    }

    pub fn copy_from_async(&mut self, src: &[u8], stream: &CudaStream) -> CudaResult<()> {
        let size = src.len();
        if size != self.1 {
            panic!("CudaBuffer::copy_from_async | src and dst have different size");
        }
        cuda_copy_async(
            src.as_ptr() as _,
            self.0,
            size,
            cudaMemcpyKind::cudaMemcpyHostToDevice,
            stream,
        )
    }

    pub fn copy_from(&mut self, src: &[u8]) -> CudaResult<()> {
        let size = src.len();
        if size != self.1 {
            panic!("CudaBuffer::copy_from | src and dst have different size");
        }
        cuda_copy(
            src.as_ptr() as _,
            self.0,
            size,
            cudaMemcpyKind::cudaMemcpyHostToDevice,
        )
    }

    pub fn copy_to_async(&self, dst: &mut [u8], stream: &CudaStream) -> CudaResult<()> {
        let size = dst.len();
        if size != self.1 {
            panic!("CudaBuffer::copy_to_async | src and dst have different size");
        }
        cuda_copy_async(
            self.0,
            dst.as_mut_ptr() as _,
            size,
            cudaMemcpyKind::cudaMemcpyDeviceToHost,
            stream,
        )
    }

    pub fn copy_to(&self, dst: &mut [u8]) -> CudaResult<()> {
        let size = dst.len();
        if size != self.1 {
            panic!("CudaBuffer::copy_to | src and dst have different size");
        }
        cuda_copy(
            self.0,
            dst.as_mut_ptr() as _,
            size,
            cudaMemcpyKind::cudaMemcpyDeviceToHost,
        )
    }

    pub(crate) fn as_ffi(&self) -> *mut core::ffi::c_void {
        self.0
    }
}

fn cuda_copy_async(
    src: *const c_void,
    dst: *mut c_void,
    size: usize,
    kind: tensorrt_sys::root::cudaMemcpyKind,
    stream: &CudaStream,
) -> CudaResult<()> {
    unsafe { tensorrt_sys::root::cudaMemcpyAsync(dst, src, size, kind, stream.as_ffi()) }.into()
}

fn cuda_copy(
    src: *const c_void,
    dst: *mut c_void,
    size: usize,
    kind: tensorrt_sys::root::cudaMemcpyKind,
) -> CudaResult<()> {
    unsafe { tensorrt_sys::root::cudaMemcpy(dst, src, size, kind) }.into()
}
