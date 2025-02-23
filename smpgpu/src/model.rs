use std::borrow::Cow;

use encase::{ShaderSize, ShaderType};

use crate::{
    cmd::CheckpointBuilder,
    typed_buffer::{IndexBufferBuilder, IndexBufferFormat, UniformBuilder, VertexBufferBuilder},
    AsRenderItem, AutoVisBindable, IndexBuffer, RenderCheckpoint, RenderItem, Uniform,
    VertexBuffer,
};

#[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
pub struct VertPos {
    pub pos: glam::Vec4,
}

#[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
pub struct VertPosNorm {
    pub pos: glam::Vec4,
    pub norm: glam::Vec4,
}

pub struct Model<V, I: IndexBufferFormat> {
    pub view: Uniform<glam::Mat4>,
    pub verts: VertexBuffer<V>,
    pub idx: IndexBuffer<I>,
    pub idx_len: u32,
    pub cp: RenderCheckpoint,
}

impl<V: ShaderSize, I: IndexBufferFormat> Model<V, I> {
    #[inline]
    pub fn builder(dev: &impl AsRef<wgpu::Device>) -> ModelBuilder<'_, V, I>
    where
        V: Clone,
    {
        ModelBuilder::new(dev.as_ref())
    }

    pub fn set_view(&self, m: glam::Mat4) {
        self.view.set_global(&m);
    }

    pub fn with_view(self, m: glam::Mat4) -> Self {
        self.view.set_global(&m);
        self
    }
}

impl<V: ShaderSize, I: IndexBufferFormat> AsRenderItem for Model<V, I> {
    fn as_item(&self) -> RenderItem<'_> {
        self.cp
            .vert_buf(&self.verts)
            .index_buf(&self.idx, 0..self.idx_len)
    }
}

pub struct ModelBuilder<'a, V: ShaderSize + Clone, I: IndexBufferFormat> {
    dev: &'a wgpu::Device,
    verts: Cow<'a, [V]>,
    idxs: Cow<'a, [I]>,
}

impl<'a, V: ShaderSize + Clone, I: IndexBufferFormat> ModelBuilder<'a, V, I> {
    pub(crate) fn new(dev: &'a wgpu::Device) -> Self {
        Self {
            dev,
            verts: Cow::Owned(Vec::new()),
            idxs: Cow::Owned(Vec::new()),
        }
    }

    pub fn verts(mut self, verts: &'a [V]) -> Self {
        self.verts = Cow::Borrowed(verts);
        self
    }

    pub fn indices(mut self, indices: &'a [I]) -> Self {
        self.idxs = Cow::Borrowed(indices);
        self
    }

    pub fn cp_build_cam(
        self,
        cam: &Uniform<glam::Mat4>,
        f: impl FnOnce(CheckpointBuilder<'a>) -> RenderCheckpoint,
    ) -> Model<V, I> {
        let view = UniformBuilder::new(self.dev)
            .writable()
            .init_data(&glam::Mat4::IDENTITY)
            .build();

        let verts = VertexBufferBuilder::new(self.dev)
            .init_data(&self.verts)
            .build();
        let idx_len = self.idxs.len();
        let idx = IndexBufferBuilder::new(self.dev)
            .init_data(&self.idxs)
            .build();

        let cp = f(CheckpointBuilder::new(self.dev).group(view.in_vertex() & cam.in_vertex()));

        Model {
            view,
            verts,
            idx,
            idx_len: idx_len as _,
            cp,
        }
    }
}

impl<I> ModelBuilder<'_, VertPosNorm, I>
where
    I: crate::typed_buffer::IndexBufferFormat,
    obj::Vertex: obj::FromRawVertex<I>,
{
    #[cfg(feature = "obj-file")]
    pub fn obj_file_reader(mut self, r: impl std::io::BufRead) -> Self {
        let obj = obj::load_obj(r).expect("failed to load model object file");
        self.verts = obj
            .vertices
            .into_iter()
            .map(|v: obj::Vertex| VertPosNorm {
                pos: glam::vec4(v.position[0], v.position[1], v.position[2], 1.),
                norm: glam::vec4(v.normal[0], v.normal[1], v.normal[2], 0.),
            })
            .collect::<Vec<_>>()
            .into();

        // let (cmin, cmax) = self.verts.iter().fold(
        //     (
        //         glam::vec4(f32::MAX, f32::MAX, f32::MAX, 1.),
        //         glam::vec4(f32::MIN, f32::MIN, f32::MIN, 1.),
        //     ),
        //     |(cmin, cmax), v| (v.pos.min(cmin), v.pos.max(cmax)),
        // );
        // println!("model has min {cmin:?} and max {cmax:?}");

        self.idxs = obj.indices.into();
        self
    }
}
