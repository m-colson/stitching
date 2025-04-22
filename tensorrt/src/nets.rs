use std::{
    ffi::CStr,
    fs,
    io::{self, Write},
    marker::PhantomData,
    path::Path,
};

use tensorrt_sys::IHostMemory;

use crate::log::Logger;

pub struct Builder<'a>(*mut tensorrt_sys::IBuilder, PhantomData<&'a mut Logger>);

impl Drop for Builder<'_> {
    fn drop(&mut self) {
        unsafe {
            (*((*self.0)._base.vtable_ as *mut tensorrt_sys::IBuilderVTable))
                .destruct
                .delete(self.0)
        }
    }
}

impl<'a> Builder<'a> {
    pub fn from_logger(logger: &'a Logger) -> Self {
        let raw = unsafe { tensorrt_sys::create_infer_builder(logger.as_ffi()) };
        Self(raw, PhantomData)
    }

    pub fn create_network_v2(
        &self,
        flags: tensorrt_sys::NetworkDefinitionCreationFlags,
    ) -> NetworkDefinition {
        let raw = unsafe {
            let funcs = tensorrt_sys::IBuilder::funcs(self.0);
            ((*funcs).create_network_v2)((*self.0).mImpl, flags)
        };
        NetworkDefinition(raw)
    }

    pub fn create_builder_config(&self) -> BuilderConfig<'_> {
        let raw = unsafe {
            let funcs = tensorrt_sys::IBuilder::funcs(self.0);
            ((*funcs).create_builder_config)((*self.0).mImpl)
        };
        BuilderConfig(raw, PhantomData)
    }

    pub fn build_serialized_network(
        &self,
        network: &NetworkDefinition,
        config: &BuilderConfig,
    ) -> Option<SerializedNetwork> {
        let raw = unsafe {
            let funcs = tensorrt_sys::IBuilder::funcs(self.0);
            ((*funcs).build_serialized_network)((*self.0).mImpl, network.as_ffi(), config.as_ffi())
        };
        (!raw.is_null()).then_some(SerializedNetwork(raw))
    }
}

pub struct BuilderConfig<'a>(
    *mut tensorrt_sys::IBuilderConfig,
    PhantomData<&'a mut Builder<'a>>,
);

impl Drop for BuilderConfig<'_> {
    fn drop(&mut self) {
        unsafe {
            (*((*self.0)._base.vtable_ as *mut tensorrt_sys::IBuilderConfigVTable))
                .destruct
                .delete(self.0)
        }
    }
}

impl BuilderConfig<'_> {
    pub fn set_memory_pool_limit(&mut self, ty: tensorrt_sys::MemoryPoolType, value: usize) {
        unsafe {
            let funcs = tensorrt_sys::IBuilderConfig::funcs(self.0);
            ((*funcs).set_memory_pool_limit)((*self.0).mImpl, ty, value);
        };
    }

    pub fn set_flag(&mut self, flag: tensorrt_sys::BuilderFlag) {
        unsafe {
            let funcs = tensorrt_sys::IBuilderConfig::funcs(self.0);
            ((*funcs).set_flag)((*self.0).mImpl, flag);
        };
    }

    pub fn clear_flag(&mut self, flag: tensorrt_sys::BuilderFlag) {
        unsafe {
            let funcs = tensorrt_sys::IBuilderConfig::funcs(self.0);
            ((*funcs).clear_flag)((*self.0).mImpl, flag);
        };
    }

    pub fn get_flag(&mut self, flag: tensorrt_sys::BuilderFlag) {
        unsafe {
            let funcs = tensorrt_sys::IBuilderConfig::funcs(self.0);
            ((*funcs).get_flag)((*self.0).mImpl, flag);
        };
    }

    #[inline]
    pub(crate) fn as_ffi(&self) -> *mut tensorrt_sys::IBuilderConfig {
        self.0
    }
}

pub struct NetworkDefinition(*mut tensorrt_sys::INetworkDefinition);

impl Drop for NetworkDefinition {
    fn drop(&mut self) {
        unsafe {
            (*((*self.0)._base.vtable_ as *mut tensorrt_sys::INetworkDefinitionVTable))
                .destruct
                .delete(self.0)
        }
    }
}

impl NetworkDefinition {
    pub fn get_name(&self) -> &str {
        let raw = unsafe {
            let funcs = tensorrt_sys::INetworkDefinition::funcs(self.0);
            ((*funcs).get_name)((*self.0).mImpl)
        };

        let null_str = unsafe { CStr::from_ptr(raw) };
        null_str.to_str().expect("illegal name")
    }

    pub fn get_flag(&self, flag: tensorrt_sys::NetworkDefinitionCreationFlag) -> bool {
        unsafe {
            let funcs = tensorrt_sys::INetworkDefinition::funcs(self.0);
            ((*funcs).get_flag)((*self.0).mImpl, flag)
        }
    }

    #[inline]
    pub(crate) fn as_ffi(&self) -> *mut tensorrt_sys::INetworkDefinition {
        self.0
    }
}

pub struct SerializedNetwork(*mut IHostMemory);

impl Drop for SerializedNetwork {
    fn drop(&mut self) {
        unsafe {
            (*((*self.0)._base.vtable_ as *mut tensorrt_sys::IHostMemoryVTable))
                .destruct
                .delete(self.0)
        }
    }
}

impl SerializedNetwork {
    pub fn save_to_file(&self, filename: impl AsRef<Path>) -> io::Result<()> {
        let data = unsafe { (*self.0).as_bytes() };
        let mut f = fs::File::create(filename)?;
        f.write_all(data)
    }
}
