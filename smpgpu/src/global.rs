//! This module contains functions that create builders using the default global [`Context`].

use std::sync::{Arc, LazyLock};

use encase::{internal::WriteInto, ShaderSize};
use pollster::FutureExt;

use crate::{
    cmd::{CheckpointBuilder, CommandBuilder},
    model::{Model, ModelBuilder},
    texture::TextureBuilder,
    typed_buffer::{
        IndexBuffer, IndexBufferBuilder, IndexBufferFormat, StorageBufferBuilder, UniformBuilder,
        VertexBuffer, VertexBufferBuilder,
    },
    Buffer, BufferBuilder, Checkpoint, Context, StorageBuffer, Texture, Uniform,
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
pub fn buffer() -> BufferBuilder<'static> {
    Buffer::builder(&**GLOBAL_CONTEXT)
}

/// Calls [`Uniform::builder`] with the global context.
pub fn uniform<T: ShaderSize>() -> UniformBuilder<'static, T> {
    Uniform::builder(&**GLOBAL_CONTEXT)
}

impl<T: ShaderSize + WriteInto> Uniform<T> {
    pub fn set_global(&self, v: &T) {
        GLOBAL_CONTEXT.write_uniform(&self.0, v);
    }
}

/// Calls [`StorageBuffer::builder`] with the global context.
pub fn storage_buffer<T: ShaderSize>() -> StorageBufferBuilder<'static, T> {
    StorageBuffer::builder(&**GLOBAL_CONTEXT)
}

impl<T: ShaderSize + WriteInto> StorageBuffer<T> {
    pub fn set_global(&self, v: &[T]) {
        GLOBAL_CONTEXT.write_storage(&self.0, v);
    }
}

/// Calls [`VertexBuffer::builder`] with the global context.
pub fn vertex_buffer<T: ShaderSize>() -> VertexBufferBuilder<'static, T> {
    VertexBuffer::builder(&**GLOBAL_CONTEXT)
}

impl<T: ShaderSize + WriteInto> VertexBuffer<T> {
    pub fn set_global(&self, v: &[T]) {
        GLOBAL_CONTEXT.write_storage(&self.0, v);
    }
}

/// Calls [`IndexBuffer::builder`] with the global context.
pub fn index_buffer<T: IndexBufferFormat>() -> IndexBufferBuilder<'static, T> {
    IndexBuffer::builder(&**GLOBAL_CONTEXT)
}

impl<T: ShaderSize + WriteInto> IndexBuffer<T> {
    pub fn set_global(&self, v: &[T]) {
        GLOBAL_CONTEXT.write_storage(&self.0, v);
    }
}

/// Calls [`Texture::builder`] with the global context.
pub fn texture() -> TextureBuilder<'static> {
    Texture::builder(&**GLOBAL_CONTEXT)
}

impl Texture {
    #[inline]
    pub fn new_staging_global(&self) -> Buffer {
        let size = self.size();
        buffer()
            .label("texture_staging_buf")
            .size((size.width * size.height * size.depth_or_array_layers * 4) as _)
            .writable()
            .build()
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
    Model::builder(&**GLOBAL_CONTEXT)
}

/// Calls [`Context::signal_wake`] on the global context.
pub fn force_wake() {
    GLOBAL_CONTEXT.signal_wake();
}
