# CASA Sticher

This repository contains the source files and documentation of the Camera for Aerospace Situational Awareness system.

## Viewing Documentation Book

Install the Rust compiler and build system onto your local machine with [rustup](https://rustup.rs).

Install [mdbook](https://rust-lang.github.io/mdBook/), which will be used to build the documentation files.
```sh
cargo install mdbook
```

Build the docs by running:
```sh
cd docs/casa-docs
mdbook build
```

This will generate a folder at `docs/casa-docs/book` containing the files for
the web based documentation. Open this location in your file explorer and click
`index.html`. You should be able to click on the left links to view each section.