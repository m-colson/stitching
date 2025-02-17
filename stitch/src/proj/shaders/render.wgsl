@group(0)
@binding(0)
var<uniform> mview: mat4x4<f32>;

@group(0)
@binding(1)
var<uniform> cview: mat4x4<f32>;

@group(1)
@binding(0)
var<uniform> pass_info: PassInfo;

struct PassInfo {
    bound_radius: f32,
    num_cameras: u32,
}

@group(1)
@binding(1)
var<storage, read> inp_frames: array<u32>;

@group(1)
@binding(2)
var<storage, read> inp_specs: array<InputSpec>;

@group(1)
@binding(3)
var<storage, read> inp_masks: array<u32>;

struct InputSpec {
    res: vec2<u32>,
    data_start: u32,
    pos: vec3<f32>,
    rev_mat: mat3x3<f32>,
    img_off: vec2<f32>,
    foc_dist: f32,
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

fn opt_input_pixel(n: u32, os: vec3<f32>) -> u32 {
    let spec = inp_specs[n];
    let inpSize = spec.res;

    let imgPos = coord_from_img(img_from_opt(spec, os), inpSize) + spec.img_off;
    if any(imgPos < vec2f(0.0, 0.0)) || any(imgPos >= vec2f(inpSize)) {
        return 0u;
    }

    return input_pixel(spec.data_start, inpSize, vec2u(imgPos));
}

fn input_pixel(base: u32, res: vec2<u32>, p: vec2<u32>) -> u32 {
    let off = p.x + (p.y) * res.x + base;
    return min(inp_masks[off], inp_frames[off]);
}

// Spaces:
// world -> (x, y, z)
// optical -> (opt_rel_x, opt_rel_y, opt_ang)
// image -> (ux, uy) on unit circle spanning diagonal

fn opt_from_world(s: InputSpec, rev_pos: vec3<f32>) -> vec3<f32> {
    let rev_dir = normalize(rev_pos - s.pos);
    let ds = s.rev_mat * rev_dir;

    let opt_rel = normalize(ds.xz);
    return vec3(opt_rel, acos(ds.y));
}

fn img_from_opt(s: InputSpec, angs: vec3<f32>) -> vec2<f32> {
    var r: f32 = 0.0;
    switch s.lens_type {
        case 0u, default: {
            r = s.foc_dist * tan(angs.z);
        }
        case 1u: {
            r = s.foc_dist * angs.z;
        }
        case 2u: {
            r = 2.0 * s.foc_dist * sin(angs.z / 2.0); 
        }
    }

    return r * angs.xy;
}

fn coord_from_img(rp: vec2<f32>, size: vec2<u32>) -> vec2<f32> {
    let sf = vec2f(size);
    return (vec2f(1, -1) * rp * length(sf) + sf) / 2.0;
}
