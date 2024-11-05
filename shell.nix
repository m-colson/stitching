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
        
        rust-bin.stable.latest.default

        unstablePkgs.rust-analyzer
    ] ++ xorgPkgs;

    LD_LIBRARY_PATH = lib.makeLibraryPath ([
            vulkan-loader
            libGL
            libclang
        ] ++ xorgPkgs);
}
