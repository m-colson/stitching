# Software Setup

## Preparing Build Environment
Install the rust compiler and build tool with [rustup](https://rustup.rs).
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installing
From within the directory with all of the source files.
```sh
cargo install -p stitching_server --features jetson --path .
```