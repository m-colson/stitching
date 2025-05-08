# Software Setup

## Preparing Build Environment
Install the rust compiler and build tool with [rustup](https://rustup.rs).
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installing
Save/unzip the source into a `/home/casa/casa-src` directory. Then navigate 
your terminal to it with:
```sh
cd /home/casa/casa-src
```

Now to compile and install the binary onto the system run:
```sh
cargo install --path stitching_server --features jetson --offline
```

If cargo reports missing dependencies, you will have to connect the system to
the network so they can be downloaded. Then run,
```sh
cargo install --path stitching_server --features jetson
```