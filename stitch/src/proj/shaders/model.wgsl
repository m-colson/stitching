@group(0)
@binding(0)
var<uniform> mview: mat4x4<f32>;

@group(0)
@binding(1)
var<uniform> cview: mat4x4<f32>;

@group(1)
@binding(0)
var<uniform> light_dir: vec3<f32>;

struct VertexOutput {
    @builtin(position) proj_pos: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) world_pos: vec4<f32>,
}

@vertex
fn vs_proj(@location(0) v_pos: vec4<f32>, @location(1) v_norm: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.proj_pos = cview * mview * v_pos;
    out.normal = v_norm;
    out.world_pos = v_pos;
    return out;
}

const COLOR = vec3(0.9, 0.9, 0.9);

@fragment
fn fs_proj(vert: VertexOutput) -> @location(0) vec4<f32> {
    let corr = max(dot(light_dir, vert.normal.xyz), 0.1);

    return vec4(corr * COLOR, 1.0);
}
