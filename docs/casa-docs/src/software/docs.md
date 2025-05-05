# Software Documentation

The detailed software docs are written directly in the source code, which provides
many benefits including clickable links to other items in the code.

It can be generated using rustdoc. From within the source directory run:
```sh
cargo doc --open
```

This will open the documentation for the `stitching_server` crate in your default
web browser. On the default page, the left bar contains the docs for all
crates that are used, which includes the other crates in this project.