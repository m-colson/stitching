use std::path::Path;
use strum::VariantArray;
use tensorrt::{CudaBuffer, CudaStream, ExecutionContext};

pub mod boxes;
pub mod coco;

pub struct Inferer<'a> {
    ctx: ExecutionContext<'a>,
    stream: CudaStream,
    in_mem: CudaBuffer,
    in_byte_count: usize,
    out_mem: CudaBuffer,
    out_byte_count: usize,
}

impl<'a> Inferer<'a> {
    pub fn from_exec_ctx(ctx: ExecutionContext<'a>, in_elems: usize, out_elems: usize) -> Self {
        let in_byte_count = in_elems * size_of::<u8>();
        let out_byte_count = out_elems * size_of::<half::f16>();

        let stream = CudaStream::new().unwrap();

        let in_mem = CudaBuffer::new(in_byte_count).unwrap();
        ctx.set_input_tensor(c"images", &in_mem);

        let mut out_mem = CudaBuffer::new(out_byte_count).unwrap();
        ctx.set_output_tensor(c"output0", &mut out_mem);

        Self {
            ctx,
            stream,
            in_mem,
            in_byte_count,
            out_mem,
            out_byte_count,
        }
    }

    pub fn run(&mut self, in_buf: &[u8], out_buf: &mut [half::f16]) {
        if size_of_val(in_buf) != self.in_byte_count {
            panic!(
                "in_buf size mismatch, got: {:?} expected: {:?}",
                size_of_val(in_buf),
                self.in_byte_count,
            )
        }

        if size_of_val(out_buf) != self.out_byte_count {
            panic!(
                "out_buf size mismatch, got: {:?} expected: {:?}",
                size_of_val(out_buf),
                self.out_byte_count,
            )
        }

        self.in_mem
            .copy_from_async(bytemuck::cast_slice(in_buf), &self.stream)
            .unwrap();
        self.ctx.enqueue(&self.stream);
        self.out_mem
            .copy_to_async(bytemuck::cast_slice_mut(out_buf), &self.stream)
            .unwrap();
        self.stream.synchronize().unwrap();
    }
}

#[derive(Clone, Copy, Debug, VariantArray)]
pub enum Which {
    // #[cfg(feature = "v11n")]
    // V11N,
    #[cfg(feature = "v11s")]
    V11S,
    // #[cfg(feature = "v11m")]
    // V11M,
    // #[cfg(feature = "v11l")]
    // V11L,
    // #[cfg(feature = "v11x")]
    // V11X,
}

impl Which {
    pub fn fastest() -> Self {
        *Self::VARIANTS
            .first()
            .expect("no models added to this binary")
    }

    pub fn best() -> Self {
        *Self::VARIANTS
            .last()
            .expect("no models added to this binary")
    }

    pub fn input_shape(self) -> [usize; 4] {
        match self {
            // #[cfg(feature = "v11n")]
            // Which::V11N => [1, 640, 640, 3],
            #[cfg(feature = "v11s")]
            Which::V11S => [1, 640, 640, 3],
            // #[cfg(feature = "v11m")]
            // Which::V11M => [1, 640, 640, 3],
            // #[cfg(feature = "v11l")]
            // Which::V11L => [1, 640, 640, 3],
            // #[cfg(feature = "v11x")]
            // Which::V11X => [1, 640, 640, 3],
        }
    }

    pub fn input_elems(self) -> usize {
        self.input_shape().into_iter().product()
    }

    pub fn out_shape(self) -> [usize; 3] {
        match self {
            // #[cfg(feature = "v11n")]
            // Which::V11N => [1, 84, 8400],
            #[cfg(feature = "v11s")]
            Which::V11S => [1, 84, 8400],
            // #[cfg(feature = "v11m")]
            // Which::V11M => [1, 84, 8400],
            // #[cfg(feature = "v11l")]
            // Which::V11L => [1, 84, 8400],
            // #[cfg(feature = "v11x")]
            // Which::V11X => [1, 84, 8400],
        }
    }

    pub fn out_elems(self) -> usize {
        self.out_shape().into_iter().product()
    }

    pub fn onnx_data(self) -> &'static [u8] {
        match self {
            // #[cfg(feature = "v11n")]
            // Which::V11N => include_bytes!("../models/yolo11n.onnx"),
            #[cfg(feature = "v11s")]
            Which::V11S => include_bytes!("../models/perm/yolo11s-perm.onnx"),
            // #[cfg(feature = "v11m")]
            // Which::V11M => include_bytes!("../models/yolo11m.onnx"),
            // #[cfg(feature = "v11l")]
            // Which::V11L => include_bytes!("../models/yolo11l.onnx"),
            // #[cfg(feature = "v11x")]
            // Which::V11X => include_bytes!("../models/yolo11x.onnx"),
        }
    }

    pub fn plan_data(self) -> std::io::Result<Box<[u8]>> {
        let name = match self {
            // #[cfg(feature = "v11n")]
            // Which::V11N => "yolo11n",
            #[cfg(feature = "v11s")]
            Which::V11S => "yolo11s-perm",
            // #[cfg(feature = "v11m")]
            // Which::V11M => "yolo11m",
            // #[cfg(feature = "v11l")]
            // Which::V11L => "yolo11l",
            // #[cfg(feature = "v11x")]
            // Which::V11X => "yolo11x",
        };

        let plan_root = dirs::home_dir()
            .expect("home dir missing")
            .join(Path::new(&".cache/tensorrt-plans".to_string()));

        let plan_path = plan_root.join(Path::new(name)).with_extension("plan");

        if !plan_path.exists() {
            std::fs::create_dir_all(&plan_root)?;

            tensorrt::onnx_slice_to_plan(self.onnx_data()).save_to_file(&plan_path)?;
        }

        std::fs::read(&plan_path).map(From::from)
    }
}
