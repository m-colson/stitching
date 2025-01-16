use tensorrt_sys::root::cudaError as CudaError;

pub type CudaResult<T> = ::std::result::Result<T, CudaError>;

pub struct CudaStream(tensorrt_sys::root::cudaStream_t);

impl CudaStream {
    pub fn new() -> CudaResult<Self> {
        let mut raw = std::ptr::null_mut();
        let err = unsafe { tensorrt_sys::root::cudaStreamCreate(&mut raw) };
        match err {
            CudaError::cudaSuccess => Ok(Self(raw)),
            err => Err(err),
        }
    }

    pub fn synchronize(&self) -> CudaResult<()> {
        let err = unsafe { tensorrt_sys::root::cudaStreamSynchronize(self.0) };
        match err {
            CudaError::cudaSuccess => Ok(()),
            err => Err(err),
        }
    }

    #[inline]
    pub(crate) fn as_ffi(&self) -> tensorrt_sys::root::cudaStream_t {
        self.0
    }
}

pub struct CudaBuffer(*mut core::ffi::c_void, usize);

impl Drop for CudaBuffer {
    fn drop(&mut self) {
        unsafe { tensorrt_sys::root::cudaFree(self.0) };
    }
}

impl CudaBuffer {
    pub fn new(bytes: usize) -> CudaResult<Self> {
        let mut raw = std::ptr::null_mut();
        let err = unsafe { tensorrt_sys::root::cudaMalloc(&mut raw, bytes) };
        match err {
            CudaError::cudaSuccess => Ok(Self(raw, bytes)),
            err => Err(err),
        }
    }

    pub fn copy_from_async(&mut self, src: &[u8], stream: &CudaStream) -> CudaResult<()> {
        let size = src.len();
        if size != self.1 {
            panic!("CudaBuffer::copy_from_async | src and dst have different size");
        }
        let err = unsafe {
            tensorrt_sys::root::cudaMemcpyAsync(
                self.0,
                src.as_ptr() as _,
                size,
                tensorrt_sys::root::cudaMemcpyKind::cudaMemcpyHostToDevice,
                stream.as_ffi(),
            )
        };
        match err {
            CudaError::cudaSuccess => Ok(()),
            err => Err(err),
        }
    }

    pub fn copy_to_async(&mut self, dst: &mut [u8], stream: &CudaStream) -> CudaResult<()> {
        let size = dst.len();
        if size != self.1 {
            panic!("CudaBuffer::copy_from_async | src and dst have different size");
        }
        let err = unsafe {
            tensorrt_sys::root::cudaMemcpyAsync(
                dst.as_mut_ptr() as _,
                self.0,
                size,
                tensorrt_sys::root::cudaMemcpyKind::cudaMemcpyDeviceToHost,
                stream.as_ffi(),
            )
        };
        match err {
            CudaError::cudaSuccess => Ok(()),
            err => Err(err),
        }
    }

    pub(crate) fn as_ffi(&self) -> *mut core::ffi::c_void {
        self.0
    }
}
