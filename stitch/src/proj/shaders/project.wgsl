const PI: f32 = 3.141592653589793;

@group(0)
@binding(0)
var<storage, read_write> out_frame: array<u32>;

@group(0)
@binding(1)
var<uniform> pass_info: PassInfo;

struct PassInfo {
    out_spec: InputSpec,
    out_size: vec2<u32>,
    inp_sizes: vec3<u32>,
    bound_radius: f32,
}

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

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let bound = forw_proj(id.xy);
    let p = back_proj(bound);
    output_pixel(id.xy, p);
}

fn forw_proj(pix: vec2<u32>) -> vec3<f32> {
    let angs = opt_from_img(pass_info.out_spec, img_from_coord(vec2f(pix), pass_info.out_size));
    return forw_hemisphere(angs);
}

fn forw_hemisphere(opt: vec2<f32>) -> vec3<f32> {
    let r = pass_info.bound_radius;
    let s = pass_info.out_spec;

    let angs_rot = opt.y + s.ang.z;
    let tan_opt_ang = tan(opt.x);

    let tan_opt_p = tan_opt_ang * sin(angs_rot);
    let tan_cam_p = tan(s.ang.y);
    let tan_p = (tan_opt_p + tan_cam_p) / (1.0 + tan_opt_p * tan_cam_p);

    let azimuth = atan(tan_opt_ang * cos(angs_rot)) + s.ang.x;
    let xy_dir = vec2(sin(azimuth), cos(azimuth));

    if abs(tan_p) < 0.0001 {
        return vec3(sqrt(r * r - s.pos.z * s.pos.z) * xy_dir, s.pos.z);
    }

    let cot_p = 1.0 / tan_p;
    let cam_xy_dist = length(s.pos.xy);

    let xy_plane_mag = cam_xy_dist - s.pos.z * cot_p;
    if xy_plane_mag > 0.0 && xy_plane_mag < r {
        return vec3(xy_plane_mag * xy_dir, 0.0);
    }

    let p2 = cot_p * cot_p;
    let p2_1 = p2 + 1.0;

    let det_sqrt = sqrt(r * r * p2_1 - pow(cam_xy_dist - s.pos.z * cot_p, 2.0));
    let z = (sign(tan_p) * det_sqrt - cot_p * cam_xy_dist + p2 * s.pos.z) / p2_1;

    return vec3(sqrt(r * r - z * z) * xy_dir, z);
}

fn back_proj(bound: vec3<f32>) -> u32 {
    for (var n = 0u; n < pass_info.inp_sizes.z; n += 1u) {
        let opt_space = opt_from_world(inp_specs[n], bound);
        let p = opt_input_pixel(n, opt_space);
        if p != 0 {
            return p;
        }
    }
    return 0u;

    // let opt_space = opt_from_world(inp_specs[0u], bound);
    // return opt_input_pixel(0u, opt_space);
}

fn output_pixel(p: vec2<u32>, v: u32) {
    out_frame[p.x + p.y * pass_info.out_size.x] = v;
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
    let rot_ang = atan2(dot(s.up, rev_dir), dot(s.right, rev_dir)) - s.ang.z;
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
    return (rp * length(sf) + sf) / 2.0;
}

fn img_from_coord(pix: vec2<f32>, size: vec2<u32>) -> vec2<f32> {
    let sf = vec2f(size);
    return (2.0 * pix - sf) / length(sf);
}

fn opt_from_img(s: InputSpec, rp: vec2<f32>) -> vec2<f32> {
    return vec2(atan(length(rp) / s.foc_dist), atan2(rp.y, rp.x));
}

// fn sphereDir(p: vec3<f32>) -> vec2<f32> {
//     return vec2(atan2(p.x, p.y), atan2(p.z, length(p.xy)));
// }

fn clamp_sphere(a: vec2<f32>) -> vec2<f32> {
    var wrapped: vec2<f32> = (a + PI) % (2.0 * PI) - PI;
    if wrapped.y > PI {
        wrapped.y = 2.0 * PI - wrapped.y;
    } else if wrapped.y < -PI {
        wrapped.y = -2.0 * PI + wrapped.y;
    }
    return wrapped;
}