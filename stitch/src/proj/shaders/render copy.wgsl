const PI: f32 = 3.141592653589793;

@group(0)
@binding(0)
var<uniform> pass_info: PassInfo;

struct PassInfo {
    inp_sizes: vec3<u32>,
    bound_radius: f32,
}

@group(0)
@binding(1)
var<uniform> view: mat4x4<f32>;

@group(0)
@binding(2)
var<storage, read> inp_frames: array<u32>;

@group(0)
@binding(3)
var<storage, read> inp_specs: array<InputSpec>;

struct InputSpec {
    pos: vec3<f32>,
    forw: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
    ang: vec3<f32>,
    foc_dist: f32,
    lens_type: u32,
}

struct VertexOutput {
    @builtin(position) proj_pos: vec4<f32>,
    @location(1) world_pos: vec4<f32>,
}

@vertex
fn vs_proj(@location(0) position: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.proj_pos = view * position;
    out.world_pos = position;
    return out;
}

@fragment
fn fs_proj(vert: VertexOutput) -> @location(0) vec4<f32> {
    // vec3(100.0 * img_from_coord(vec2f(id.xy), pass_info.out_size), 0.0)
    let p = back_proj(vert.world_pos.xyz);
    return unpack4x8unorm(p);
}

fn back_proj(bound: vec3<f32>) -> u32 {
    var best_index = 0u;
    var best = opt_from_world(inp_specs[0], bound);
    for (var n = 1u; n < pass_info.inp_sizes.z; n += 1u) {
        let opt = opt_from_world(inp_specs[n], bound);
        if opt.x < best.x {
            best = opt;
            best_index = n;
        }
    }

    return opt_input_pixel(best_index, best);       
}

fn opt_input_pixel(n: u32, os: vec2<f32>) -> u32 {
    let inpSize = pass_info.inp_sizes.xy;
    let spec = inp_specs[n];

    let imgPos = coord_from_img(img_from_opt(spec, os), inpSize);
    if any(imgPos < vec2f(0.0, 0.0)) || any(imgPos >= vec2f(inpSize)) {
        return 0u;
    }

    return input_pixel(n, vec2u(imgPos));
}

fn input_pixel(n: u32, p: vec2<u32>) -> u32 {
    return inp_frames[p.x + (p.y + n * pass_info.inp_sizes.y) * pass_info.inp_sizes.x];
}

// Spaces:
// world -> (x, y, z)
// optical -> (opt_ang, rot_ang)
// image -> (ux, uy) on unit circle spanning diagonal

fn opt_from_world(s: InputSpec, rev_pos: vec3<f32>) -> vec2<f32> {
    let rev_dir = normalize(rev_pos - s.pos);
    let opt_ang = acos(dot(rev_dir, s.forw));

    let dright = dot(rev_dir, s.right);
    let dup = dot(rev_dir, s.up);
    let rot_ang = sign(dup) * acos(dright / length(vec2(dright, dup)));
    // let rot_ang = atan2(dot(s.up, rev_dir), dot(s.right, rev_dir)) - s.ang.z;
    return vec2(opt_ang, rot_ang);
}

fn img_from_opt(s: InputSpec, angs: vec2<f32>) -> vec2<f32> {
    var r: f32 = 0.0;
    switch s.lens_type {
        case 0u, default: {
            r = s.foc_dist * tan(angs.x);
        }
        case 1u: {
            r = s.foc_dist * angs.x;
        }
    }

    return vec2(r * cos(angs.y), r * sin(angs.y));
}

fn coord_from_img(rp: vec2<f32>, size: vec2<u32>) -> vec2<f32> {
    let sf = vec2f(size);
    return (vec2f(1, -1) * rp * length(sf) + sf) / 2.0;
}
