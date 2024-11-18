use encase::{CalculateSizeFor, ShaderSize};

use crate::cmd::{BindResource, Bindable, CopyOp, EncoderOp};

pub struct Texture {
    inner: wgpu::Texture,
    width: u32,
    height: u32,
    layers: u32,
}

impl Texture {
    #[inline]
    pub fn builder(dev: &wgpu::Device) -> TextureBuilder<'_> {
        TextureBuilder::new(dev)
    }

    #[inline]
    pub(crate) fn texture_view_dimension(&self) -> wgpu::TextureViewDimension {
        if self.layers != 1 {
            wgpu::TextureViewDimension::D2Array
        } else {
            wgpu::TextureViewDimension::D2
        }
    }

    #[inline]
    pub fn view(&self) -> wgpu::TextureView {
        self.inner.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(self.texture_view_dimension()),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        })
    }

    #[inline]
    pub fn write_to_layer(&self, queue: &wgpu::Queue, data: &[u8], layer: u32) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
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
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.width * 4),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    #[inline]
    pub fn new_staging(&self, dev: &impl AsRef<wgpu::Device>) -> Buffer {
        Buffer::builder(dev)
            .label("texture_staging_buf")
            .size((self.width * self.height * self.layers * 4) as _)
            .writable()
            .build()
    }

    #[inline]
    pub fn copy_to_buf_op<'a>(&'a self, buf: &'a Buffer) -> impl EncoderOp + 'a {
        CopyOp::TextBuf(
            self,
            wgpu::Origin3d::ZERO,
            wgpu::TextureAspect::All,
            buf,
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: self.layers,
            },
        )
    }
}

impl Bindable for Texture {
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
    usage: wgpu::TextureUsages,
}

impl<'a> TextureBuilder<'a> {
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self {
            dev,
            label: None,
            width: 0,
            height: 0,
            layers: 1,
            usage: wgpu::TextureUsages::empty(),
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn size(mut self, width: usize, height: usize) -> Self {
        self.width = width as _;
        self.height = height as _;
        self
    }

    pub fn layers(mut self, layers: usize) -> Self {
        self.layers = layers as _;
        self
    }

    pub fn with_usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage |= usage;
        self
    }

    pub fn storage(self) -> Self {
        self.with_usage(wgpu::TextureUsages::STORAGE_BINDING)
    }

    pub fn readable(self) -> Self {
        self.with_usage(wgpu::TextureUsages::COPY_SRC)
    }

    pub fn writable(self) -> Self {
        self.with_usage(wgpu::TextureUsages::COPY_DST)
    }

    pub fn build(self) -> Texture {
        let inner = self.dev.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: self.layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: self.usage,
            view_formats: &[],
        });

        Texture {
            inner,
            width: self.width,
            height: self.height,
            layers: self.layers,
        }
    }
}

impl std::ops::Deref for Texture {
    type Target = wgpu::Texture;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct Buffer {
    inner: wgpu::Buffer,
}

impl Buffer {
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> BufferBuilder<'_> {
        BufferBuilder::new(dev.as_ref())
    }

    #[inline]
    pub fn copy_to_buf_op<'a>(&'a self, buf: &'a Buffer) -> impl EncoderOp + 'a {
        CopyOp::BufBuf(self, 0, buf, 0, self.size())
    }
}

impl Bindable for Buffer {
    fn as_binding(&self) -> (wgpu::BindingType, BindResource<'_>) {
        let ty = if self.usage().contains(wgpu::BufferUsages::STORAGE) {
            wgpu::BufferBindingType::Storage {
                read_only: !self.usage().contains(wgpu::BufferUsages::COPY_SRC),
            }
        } else if self.usage().contains(wgpu::BufferUsages::UNIFORM) {
            wgpu::BufferBindingType::Uniform
        } else {
            panic!("attempted to make a binding for a buffer that is neither STORAGE or UNIFORM");
        };

        (
            wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            BindResource::Buffer(self),
        )
    }
}

pub struct BufferBuilder<'a> {
    dev: &'a wgpu::Device,
    label: Option<&'a str>,
    size: u64,
    usage: wgpu::BufferUsages,
}

impl<'a> BufferBuilder<'a> {
    pub fn new(dev: &'a wgpu::Device) -> Self {
        Self {
            dev,
            label: None,
            size: 0,
            usage: wgpu::BufferUsages::empty(),
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn size(mut self, size: usize) -> Self {
        self.size = size as _;
        self
    }

    pub fn size_for<T: ShaderSize>(mut self) -> Self {
        self.size = u64::from(T::SHADER_SIZE);
        self
    }

    pub fn size_for_many<T: CalculateSizeFor>(mut self, elms: u64) -> Self {
        self.size = u64::from(T::calculate_size_for(elms));
        self
    }

    pub fn with_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage |= usage;
        if (self.usage.contains(wgpu::BufferUsages::MAP_READ)
            || self.usage.contains(wgpu::BufferUsages::MAP_WRITE))
            && (self.usage.contains(wgpu::BufferUsages::UNIFORM)
                || self.usage.contains(wgpu::BufferUsages::STORAGE))
        {
            self.usage
                .remove(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::MAP_WRITE);
        }
        self
    }

    pub fn readable(self) -> Self {
        self.with_usage(wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE)
    }

    pub fn writable(self) -> Self {
        self.with_usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ)
    }

    pub fn storage(self) -> Self {
        self.with_usage(wgpu::BufferUsages::STORAGE)
    }

    pub fn uniform(self) -> Self {
        self.with_usage(wgpu::BufferUsages::UNIFORM)
    }

    pub fn build(self) -> Buffer {
        let inner = self.dev.create_buffer(&wgpu::BufferDescriptor {
            label: self.label,
            size: self.size,
            usage: self.usage,
            mapped_at_creation: false,
        });

        Buffer { inner }
    }
}

impl std::ops::Deref for Buffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
