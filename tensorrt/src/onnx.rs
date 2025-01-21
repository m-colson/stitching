use std::{ffi::CStr, fmt::Debug, marker::PhantomData};

use crate::{
    log::{Logger, Severity},
    nets::NetworkDefinition,
};

pub struct Parser<'a>(*mut tensorrt_sys::onnx::IParser, PhantomData<&'a Logger>);

impl Drop for Parser<'_> {
    fn drop(&mut self) {
        unsafe {
            (*((*self.0).vtable_ as *mut tensorrt_sys::onnx::IParserVTable))
                .destruct
                .delete(self.0)
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(net: &mut NetworkDefinition, logger: &'a Logger) -> Self {
        let raw = unsafe { tensorrt_sys::create_onnx_parser(net.as_ffi(), logger.as_ffi()) };
        Self(raw, PhantomData)
    }

    unsafe fn vfuncs(&self) -> *const tensorrt_sys::onnx::IParserVTable {
        (*self.0).vtable_ as *const tensorrt_sys::onnx::IParserVTable
    }

    pub fn parse_from_file(&mut self, name: &CStr, severity: Severity) {
        unsafe {
            let vfuncs = self.vfuncs();
            ((*vfuncs).parse_from_file)(self.0, name.as_ptr(), severity as i32);
        }
    }

    pub fn parse_from_slice(&mut self, data: &[u8]) {
        unsafe {
            let vfuncs = self.vfuncs();
            ((*vfuncs).parse)(
                self.0,
                data.as_ptr() as _,
                data.len(),
                c"from-slice".as_ptr(),
            );
        }
    }

    pub fn get_errors(&self) -> Vec<ParserError<'_>> {
        let vfuncs = unsafe { self.vfuncs() };
        let err_count = unsafe { ((*vfuncs).get_nb_errors)(self.0) };

        (0..err_count)
            .map(|i| ParserError(unsafe { ((*vfuncs).get_error)(self.0, i) }, PhantomData))
            .collect()
    }
}

pub struct ParserError<'a>(
    *const tensorrt_sys::onnx::IParserError,
    PhantomData<&'a Parser<'a>>,
);

impl ParserError<'_> {
    unsafe fn vfuncs(&self) -> *const tensorrt_sys::onnx::IParserErrorVTable {
        (*self.0).vtable_ as _
    }

    pub fn code(&self) -> tensorrt_sys::onnx::ErrorCode {
        unsafe { ((*self.vfuncs()).code)(self.0) }
    }

    pub fn desc(&self) -> &CStr {
        unsafe { CStr::from_ptr(((*self.vfuncs()).desc)(self.0)) }
    }

    pub fn node_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(((*self.vfuncs()).node_name)(self.0)) }
    }
}

impl Debug for ParserError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserError")
            .field("code", &self.code())
            .field("desc", &self.desc())
            .field("node_name", &self.node_name())
            .finish()
    }
}
