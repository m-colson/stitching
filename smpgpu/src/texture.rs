use crate::{
    Buffer, FragTarget,
    bind::{AsBinding, BindResource},
    cmd::{
        CopyOp,
        render::{ColorAttachment, DepthAttachment},
    },
};

pub struct Texture {
    inner: wgpu::Texture,
}

impl Texture {
    #[inline]
    pub fn builder<'a>(dev: &'a impl AsRef<wgpu::Device>, label: &'a str) -> TextureBuilder<'a> {
        TextureBuilder::new(dev.as_ref(), Some(label))
    }

    #[must_use]
    #[inline]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.inner.format()
    }

    #[must_use]
    #[inline]
    pub fn as_frag_target(&self) -> FragTarget {
        FragTarget::new(self.inner.format())
    }

    #[inline]
    pub(crate) fn texture_view_dimension(&self) -> wgpu::TextureViewDimension {
        if self.size().depth_or_array_layers == 1 {
            wgpu::TextureViewDimension::D2
        } else {
            wgpu::TextureViewDimension::D2Array
        }
    }

    #[inline]
    pub(crate) fn view(&self) -> wgpu::TextureView {
        self.inner.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(self.format()),
            dimension: Some(self.texture_view_dimension()),
            usage: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        })
    }

    #[inline]
    pub fn write_to_layer(&self, queue: &wgpu::Queue, data: &[u8], layer: u32) {
        let size = self.size();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: self,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * 4),
                rows_per_image: Some(size.height),
            },
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
        );
    }

    #[inline]
    pub fn new_staging(&self, dev: &impl AsRef<wgpu::Device>) -> Buffer {
        let size = self.size();
        Buffer::builder(dev, "texture_staging_buf")
            .size((size.width * size.height * size.depth_or_array_layers * 4) as _)
            .writable()
            .build_untyped()
    }

    #[must_use]
    #[inline]
    pub fn copy_to<'a>(&'a self, buf: &'a Buffer) -> CopyOp<'a> {
        let size = self.size();
        CopyOp::TextBuf(
            self,
            wgpu::Origin3d::ZERO,
            wgpu::TextureAspect::All,
            buf,
            size,
        )
    }

    #[must_use]
    #[inline]
    pub fn color_attach(&self) -> ColorAttachment {
        ColorAttachment::new(self.view())
    }

    #[must_use]
    #[inline]
    pub fn depth_attach(&self) -> DepthAttachment {
        DepthAttachment::new(self.view())
    }
}

impl AsBinding for Texture {
    fn as_binding(&self) -> (wgpu::BindingType, BindResource<'_>) {
        let access: wgpu::StorageTextureAccess = match (
            self.usage().contains(wgpu::TextureUsages::COPY_SRC),
            self.usage().contains(wgpu::TextureUsages::COPY_DST),
        ) {
            (true, true) => wgpu::StorageTextureAccess::ReadWrite,
            (true, false) => wgpu::StorageTextureAccess::WriteOnly,
            (false, true) => wgpu::StorageTextureAccess::ReadOnly,
            (false, false) => panic!("attempted to add a texture with read or write flags"),
        };

        let ty = if self.usage().contains(wgpu::TextureUsages::STORAGE_BINDING) {
            wgpu::BindingType::StorageTexture {
                access,
                format: self.format(),
                view_dimension: self.texture_view_dimension(),
            }
        } else {
            todo!("non storage textures");
        };

        (ty, BindResource::TextureView(self.view()))
    }
}

pub struct TextureBuilder<'a> {
    dev: &'a wgpu::Device,
    label: Option<&'a str>,
    width: u32,
    height: u32,
    layers: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
}

impl<'a> TextureBuilder<'a> {
    #[must_use]
    #[inline]
    pub const fn new(dev: &'a wgpu::Device, label: Option<&'a str>) -> Self {
        Self {
            dev,
            label,
            width: 0,
            height: 0,
            layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
        }
    }

    #[must_use]
    #[inline]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    #[must_use]
    #[inline]
    pub fn depth(mut self) -> Self {
        self.format = wgpu::TextureFormat::Depth32Float;
        self.render_target()
    }

    #[must_use]
    #[inline]
    pub const fn size(mut self, width: usize, height: usize) -> Self {
        self.width = width as _;
        self.height = height as _;
        self
    }

    #[must_use]
    #[inline]
    pub const fn layers(mut self, layers: usize) -> Self {
        self.layers = layers as _;
        self
    }

    #[must_use]
    #[inline]
    fn with_usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage |= usage;
        self
    }

    #[must_use]
    #[inline]
    pub fn storage(mut self) -> Self {
        self.usage |= wgpu::TextureUsages::STORAGE_BINDING;
        self.usage.remove(wgpu::TextureUsages::TEXTURE_BINDING);
        self
    }

    #[must_use]
    #[inline]
    pub fn render_target(self) -> Self {
        self.with_usage(wgpu::TextureUsages::RENDER_ATTACHMENT)
    }

    #[must_use]
    #[inline]
    pub fn readable(self) -> Self {
        self.with_usage(wgpu::TextureUsages::COPY_SRC)
    }

    #[must_use]
    #[inline]
    pub fn writable(self) -> Self {
        self.with_usage(wgpu::TextureUsages::COPY_DST)
    }

    #[must_use]
    #[inline]
    pub fn build(self) -> Texture {
        let inner = self.dev.create_texture(&wgpu::TextureDescriptor {
            label: self.label,
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: self.layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: self.usage,
            view_formats: &[],
        });

        Texture { inner }
    }
}

impl std::ops::Deref for Texture {
    type Target = wgpu::Texture;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
