//! This module contains functions that create builders using the default global [`Context`].

use std::sync::{Arc, LazyLock};

use encase::{ShaderSize, internal::WriteInto};
use pollster::FutureExt;

use crate::{
    Buffer, Checkpoint, Context, StorageBuffer, Texture, Uniform,
    buffer::{
        BufferBuilder,
        typed::{IndexBuffer, IndexBufferFormat, UniformBuilder, VertexBuffer},
    },
    cmd::{CheckpointBuilder, CommandBuilder},
    model::{ModelBuilder, RendModel},
    texture::TextureBuilder,
};

static GLOBAL_CONTEXT: LazyLock<Arc<Context>> = LazyLock::new(|| {
    async {
        Context::builder()
            .request_adapter()
            .await?
            .request_build()
            .await
    }
    .block_on()
    .expect("failed to initialize global context")
});

/// Gets the current global context. AVOID when possible.
pub fn get_global_context() -> Arc<Context> {
    GLOBAL_CONTEXT.clone()
}

/// Calls [`Buffer::builder`] with the global context.
pub fn buffer(label: &str) -> BufferBuilder<'_> {
    Buffer::builder(&**GLOBAL_CONTEXT, label)
}

/// Calls [`Uniform::builder`] with the global context.
pub fn uniform<T: ShaderSize>(label: &str) -> UniformBuilder<'_, T> {
    Uniform::builder(&**GLOBAL_CONTEXT, label)
}

impl<T: ShaderSize + WriteInto> Uniform<T> {
    pub fn set_global(&self, v: &T) {
        GLOBAL_CONTEXT.write_uniform(&self.0, v);
    }
}

impl<T: ShaderSize + WriteInto> StorageBuffer<T> {
    pub fn set_global(&self, v: &[T]) {
        GLOBAL_CONTEXT.write_storage(&self.0, v);
    }
}

impl<T: ShaderSize + WriteInto> VertexBuffer<T> {
    pub fn set_global(&mut self, v: &[T]) {
        GLOBAL_CONTEXT.write_storage(&self.0, v);
        self.1 = v.len() as _;
    }
}

impl<T: ShaderSize + WriteInto> IndexBuffer<T> {
    pub fn set_global(&self, v: &[T]) {
        GLOBAL_CONTEXT.write_storage(&self.0, v);
    }
}

/// Calls [`Texture::builder`] with the global context.
pub fn texture(label: &str) -> TextureBuilder<'_> {
    Texture::builder(&**GLOBAL_CONTEXT, label)
}

impl Texture {
    #[inline]
    pub fn new_staging_global(&self) -> Buffer {
        let size = self.size();
        Buffer::builder(&**GLOBAL_CONTEXT, "texture_staging_buf")
            .size((size.width * size.height * size.depth_or_array_layers * 4) as _)
            .writable()
            .build_untyped()
    }
}

/// Calls [`Checkpoint::builder`] with the global context.
pub fn checkpoint() -> CheckpointBuilder<'static> {
    Checkpoint::builder(&**GLOBAL_CONTEXT)
}

/// Calls [`CommandBuilder::new`] with the global context.
pub fn command() -> CommandBuilder<'static> {
    CommandBuilder::new(&GLOBAL_CONTEXT)
}

pub fn model<V: ShaderSize + Clone, I: IndexBufferFormat>() -> ModelBuilder<'static, V, I> {
    RendModel::builder(&**GLOBAL_CONTEXT)
}

/// Calls [`Context::signal_wake`] on the global context.
pub fn force_wake() {
    GLOBAL_CONTEXT.signal_wake();
}

pub mod prelude {
    pub use super::{
        buffer, checkpoint, command, force_wake as force_global_wake, model, texture, uniform,
    };
}
