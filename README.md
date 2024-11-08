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
Run server
```sh
cargo run -p stitching_server --release
```

Run in window
```sh
cargo run -p stitching_cli --release window
```

Generate GIF
```sh
cargo run -p stitching_cli --release gif
```

