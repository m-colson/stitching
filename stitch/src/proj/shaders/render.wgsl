//! This file contains the shader implementation of the reverse stitching
//! process.


/// The model (world) view transformation matrix.
@group(0)
@binding(0)
var<uniform> mview: mat4x4<f32>;

/// The camera view transformation matrix.
@group(0)
@binding(1)
var<uniform> cview: mat4x4<f32>;

/// Information about this pass of the shader.
@group(1)
@binding(0)
var<uniform> pass_info: PassInfo;

struct PassInfo {
    /// unused.
    bound_radius: f32,
    /// number of the cameras in the inp_specs array. 
    num_cameras: u32,
}

/// Contains the input frame pixels with each camera back-to-back.
@group(1)
@binding(1)
var<storage, read> inp_frames: array<u32>;

/// Contains information about every camera.
@group(1)
@binding(2)
var<storage, read> inp_specs: array<InputSpec>;

/// Just like `inp_frames` but for the frame mask.
@group(1)
@binding(3)
var<storage, read> inp_masks: array<u32>;

struct InputSpec {
    /// Resolution of the camera.
    res: vec2<u32>,
    /// Offset into `inp_frames` that the pixels start at.
    data_start: u32,
    /// 3d coordinate of the camera.
    pos: vec3<f32>,
    /// Transform matrix representing the cameras rotation.
    rev_mat: mat3x3<f32>,
    /// Pixel offset of the sensor.
    img_off: vec2<f32>,
    /// Equivalent focal distance of the camera.
    foc_dist: f32,
    /// LensKind of the camera.
    lens_type: u32,
}

struct VertexOutput {
    @builtin(position) proj_pos: vec4<f32>,
    @location(1) world_pos: vec4<f32>,
}

@vertex
fn vs_proj(@location(0) v_pos: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.proj_pos = cview * mview * v_pos;
    out.world_pos = v_pos;
    return out;
}

@fragment
fn fs_proj(vert: VertexOutput) -> @location(0) vec4<f32> {
    // vec3(100.0 * img_from_coord(vec2f(id.xy), pass_info.out_size), 0.0)
    let p = back_proj(vert.world_pos.xyz);
    return unpack4x8unorm(p);
}

/// Spaces:
/// world -> (x, y, z)
/// optical -> (opt_rel_x, opt_rel_y, opt_ang)
/// image -> (ux, uy) on unit circle spanning diagonal

/// Takes a world coordinate and determines the color of that position based
/// on the other cameras.
fn back_proj(bound: vec3<f32>) -> u32 {
    var opts: array<vec3<f32>, 4>;
    // First, precompute the optical space coords for the bound coord
    for (var n = 0u; n < pass_info.num_cameras; n += 1u) {
        opts[n] = opt_from_world(inp_specs[n], bound);
    }

    /// Next, loop through them and find the smallest optical angle
    var min_opt: f32 = 0.0;
    for (var iters = 0u; iters < pass_info.num_cameras; iters += 1u) {
        var best_index = 0u;
        var best = opts[0];
        for (var n = 1u; n < pass_info.num_cameras; n += 1u) {
            if opts[n].z < best.z && opts[n].z > min_opt {
                best = opts[n];
                best_index = n;
            }
        }

        let p = opt_input_pixel(best_index, best);
        // If we found a pixel with a non-zero alpha channel, return it
        if (p & 0xff000000u) != 0u {
            return p;
        }

        // Otherwise, repeat the loop again but skip any pixel with an optical
        // angle smaller than this one
        min_opt = best.z;
    }

    return 0u;
}

/// Finds the color of the pixel on camera `n` based on the provided
/// optical coordinates. Returns 0 if it is out of bounds or masked.
fn opt_input_pixel(n: u32, os: vec3<f32>) -> u32 {
    let spec = inp_specs[n];
    let inpSize = spec.res;

    let imgPos = coord_from_img(img_from_opt(spec, os), inpSize) + spec.img_off;
    if any(imgPos < vec2f(0.0, 0.0)) || any(imgPos >= vec2f(inpSize)) {
        return 0u;
    }

    return input_pixel(spec.data_start, inpSize, vec2u(imgPos));
}

/// Finds the color of the input pixel based on the base offeset and resolution.
/// Returns 0 if it is out of bounds or masked.
fn input_pixel(base: u32, res: vec2<u32>, p: vec2<u32>) -> u32 {
    let off = p.x + (p.y) * res.x + base;
    return min(inp_masks[off], inp_frames[off]);
}

/// Calculates the optical coordinates relative to the provided input spec
/// of a world position
fn opt_from_world(s: InputSpec, rev_pos: vec3<f32>) -> vec3<f32> {
    let rev_dir = normalize(rev_pos - s.pos);
    /// This is a trick to find the dot product of the reverse direction
    /// relative to the camera's x y and z direction vectors.
    let ds = s.rev_mat * rev_dir;

    /// This gives us the x and y components of the pixel's direction. 
    let opt_rel = normalize(ds.xz);
    /// Since `ds` is a bunch of dot products, we can take the inverse cosine of
    /// the y component to find the angle between the rev_dir and the camera's
    /// y direction, which is forward.
    let opt_ang = acos(ds.y);
    return vec3(opt_rel, opt_ang);
}

/// Calculates the image coordinate relative to the provided input spec
/// of an optical coordinate.
fn img_from_opt(s: InputSpec, angs: vec3<f32>) -> vec2<f32> {
    var r: f32 = 0.0;
    switch s.lens_type {
        // Rectilinear
        case 0u, default: {
            r = s.foc_dist * tan(angs.z);
        }
        // Equidistant
        case 1u: {
            r = s.foc_dist * angs.z;
        }
        // Equisolid
        case 2u: {
            r = 2.0 * s.foc_dist * sin(angs.z / 2.0); 
        }
    }

    return r * angs.xy;
}

/// Normalizes an image coordinate to be on image of the provided size.
fn coord_from_img(rp: vec2<f32>, size: vec2<u32>) -> vec2<f32> {
    let sf = vec2f(size);
    return (vec2f(1, -1) * rp * length(sf) + sf) / 2.0;
}
