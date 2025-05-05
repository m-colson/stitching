//! This crate makes it simple to use a [YOLO](https://docs.ultralytics.com/models/yolo11/) model with [`tensorrt`].
//!
//! Start by looking at the [`Which`] enum.

#![warn(missing_docs)]

use std::path::Path;
use strum::VariantArray;

pub use tensorrt::{CudaBuffer, CudaError, CudaStream, RuntimeEngineContext};

pub mod boxes;
pub mod coco;

pub use boxes::{nms_to_bounding, BoundingClass};

/// This represents the kind of YOLO model you want to use. Since the ONNX file
/// for the model will be included in the build, which could be large, you can enable or disable
/// different variants using the features flags of this crate.
#[derive(Clone, Copy, Debug, VariantArray)]
pub enum Which {
    // #[cfg(feature = "v11n")]
    // V11N,
    /// YOLO V11 Small
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
    /// Returns the fastest variant of [`Which`] based on the enabled feature flags.
    pub fn fastest() -> Self {
        *Self::VARIANTS
            .first()
            .expect("no models added to this binary")
    }

    /// Returns the best variant of [`Which`] based on the enabled feature flags.
    pub fn best() -> Self {
        *Self::VARIANTS
            .last()
            .expect("no models added to this binary")
    }

    /// Returns the input tensor shape for the `self`.
    pub fn input_shape(self) -> [usize; 4] {
        match self {
            // #[cfg(feature = "v11n")]
            // Which::V11N => [1, 640, 640, 3],
            #[cfg(feature = "v11s")]
            Which::V11S => [1, 640, 640, 4],
            // #[cfg(feature = "v11m")]
            // Which::V11M => [1, 640, 640, 3],
            // #[cfg(feature = "v11l")]
            // Which::V11L => [1, 640, 640, 3],
            // #[cfg(feature = "v11x")]
            // Which::V11X => [1, 640, 640, 3],
        }
    }

    /// Returns the total number of elements in the input tensor for `self`.
    pub fn input_elems(self) -> usize {
        self.input_shape().into_iter().product()
    }

    /// Returns the output tensor shape for `self`.
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

    /// Returns the total number of elements in the output tensor for `self`.
    pub fn out_elems(self) -> usize {
        self.out_shape().into_iter().product()
    }

    /// Returns a static included byte slice containing the ONNX-based
    /// description of `self`.
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

    /// Returns a new boxed byte slice containing the TensorRT plan for `self`.
    ///
    /// These plans are architecture specific so they must be generated on the
    /// target machine. This function will check `$HOME/.cache/tensorrt-plans`
    /// for a plan with the correct name and return that if found. Otherwise, the
    /// plan will be generated, which takes a decently long time, cached and
    /// returned.
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
            #[cfg(feature = "tracing")]
            tracing::info!("missing plan for {name}, building...");
            std::fs::create_dir_all(&plan_root)?;

            tensorrt::onnx_slice_to_plan(self.onnx_data()).save_to_file(&plan_path)?;
        }

        std::fs::read(&plan_path).map(From::from)
    }
}
