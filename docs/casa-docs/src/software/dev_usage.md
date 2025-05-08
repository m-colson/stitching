# Development Usage

Assuming your system has the [prequisites](./prereqs.md) setup, you can run the
software (to some extent) on any computer.

## Feature Flags

Rust/Cargo has a builtin concept of feature flags that can enable or disable
functionality and dependencies using a command line option. For the stitching_server
these include:
- `trt`: Enables TensorRT for object detection. If disabled, bounding boxes will
    not be shown. Should work on any system with an Nvidia Graphics Card.
- `v4l`: Enables Video4Linux as a camera source. Should allow live camera feeds
    on any linux machine.
- `argus`: Enable libargus as a camera source. Will only work on Jetson processors.
- `image`: Enables using static images as a camera source. Should work on any system.
- `jetson`: Enables the `trt`, `v4l`, and `argus` features.

## Running

As any example, you could build and serve the server with the `trt` and `v4l`
features enabled using:
```sh
cargo run --features trt,v4l -- serve
```