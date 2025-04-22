{ pkgs ? (import <nixpkgs> {
    overlays = [(import <rust-overlay>)];
  })
, ...
}:
let 
    xorgPkgs = with pkgs.xorg; [
        libX11 libXrandr libXinerama libXcursor libXi
    ];
    unstablePkgs = import <unstable> {};
in with pkgs; mkShell {
    buildInputs = [
        openssl
        pkg-config
        cmake
        libGL
        clang
        llvmPackages_17.libclang.lib
        
        (rust-bin.stable.latest.default.override {
            extensions = ["rust-src"];
        })

        unstablePkgs.rust-analyzer

        mdbook
    ] ++ xorgPkgs;

    LD_LIBRARY_PATH = lib.makeLibraryPath ([
            vulkan-loader
            libGL
        ] ++ xorgPkgs);

    LIBCLANG_PATH = "${llvmPackages_17.libclang.lib}/lib";
}
