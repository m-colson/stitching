# Configuration

The software configuration files are written in the [TOML](https://toml.io) format.

By default, the software will look for a file named `live.toml` in the directory the
binary was executed from.

By default, the casa systemd service will read from the file at `/home/casa/casa-src/prod.toml`.

## Options
```toml
[server] #server options
host = #hostname to listen on
port = #port to listen on
asset_dir = #path to the directory that contains the server assets

[world.cylinder] #use a cylindrical boundary for the world
radius = #radius of the cylinder
height = #height of the cylinder

[model] #plane model options
path = #path to the *.obj file of the plane
origin = #[x, y, z] in the model that should be [0, 0, 0] in the world
scale = #[scale x, scale y, scale z] of the model relative to the world units
rot = #[azimuth, pitch, roll] for the model
light_dir = #[dx, dy, dz] for the shading light, based to MODEL coordinates

[view.orbit] #default view style of orbit
fov_y = #vertical fov in degrees
dist = #distance away for the camera while orbiting
z = #height of the camera
look_at = #[lx, ly, lz] position to keep in the middle of the camera
frame_per_rev = #amount of frames until the camera makes a complete orbit

# NOTE: the views selected from the UI must be configured in the
#   `set_view_handler` function in the stitching_server/src/app.rs file

[[cameras]] #one camera entry, repeat the same thing for multiple cameras
pos = #[x, y, z] position of the real camera in world coordinates
pitch = #pitch of the real camera in degrees
azimuth = #azimuth of the real camera in degrees
roll = #roll of the real camera in degrees. 180 is an upside down camera
sensor = { img_off =, fov.W =, fov.H =, fov.D = }
    # img_off is the [x, y] pixel offset from the optical center to the image center
    # use of fov.{W, H, D} to indicate the horizontal, vertical or diagonal
    # fov in degrees, respectively. Diagonal typically works best
lens = #type of lens on the real camera
    #^ One of "rectilinear", "equidistant", or "equisolid"
resolution = #[w, h] of the captured imaged
framerate = #target framerate for the capture stream
mask_path = #path to the image containing the camera's mask

#capture source. one of:
pattern.color = #[r, g, b] color for the grid
image = #path to the pre-captured image to use
v4l.index = #video4linux device index number
argus = { index =, mode = }
    # index is the number for the camera as reported by argus
    # mode is the number for the camera mode as reported by argus


```