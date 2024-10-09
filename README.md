# Stitching
Experimentation program for creating a 360 image from multiple cameras.

# Prerequisites
- [Rust Compiler](https://rustup.rs)
- cmake
- libclang

## Windows
```sh
winget install cmake llvm
```

# Building/Running
Run in window
```sh
cargo run --release
```

Generate GIF
```sh
cargo run --release --no-default-features
```

