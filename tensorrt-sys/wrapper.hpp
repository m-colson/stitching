#include "trt-include-12.6/NvInfer.h"
#include "trt-include-12.6/NvOnnxParser.h"
#include "cuda_runtime.h"

namespace nvinfer1
{
    const int32_t TENSORRT_VERSION = NV_TENSORRT_VERSION;
    const int32_t ONNX_PARSER_VERSION = NV_ONNX_PARSER_VERSION;
}