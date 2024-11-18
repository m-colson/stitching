cross.exe build -p stitching_server --target aarch64-unknown-linux-gnu --release

(
    upload-runner.exe
    -perf
    -bin server
    -i 'live.toml,stitching_server\assets\*,stitching_server\assets\**\*'
    'C:\Users\ninja\.cargo-target\aarch64-unknown-linux-gnu\release\stitching_server' serve-gpu --timeout 20 
)
# cross.exe build -p stitching_server --target aarch64-unknown-linux-gnu

# (
#     upload-runner.exe
#     -bin server
#     -i 'live.toml,stitching_server\assets\*,stitching_server\assets\**\*'
#     'C:\Users\ninja\.cargo-target\aarch64-unknown-linux-gnu\debug\stitching_server' serve-gpu
# )