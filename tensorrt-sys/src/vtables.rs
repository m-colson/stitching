use root::cudaEvent_t;

use cpp_interop::DestructorVEntry;

use crate::{bind::apiv::*, bind::*, root::cudaStream_t};

#[repr(C)]
pub struct ILoggerVTable {
    pub log: unsafe extern "C" fn(
        this: *mut ILogger,
        severity: ILogger_Severity,
        msg: *const core::ffi::c_char,
    ),
    pub destruct: DestructorVEntry<ILogger>,
}

#[repr(C)]
pub struct IBuilderVTable {
    pub destruct: DestructorVEntry<IBuilder>,
}

#[repr(C)]
pub struct VBuilderVTable {
    pub destruct: DestructorVEntry<VBuilder>,
    pub platform_has_fast_fp16: unsafe extern "C" fn(this: *mut VBuilder) -> bool,
    pub platform_has_fast_int8: unsafe extern "C" fn(this: *mut VBuilder) -> bool,
    pub get_max_dla_batch_size: unsafe extern "C" fn(this: *mut VBuilder) -> i32,
    pub get_nb_dlacores: unsafe extern "C" fn(this: *mut VBuilder) -> i32,
    pub set_gpu_allocator: unsafe extern "C" fn(this: *mut VBuilder, allocator: *mut IGpuAllocator),
    pub create_builder_config: unsafe extern "C" fn(this: *mut VBuilder) -> *mut IBuilderConfig,
    pub create_network_v2: unsafe extern "C" fn(
        this: *mut VBuilder,
        flags: NetworkDefinitionCreationFlags,
    ) -> *mut INetworkDefinition,
    pub create_optimization_profile:
        unsafe extern "C" fn(this: *mut VBuilder) -> *mut IOptimizationProfile,
    pub set_error_recorder:
        unsafe extern "C" fn(this: *mut VBuilder, recorder: *mut IErrorRecorder),
    pub get_error_recorder: unsafe extern "C" fn(this: *mut VBuilder) -> *mut IErrorRecorder,
    pub reset: unsafe extern "C" fn(this: *mut VBuilder),
    pub platform_has_tf32: unsafe extern "C" fn(this: *mut VBuilder) -> bool,
    pub build_serialized_network: unsafe extern "C" fn(
        this: *mut VBuilder,
        network: *const INetworkDefinition,
        config: *mut IBuilderConfig,
    ) -> *mut IHostMemory,
    pub is_network_supported: unsafe extern "C" fn(
        this: *mut VBuilder,
        network: *const INetworkDefinition,
        config: *const IBuilderConfig,
    ) -> bool,
    pub get_logger: unsafe extern "C" fn(this: *mut VBuilder) -> *mut ILogger,
    pub set_max_threads: unsafe extern "C" fn(this: *mut VBuilder, maxThreads: i32) -> bool,
    pub get_max_threads: unsafe extern "C" fn(this: *mut VBuilder) -> i32,
    pub get_plugin_registry: unsafe extern "C" fn(this: *mut VBuilder) -> *mut IPluginRegistry,
    pub build_engine_with_config: unsafe extern "C" fn(
        network: *mut INetworkDefinition,
        config: *mut IBuilderConfig,
    ) -> *mut ICudaEngine,
}

pub struct IBuilderConfigVTable {
    pub destruct: DestructorVEntry<IBuilderConfig>,
}

pub struct VBuilderConfigVTable {
    pub destruct: DestructorVEntry<VBuilderConfig>,
    pub set_avg_timing_iterations: unsafe extern "C" fn(this: *mut VBuilderConfig, avgTiming: i32),
    pub get_avg_timing_iterations: unsafe extern "C" fn(this: *mut VBuilderConfig) -> i32,
    pub set_engine_capability:
        unsafe extern "C" fn(this: *mut VBuilderConfig, capability: EngineCapability),
    pub get_engine_capability: unsafe extern "C" fn(this: *mut VBuilderConfig) -> EngineCapability,
    pub set_int8_calibrator:
        unsafe extern "C" fn(this: *mut VBuilderConfig, calibrator: *mut IInt8Calibrator),
    pub get_int8_calibrator:
        unsafe extern "C" fn(this: *mut VBuilderConfig) -> *mut IInt8Calibrator,
    pub set_flags: unsafe extern "C" fn(this: *mut VBuilderConfig, builderFlags: BuilderFlags),
    pub get_flags: unsafe extern "C" fn(this: *mut VBuilderConfig) -> BuilderFlags,
    pub clear_flag: unsafe extern "C" fn(this: *mut VBuilderConfig, builderFlag: BuilderFlag),
    pub set_flag: unsafe extern "C" fn(this: *mut VBuilderConfig, builderFlag: BuilderFlag),
    pub get_flag: unsafe extern "C" fn(this: *mut VBuilderConfig, builderFlag: BuilderFlag) -> bool,
    pub set_device_type: unsafe extern "C" fn(
        this: *mut VBuilderConfig,
        layer: *const ILayer,
        deviceType: DeviceType,
    ),
    pub get_device_type:
        unsafe extern "C" fn(this: *mut VBuilderConfig, layer: *const ILayer) -> DeviceType,
    pub is_device_type_set:
        unsafe extern "C" fn(this: *mut VBuilderConfig, layer: *const ILayer) -> bool,
    pub reset_device_type: unsafe extern "C" fn(this: *mut VBuilderConfig, layer: *const ILayer),
    pub can_run_on_dla:
        unsafe extern "C" fn(this: *mut VBuilderConfig, layer: *const ILayer) -> bool,
    pub set_dlacore: unsafe extern "C" fn(this: *mut VBuilderConfig, dlaCore: i32),
    pub get_dlacore: unsafe extern "C" fn(this: *mut VBuilderConfig) -> i32,
    pub set_default_device_type:
        unsafe extern "C" fn(this: *mut VBuilderConfig, deviceType: DeviceType),
    pub get_default_device_type: unsafe extern "C" fn(this: *mut VBuilderConfig) -> DeviceType,
    pub reset: unsafe extern "C" fn(this: *mut VBuilderConfig),
    pub set_profile_stream: unsafe extern "C" fn(this: *mut VBuilderConfig, stream: cudaStream_t),
    pub get_profile_stream: unsafe extern "C" fn(this: *mut VBuilderConfig) -> cudaStream_t,
    pub add_optimization_profile: unsafe extern "C" fn(
        this: *mut VBuilderConfig,
        profile: *const IOptimizationProfile,
    ) -> i32,
    pub get_nb_optimization_profiles: unsafe extern "C" fn(this: *mut VBuilderConfig) -> i32,
    pub set_profiling_verbosity:
        unsafe extern "C" fn(this: *mut VBuilderConfig, verbosity: ProfilingVerbosity),
    pub get_profiling_verbosity:
        unsafe extern "C" fn(this: *mut VBuilderConfig) -> ProfilingVerbosity,
    pub set_algorithm_selector:
        unsafe extern "C" fn(this: *mut VBuilderConfig, selector: *mut IAlgorithmSelector),
    pub get_algorithm_selector:
        unsafe extern "C" fn(this: *mut VBuilderConfig) -> *mut IAlgorithmSelector,
    pub set_calibration_profile: unsafe extern "C" fn(
        this: *mut VBuilderConfig,
        profile: *const IOptimizationProfile,
    ) -> bool,
    pub get_calibration_profile:
        unsafe extern "C" fn(this: *mut VBuilderConfig) -> *const IOptimizationProfile,
    pub set_quantization_flags:
        unsafe extern "C" fn(this: *mut VBuilderConfig, flags: QuantizationFlags),
    pub get_quantization_flags:
        unsafe extern "C" fn(this: *mut VBuilderConfig) -> QuantizationFlags,
    pub clear_quantization_flag:
        unsafe extern "C" fn(this: *mut VBuilderConfig, flag: QuantizationFlag),
    pub set_quantization_flag:
        unsafe extern "C" fn(this: *mut VBuilderConfig, flag: QuantizationFlag),
    pub get_quantization_flag:
        unsafe extern "C" fn(this: *mut VBuilderConfig, flag: QuantizationFlag) -> bool,
    pub set_tactic_sources:
        unsafe extern "C" fn(this: *mut VBuilderConfig, tacticSources: TacticSources) -> bool,
    pub get_tactic_sources: unsafe extern "C" fn(this: *mut VBuilderConfig) -> TacticSources,
    pub create_timing_cache: unsafe extern "C" fn(
        this: *mut VBuilderConfig,
        blob: *const core::ffi::c_void,
        size: usize,
    ) -> *mut ITimingCache,
    pub set_timing_cache: unsafe extern "C" fn(
        this: *mut VBuilderConfig,
        cache: *const ITimingCache,
        ignoreMismatch: bool,
    ) -> bool,
    pub get_timing_cache: unsafe extern "C" fn(this: *mut VBuilderConfig) -> *const ITimingCache,
    pub set_memory_pool_limit:
        unsafe extern "C" fn(this: *mut VBuilderConfig, pool: MemoryPoolType, poolSize: usize),
    pub get_memory_pool_limit:
        unsafe extern "C" fn(this: *mut VBuilderConfig, pool: MemoryPoolType) -> usize,
    pub set_preview_feature:
        unsafe extern "C" fn(this: *mut VBuilderConfig, feature: PreviewFeature, enable: bool),
    pub get_preview_feature:
        unsafe extern "C" fn(this: *mut VBuilderConfig, feature: PreviewFeature) -> bool,
    pub set_builder_optimization_level: unsafe extern "C" fn(this: *mut VBuilderConfig, level: i32),
    pub get_builder_optimization_level: unsafe extern "C" fn(this: *mut VBuilderConfig) -> i32,
    pub set_hardware_compatibility_level: unsafe extern "C" fn(
        this: *mut VBuilderConfig,
        hardwareCompatibilityLevel: HardwareCompatibilityLevel,
    ),
    pub get_hardware_compatibility_level:
        unsafe extern "C" fn(this: *mut VBuilderConfig) -> HardwareCompatibilityLevel,
    pub set_plugins_to_serialize: unsafe extern "C" fn(
        this: *mut VBuilderConfig,
        paths: *const *const core::ffi::c_char,
        nbPaths: i32,
    ),
    pub get_plugin_to_serialize:
        unsafe extern "C" fn(this: *mut VBuilderConfig, index: i32) -> *const core::ffi::c_char,
    pub get_nb_plugins_to_serialize: unsafe extern "C" fn(this: *mut VBuilderConfig) -> i32,
    pub set_max_aux_streams: unsafe extern "C" fn(this: *mut VBuilderConfig, nbStreams: i32),
    pub get_max_aux_streams: unsafe extern "C" fn(this: *mut VBuilderConfig) -> i32,
    pub set_progress_monitor:
        unsafe extern "C" fn(this: *mut VBuilderConfig, monitor: *mut IProgressMonitor),
    pub get_progress_monitor:
        unsafe extern "C" fn(this: *mut VBuilderConfig) -> *mut IProgressMonitor,
    pub set_runtime_platform:
        unsafe extern "C" fn(this: *mut VBuilderConfig, runtimePlatform: RuntimePlatform),
    pub get_runtime_platform: unsafe extern "C" fn(this: *mut VBuilderConfig) -> RuntimePlatform,
    pub set_max_nb_tactics: unsafe extern "C" fn(this: *mut VBuilderConfig, maxTactics: i32),
    pub get_max_nb_tactics: unsafe extern "C" fn(this: *mut VBuilderConfig) -> i32,
}

#[repr(C)]
pub struct INetworkDefinitionVTable {
    pub destruct: DestructorVEntry<INetworkDefinition>,
}

#[repr(C)]
pub struct VNetworkDefinitionVTable {
    pub destruct: DestructorVEntry<VNetworkDefinition>,
    pub add_input: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        name: *const core::ffi::c_char,
        r#type: DataType,
        dimensions: *const Dims,
    ) -> *mut ITensor,
    pub mark_output: unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *mut ITensor),
    pub add_activation: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        r#type: ActivationType,
    ) -> *mut IActivationLayer,
    pub add_lrn: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        window: i64,
        alpha: f32,
        beta: f32,
        k: f32,
    ) -> *mut ILRNLayer,
    pub add_scale: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        mode: ScaleMode,
        shift: Weights,
        scale: Weights,
        power: Weights,
    ) -> *mut IScaleLayer,
    pub add_soft_max: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
    ) -> *mut ISoftMaxLayer,
    pub add_concatenation: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        inputs: *mut *const ITensor,
        nbInputs: i32,
    ) -> *mut IConcatenationLayer,
    pub add_element_wise: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input1: *mut ITensor,
        input2: *mut ITensor,
        op: ElementWiseOperation,
    ) -> *mut IElementWiseLayer,
    pub add_unary: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        operation: UnaryOperation,
    ) -> *mut IUnaryLayer,
    pub add_shuffle: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
    ) -> *mut IShuffleLayer,
    pub get_nb_layers: unsafe extern "C" fn(this: *mut VNetworkDefinition) -> i32,
    pub get_layer: unsafe extern "C" fn(this: *mut VNetworkDefinition, index: i32) -> *mut ILayer,
    pub get_nb_inputs: unsafe extern "C" fn(this: *mut VNetworkDefinition) -> i32,
    pub get_input: unsafe extern "C" fn(this: *mut VNetworkDefinition, index: i32) -> *mut ITensor,
    pub get_nb_outputs: unsafe extern "C" fn(this: *mut VNetworkDefinition) -> i32,
    pub get_output: unsafe extern "C" fn(this: *mut VNetworkDefinition, index: i32) -> *mut ITensor,
    pub add_reduce: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        operation: ReduceOperation,
        reduceAxes: u32,
        keepDimensions: bool,
    ) -> *mut IReduceLayer,
    pub add_top_k: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        op: TopKOperation,
        k: i32,
        reduceAxes: u32,
    ) -> *mut ITopKLayer,
    pub add_gather: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        data: *mut ITensor,
        indices: *mut ITensor,
        axis: i32,
    ) -> *mut IGatherLayer,
    pub add_ragged_soft_max: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        bounds: *mut ITensor,
    ) -> *mut IRaggedSoftMaxLayer,
    pub add_matrix_multiply: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input0: *mut ITensor,
        op0: MatrixOperation,
        input1: *mut ITensor,
        op1: MatrixOperation,
    ) -> *mut IMatrixMultiplyLayer,
    pub add_constant: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        dimensions: *const Dims,
        weights: Weights,
    ) -> *mut IConstantLayer,
    pub add_identity: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
    ) -> *mut IIdentityLayer,
    pub remove_tensor: unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *mut ITensor),
    pub unmark_output: unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *mut ITensor),
    pub add_plugin_v2: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        inputs: *mut *const ITensor,
        nbInputs: i32,
        plugin: *mut IPluginV2,
    ) -> *mut IPluginV2Layer,
    pub add_plugin_v3: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        inputs: *mut *const ITensor,
        nbInputs: i32,
        shapeInputs: *mut *const ITensor,
        nbShapeInputs: i32,
        plugin: *mut IPluginV3,
    ) -> *mut IPluginV3Layer,
    pub add_slice: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        start: *const Dims,
        size: *const Dims,
        stride: *const Dims,
    ) -> *mut ISliceLayer,
    pub set_name:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, name: *const core::ffi::c_char),
    pub get_name: unsafe extern "C" fn(this: *mut VNetworkDefinition) -> *const core::ffi::c_char,
    pub add_shape: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
    ) -> *mut IShapeLayer,
    pub has_implicit_batch_dimension: unsafe extern "C" fn(this: *mut VNetworkDefinition) -> bool,
    pub mark_output_for_shapes:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *mut ITensor) -> bool,
    pub unmark_output_for_shapes:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *mut ITensor) -> bool,
    pub add_parametric_re_lu: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        slope: *mut ITensor,
    ) -> *mut IParametricReLULayer,
    pub add_convolution_nd: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        nbOutputMaps: i64,
        kernelSize: *const Dims,
        kernelWeights: Weights,
        biasWeights: Weights,
    ) -> *mut IConvolutionLayer,
    pub add_pooling_nd: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        r#type: PoolingType,
        windowSize: *const Dims,
    ) -> *mut IPoolingLayer,
    pub add_deconvolution_nd: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        nbOutputMaps: i64,
        kernelSize: *const Dims,
        kernelWeights: Weights,
        biasWeights: Weights,
    ) -> *mut IDeconvolutionLayer,
    pub add_scale_nd: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        mode: ScaleMode,
        shift: Weights,
        scale: Weights,
        power: Weights,
        channelAxis: i32,
    ) -> *mut IScaleLayer,
    pub add_resize: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
    ) -> *mut IResizeLayer,
    pub add_loop: unsafe extern "C" fn(this: *mut VNetworkDefinition) -> *mut ILoop,
    pub add_select: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        condition: *mut ITensor,
        thenInput: *mut ITensor,
        elseInput: *mut ITensor,
    ) -> *mut ISelectLayer,
    pub add_fill: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        dimensions: *const Dims,
        op: FillOperation,
    ) -> *mut IFillLayer,
    pub add_padding_nd: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        prePadding: *const Dims,
        postPadding: *const Dims,
    ) -> *mut IPaddingLayer,
    pub set_weights_name: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        weights: Weights,
        name: *const core::ffi::c_char,
    ) -> bool,
    pub set_error_recorder:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, recorder: *mut IErrorRecorder),
    pub get_error_recorder:
        unsafe extern "C" fn(this: *mut VNetworkDefinition) -> *mut IErrorRecorder,
    pub add_dequantize: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        scale: *mut ITensor,
    ) -> *mut IDequantizeLayer,
    pub add_quantize: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        scale: *mut ITensor,
    ) -> *mut IQuantizeLayer,
    pub add_gather_v2: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        data: *mut ITensor,
        indices: *mut ITensor,
        mode: GatherMode,
    ) -> *mut IGatherLayer,
    pub add_if_conditional:
        unsafe extern "C" fn(this: *mut VNetworkDefinition) -> *mut IIfConditional,
    pub add_scatter: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        data: *mut ITensor,
        indices: *mut ITensor,
        updates: *mut ITensor,
        mode: ScatterMode,
    ) -> *mut IScatterLayer,
    pub add_einsum: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        inputs: *mut *const ITensor,
        nbInputs: i32,
        equation: *const core::ffi::c_char,
    ) -> *mut IEinsumLayer,
    pub add_assertion: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        condition: *mut ITensor,
        message: *const core::ffi::c_char,
    ) -> *mut IAssertionLayer,
    pub add_one_hot: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        indices: *mut ITensor,
        values: *mut ITensor,
        depth: *mut ITensor,
        axis: i32,
    ) -> *mut IOneHotLayer,
    pub add_non_zero: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
    ) -> *mut INonZeroLayer,
    pub add_grid_sample: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        grid: *mut ITensor,
    ) -> *mut IGridSampleLayer,
    pub add_nms: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        boxes: *mut ITensor,
        scores: *mut ITensor,
        maxOutputBoxesPerClass: *mut ITensor,
    ) -> *mut INMSLayer,
    pub add_reverse_sequence: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        sequenceLens: *mut ITensor,
    ) -> *mut IReverseSequenceLayer,
    pub add_normalization: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        scale: *mut ITensor,
        bias: *mut ITensor,
        axesMask: u32,
    ) -> *mut INormalizationLayer,
    pub add_cast: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        toType: DataType,
    ) -> *mut ICastLayer,
    pub get_builder: unsafe extern "C" fn(this: *mut VNetworkDefinition) -> *mut IBuilder,
    pub get_flags:
        unsafe extern "C" fn(this: *mut VNetworkDefinition) -> NetworkDefinitionCreationFlags,
    pub get_flag: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        networkDefinitionCreationFlag: NetworkDefinitionCreationFlag,
    ) -> bool,
    pub add_quantize_v2: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        scale: *mut ITensor,
        outputType: DataType,
    ) -> *mut IQuantizeLayer,
    pub add_dequantize_v2: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        scale: *mut ITensor,
        outputType: DataType,
    ) -> *mut IDequantizeLayer,
    pub add_fill_v2: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        dimensions: *const Dims,
        op: FillOperation,
        outputType: DataType,
    ) -> *mut IFillLayer,
    pub mark_debug:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *mut ITensor) -> bool,
    pub unmark_debug:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *mut ITensor) -> bool,
    pub is_debug_tensor:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, tensor: *const ITensor) -> bool,
    pub mark_weights_refittable:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, name: *const core::ffi::c_char) -> bool,
    pub unmark_weights_refittable:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, name: *const core::ffi::c_char) -> bool,
    pub are_weights_marked_refittable:
        unsafe extern "C" fn(this: *mut VNetworkDefinition, name: *const core::ffi::c_char) -> bool,
    pub add_squeeze: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        axes: *mut ITensor,
    ) -> *mut ISqueezeLayer,
    pub add_unsqueeze: unsafe extern "C" fn(
        this: *mut VNetworkDefinition,
        input: *mut ITensor,
        axes: *mut ITensor,
    ) -> *mut IUnsqueezeLayer,
}

pub struct IHostMemoryVTable {
    pub destruct: DestructorVEntry<IHostMemory>,
}

pub struct VHostMemoryVTable {
    pub destruct: DestructorVEntry<VHostMemory>,
    pub data: unsafe extern "C" fn(this: *const VHostMemory) -> *mut core::ffi::c_void,
    pub size: unsafe extern "C" fn(this: *const VHostMemory) -> usize,
    // `type` is the name in header
    pub data_type: unsafe extern "C" fn(this: *const VHostMemory) -> DataType,
}

pub struct IRuntimeVTable {
    pub destruct: DestructorVEntry<IRuntime>,
}

#[cfg(target_os = "windows")]
pub struct VRuntimeVTable {
    pub destruct: DestructorVEntry<VRuntime>,
    pub get_pimpl: unsafe extern "C" fn(this: *mut VRuntime) -> *mut IRuntime,
    // NOTE: deserializeCudaEngine methods have order reversed in windows
    pub deserialize_cuda_engine_reader: unsafe extern "C" fn(
        this: *mut VRuntime,
        streamReader: *mut IStreamReader,
    ) -> *mut ICudaEngine,
    pub deserialize_cuda_engine_blob: unsafe extern "C" fn(
        this: *mut VRuntime,
        blob: *const core::ffi::c_void,
        size: usize,
    ) -> *mut ICudaEngine,
    pub set_dlacore: unsafe extern "C" fn(this: *mut VRuntime, dlaCore: i32),
    pub get_dlacore: unsafe extern "C" fn(this: *mut VRuntime) -> i32,
    pub get_nb_dlacores: unsafe extern "C" fn(this: *mut VRuntime) -> i32,
    pub set_gpu_allocator: unsafe extern "C" fn(this: *mut VRuntime, allocator: *mut IGpuAllocator),
    pub set_error_recorder:
        unsafe extern "C" fn(this: *mut VRuntime, recorder: *mut IErrorRecorder),
    pub get_error_recorder: unsafe extern "C" fn(this: *mut VRuntime) -> *mut IErrorRecorder,
    pub get_logger: unsafe extern "C" fn(this: *mut VRuntime) -> *mut ILogger,
    pub set_max_threads: unsafe extern "C" fn(this: *mut VRuntime, maxThreads: i32) -> bool,
    pub get_max_threads: unsafe extern "C" fn(this: *mut VRuntime) -> i32,
    pub set_temporary_directory:
        unsafe extern "C" fn(this: *mut VRuntime, path: *const core::ffi::c_char),
    pub get_temporary_directory:
        unsafe extern "C" fn(this: *mut VRuntime) -> *const core::ffi::c_char,
    pub set_tempfile_control_flags: unsafe extern "C" fn(this: *mut VRuntime, TempfileControlFlags),
    pub get_tempfile_control_flags:
        unsafe extern "C" fn(this: *mut VRuntime) -> TempfileControlFlags,
    pub get_plugin_registry: unsafe extern "C" fn(this: *mut VRuntime) -> *mut IPluginRegistry,
    pub set_plugin_registry_parent:
        unsafe extern "C" fn(this: *mut VRuntime, parent: *mut IPluginRegistry),
    pub load_runtime: unsafe extern "C" fn(this: *mut VRuntime, path: *const u8) -> *mut IRuntime,
    pub set_engine_host_code_allowed: unsafe extern "C" fn(this: *mut VRuntime, allowed: bool),
    pub get_engine_host_code_allowed: unsafe extern "C" fn(this: *mut VRuntime) -> bool,
    // Added in TensorRT version 10.7
    pub deserialize_cuda_engine_v2: unsafe extern "C" fn(
        this: *mut VRuntime,
        streamReader: *mut IStreamReaderV2,
    ) -> *mut ICudaEngine,
}

#[cfg(target_os = "linux")]
pub struct VRuntimeVTable {
    pub destruct: DestructorVEntry<VRuntime>,
    pub get_pimpl: unsafe extern "C" fn(this: *mut VRuntime) -> *mut IRuntime,
    pub deserialize_cuda_engine_blob: unsafe extern "C" fn(
        this: *mut VRuntime,
        blob: *const core::ffi::c_void,
        size: usize,
    ) -> *mut ICudaEngine,
    pub deserialize_cuda_engine_reader: unsafe extern "C" fn(
        this: *mut VRuntime,
        streamReader: *mut IStreamReader,
    ) -> *mut ICudaEngine,
    pub set_dlacore: unsafe extern "C" fn(this: *mut VRuntime, dlaCore: i32),
    pub get_dlacore: unsafe extern "C" fn(this: *mut VRuntime) -> i32,
    pub get_nb_dlacores: unsafe extern "C" fn(this: *mut VRuntime) -> i32,
    pub set_gpu_allocator: unsafe extern "C" fn(this: *mut VRuntime, allocator: *mut IGpuAllocator),
    pub set_error_recorder:
        unsafe extern "C" fn(this: *mut VRuntime, recorder: *mut IErrorRecorder),
    pub get_error_recorder: unsafe extern "C" fn(this: *mut VRuntime) -> *mut IErrorRecorder,
    pub get_logger: unsafe extern "C" fn(this: *mut VRuntime) -> *mut ILogger,
    pub set_max_threads: unsafe extern "C" fn(this: *mut VRuntime, maxThreads: i32) -> bool,
    pub get_max_threads: unsafe extern "C" fn(this: *mut VRuntime) -> i32,
    pub set_temporary_directory:
        unsafe extern "C" fn(this: *mut VRuntime, path: *const core::ffi::c_char),
    pub get_temporary_directory:
        unsafe extern "C" fn(this: *mut VRuntime) -> *const core::ffi::c_char,
    pub set_tempfile_control_flags: unsafe extern "C" fn(this: *mut VRuntime, TempfileControlFlags),
    pub get_tempfile_control_flags:
        unsafe extern "C" fn(this: *mut VRuntime) -> TempfileControlFlags,
    pub get_plugin_registry: unsafe extern "C" fn(this: *mut VRuntime) -> *mut IPluginRegistry,
    pub set_plugin_registry_parent:
        unsafe extern "C" fn(this: *mut VRuntime, parent: *mut IPluginRegistry),
    pub load_runtime: unsafe extern "C" fn(this: *mut VRuntime, path: *const u8) -> *mut IRuntime,
    pub set_engine_host_code_allowed: unsafe extern "C" fn(this: *mut VRuntime, allowed: bool),
    pub get_engine_host_code_allowed: unsafe extern "C" fn(this: *mut VRuntime) -> bool,
    // Added in TensorRT version 10.7
    pub deserialize_cuda_engine_v2: unsafe extern "C" fn(
        this: *mut VRuntime,
        streamReader: *mut IStreamReaderV2,
    ) -> *mut ICudaEngine,
}

pub struct ICudaEngineVTable {
    pub destruct: DestructorVEntry<ICudaEngine>,
}

pub struct VCudaEngineVTable {
    pub destruct: DestructorVEntry<VCudaEngine>,
    pub get_pimpl: unsafe extern "C" fn(this: *mut VCudaEngine) -> *mut ICudaEngine,
    pub get_nb_layers: unsafe extern "C" fn(this: *mut VCudaEngine) -> i32,
    pub serialize: unsafe extern "C" fn(this: *mut VCudaEngine) -> *mut IHostMemory,
    pub create_execution_context: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        strategy: ExecutionContextAllocationStrategy,
    ) -> *mut IExecutionContext,
    pub create_execution_context_without_device_memory:
        unsafe extern "C" fn(this: *mut VCudaEngine) -> *mut IExecutionContext,
    pub get_device_memory_size: unsafe extern "C" fn(this: *mut VCudaEngine) -> usize,
    pub is_refittable: unsafe extern "C" fn(this: *mut VCudaEngine) -> bool,
    pub get_name: unsafe extern "C" fn(this: *mut VCudaEngine) -> *const core::ffi::c_char,
    pub get_nb_optimization_profiles: unsafe extern "C" fn(this: *mut VCudaEngine) -> i32,
    pub get_profile_tensor_values: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
        profileIndex: i32,
        select: OptProfileSelector,
    ) -> *const i32,
    pub get_engine_capability: unsafe extern "C" fn(this: *mut VCudaEngine) -> EngineCapability,
    pub set_error_recorder:
        unsafe extern "C" fn(this: *mut VCudaEngine, recorder: *mut IErrorRecorder),
    pub get_error_recorder: unsafe extern "C" fn(this: *mut VCudaEngine) -> *mut IErrorRecorder,
    pub has_implicit_batch_dimension: unsafe extern "C" fn(this: *mut VCudaEngine) -> bool,
    pub get_tactic_sources: unsafe extern "C" fn(this: *mut VCudaEngine) -> TacticSources,
    pub get_profiling_verbosity: unsafe extern "C" fn(this: *mut VCudaEngine) -> ProfilingVerbosity,
    pub create_engine_inspector:
        unsafe extern "C" fn(this: *mut VCudaEngine) -> *mut IEngineInspector,
    pub get_tensor_shape:
        unsafe extern "C" fn(this: *mut VCudaEngine, tensorName: *const core::ffi::c_char) -> Dims,
    pub get_tensor_data_type: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
    ) -> DataType,
    pub get_tensor_location: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
    ) -> TensorLocation,
    pub is_shape_inference_io:
        unsafe extern "C" fn(this: *mut VCudaEngine, tensorName: *const core::ffi::c_char) -> bool,
    pub get_tensor_iomode: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
    ) -> TensorIOMode,
    pub get_tensor_bytes_per_component:
        unsafe extern "C" fn(this: *mut VCudaEngine, tensorName: *const core::ffi::c_char) -> i32,
    pub get_tensor_components_per_element:
        unsafe extern "C" fn(this: *mut VCudaEngine, tensorName: *const core::ffi::c_char) -> i32,
    pub get_tensor_format: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
    ) -> TensorFormat,
    pub get_tensor_format_desc: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
    ) -> *const core::ffi::c_char,
    pub get_tensor_vectorized_dim:
        unsafe extern "C" fn(this: *mut VCudaEngine, tensorName: *const core::ffi::c_char) -> i32,
    pub get_profile_shape: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
        profileIndex: i32,
        select: OptProfileSelector,
    ) -> Dims,
    pub get_nb_iotensors: unsafe extern "C" fn(this: *mut VCudaEngine) -> i32,
    pub get_iotensor_name:
        unsafe extern "C" fn(this: *mut VCudaEngine, index: i32) -> *const core::ffi::c_char,
    pub get_hardware_compatibility_level:
        unsafe extern "C" fn(this: *mut VCudaEngine) -> HardwareCompatibilityLevel,
    pub get_nb_aux_streams: unsafe extern "C" fn(this: *mut VCudaEngine) -> i32,

    pub get_tensor_bytes_per_component_v2: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
        profileIndex: i32,
    ) -> i32,
    pub get_tensor_components_per_element_v2: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
        profileIndex: i32,
    ) -> i32,
    pub get_tensor_format_v2: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
        profileIndex: i32,
    ) -> TensorFormat,
    pub get_tensor_format_desc_v2: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
        profileIndex: i32,
    ) -> *const core::ffi::c_char,
    pub get_tensor_vectorized_dim_v2: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        tensorName: *const core::ffi::c_char,
        profileIndex: i32,
    ) -> i32,

    pub create_serialization_config:
        unsafe extern "C" fn(this: *mut VCudaEngine) -> *mut ISerializationConfig,
    pub serialize_with_config: unsafe extern "C" fn(
        this: *mut VCudaEngine,
        config: *mut ISerializationConfig,
    ) -> *mut IHostMemory,

    pub get_device_memory_size_for_profile:
        unsafe extern "C" fn(this: *mut VCudaEngine, profileIndex: i32) -> usize,
    pub create_refitter:
        unsafe extern "C" fn(this: *mut VCudaEngine, logger: *mut ILogger) -> *mut IRefitter,

    pub set_weight_streaming_budget:
        unsafe extern "C" fn(this: *mut VCudaEngine, gpuMemoryBudget: i64) -> bool,
    pub get_weight_streaming_budget: unsafe extern "C" fn(this: *mut VCudaEngine) -> i64,
    pub get_minimum_weight_streaming_budget: unsafe extern "C" fn(this: *mut VCudaEngine) -> i64,
    pub get_streamable_weights_size: unsafe extern "C" fn(this: *mut VCudaEngine) -> i64,

    pub is_debug_tensor:
        unsafe extern "C" fn(this: *mut VCudaEngine, name: *const core::ffi::c_char) -> bool,

    // Added in TensorRT 10.1,
    pub set_weight_streaming_budget_v2:
        unsafe extern "C" fn(this: *mut VCudaEngine, gpuMemoryBudget: i64) -> bool,
    pub get_weight_streaming_budget_v2: unsafe extern "C" fn(this: *mut VCudaEngine) -> i64,
    pub get_weight_streaming_automatic_budget: unsafe extern "C" fn(this: *mut VCudaEngine) -> i64,
    pub get_weight_streaming_scratch_memory_size:
        unsafe extern "C" fn(this: *mut VCudaEngine) -> i64,
    pub get_device_memory_size_v2: unsafe extern "C" fn(this: *mut VCudaEngine) -> i64,
    pub get_device_memory_size_for_profile_v2:
        unsafe extern "C" fn(this: *mut VCudaEngine, profileIndex: i32) -> i64,
}

pub struct IExecutionContextVTable {
    pub destruct: DestructorVEntry<IExecutionContext>,
}

pub struct VExecutionContextVTable {
    pub destruct: DestructorVEntry<VExecutionContext>,
    pub get_pimpl: unsafe extern "C" fn(this: *mut VExecutionContext) -> *mut IExecutionContext,
    pub set_debug_sync: unsafe extern "C" fn(this: *mut VExecutionContext, sync: bool),
    pub get_debug_sync: unsafe extern "C" fn(this: *mut VExecutionContext) -> bool,
    pub set_profiler: unsafe extern "C" fn(this: *mut VExecutionContext, profiler: *mut IProfiler),
    pub get_profiler: unsafe extern "C" fn(this: *mut VExecutionContext) -> *mut IProfiler,
    pub get_engine: unsafe extern "C" fn(this: *mut VExecutionContext) -> *const ICudaEngine,
    pub set_name:
        unsafe extern "C" fn(this: *mut VExecutionContext, name: *const core::ffi::c_char),
    pub get_name: unsafe extern "C" fn(this: *mut VExecutionContext) -> *const core::ffi::c_char,
    pub set_device_memory:
        unsafe extern "C" fn(this: *mut VExecutionContext, memory: *mut core::ffi::c_void),
    pub get_optimization_profile: unsafe extern "C" fn(this: *mut VExecutionContext) -> i32,
    pub all_input_dimensions_specified: unsafe extern "C" fn(this: *mut VExecutionContext) -> bool,
    pub all_input_shapes_specified: unsafe extern "C" fn(this: *mut VExecutionContext) -> bool,
    pub set_error_recorder:
        unsafe extern "C" fn(this: *mut VExecutionContext, recorder: *mut IErrorRecorder),
    pub get_error_recorder:
        unsafe extern "C" fn(this: *mut VExecutionContext) -> *mut IErrorRecorder,
    pub execute_v2: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        bindings: *const *mut core::ffi::c_void,
    ) -> bool,
    pub set_optimization_profile_async: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        profileIndex: i32,
        stream: cudaStream_t,
    ) -> bool,
    pub set_enqueue_emits_profile:
        unsafe extern "C" fn(this: *mut VExecutionContext, enqueueEmitsProfile: bool),
    pub get_enqueue_emits_profile: unsafe extern "C" fn(this: *mut VExecutionContext) -> bool,
    pub report_to_profiler: unsafe extern "C" fn(this: *mut VExecutionContext) -> bool,
    pub set_input_shape: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
        dims: *const Dims,
    ) -> bool,
    pub get_tensor_shape: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
    ) -> Dims,
    pub get_tensor_strides: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
    ) -> Dims,
    pub set_tensor_address: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
        data: *mut core::ffi::c_void,
    ) -> bool,
    pub get_tensor_address: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
    ) -> *const core::ffi::c_void,
    pub set_input_tensor_address: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
        data: *const core::ffi::c_void,
    ) -> bool,
    pub set_output_tensor_address: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
        data: *mut core::ffi::c_void,
    ) -> bool,
    pub infer_shapes: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        nbMaxNames: i32,
        tensorNames: *mut *const core::ffi::c_char,
    ) -> i32,
    pub set_input_consumed_event:
        unsafe extern "C" fn(this: *mut VExecutionContext, event: cudaEvent_t) -> bool,
    pub get_input_consumed_event: unsafe extern "C" fn(this: *mut VExecutionContext) -> cudaEvent_t,
    pub get_output_tensor_address: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
    ) -> *mut core::ffi::c_void,
    pub set_output_allocator: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
        outputAllocator: *mut IOutputAllocator,
    ) -> bool,
    pub get_output_allocator: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        name: *const core::ffi::c_char,
    ) -> *mut IOutputAllocator,
    pub get_max_output_size: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        tensorName: *const core::ffi::c_char,
    ) -> i64,
    pub set_temporary_storage_allocator:
        unsafe extern "C" fn(this: *mut VExecutionContext, allocator: *mut IGpuAllocator) -> bool,
    pub get_temporary_storage_allocator:
        unsafe extern "C" fn(this: *mut VExecutionContext) -> *mut IGpuAllocator,
    pub enqueue_v3:
        unsafe extern "C" fn(this: *mut VExecutionContext, stream: cudaStream_t) -> bool,
    pub set_persistent_cache_limit: unsafe extern "C" fn(this: *mut VExecutionContext, size: usize),
    pub get_persistent_cache_limit: unsafe extern "C" fn(this: *mut VExecutionContext) -> usize,
    pub set_nvtx_verbosity:
        unsafe extern "C" fn(this: *mut VExecutionContext, verbosity: ProfilingVerbosity) -> bool,
    pub get_nvtx_verbosity:
        unsafe extern "C" fn(this: *mut VExecutionContext) -> ProfilingVerbosity,
    pub set_aux_streams: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        auxStreams: *mut cudaStream_t,
        nbStreams: i32,
    ),
    pub set_debug_listener:
        unsafe extern "C" fn(this: *mut VExecutionContext, listener: *mut IDebugListener) -> bool,
    pub get_debug_listener:
        unsafe extern "C" fn(this: *mut VExecutionContext) -> *mut IDebugListener,
    pub set_tensor_debug_state: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        name: *const core::ffi::c_char,
        flag: bool,
    ) -> bool,
    pub get_debug_state:
        unsafe extern "C" fn(this: *mut VExecutionContext, name: *const core::ffi::c_char) -> bool,
    pub set_all_tensors_debug_state:
        unsafe extern "C" fn(this: *mut VExecutionContext, flag: bool) -> bool,
    pub update_device_memory_size_for_shapes:
        unsafe extern "C" fn(this: *mut VExecutionContext) -> usize,

    // Added in TensorRT 10.1
    pub set_device_memory_v2: unsafe extern "C" fn(
        this: *mut VExecutionContext,
        memory: *mut core::ffi::c_void,
        size: i64,
    ),
}
