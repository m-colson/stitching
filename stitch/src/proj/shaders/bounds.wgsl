@group(0)
@binding(0)
var<uniform> mview: mat4x4<f32>;

@group(0)
@binding(1)
var<uniform> cview: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) proj_pos: vec4<f32>,
}

@vertex
fn vs_proj(@location(0) v_pos: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.proj_pos = cview * mview * v_pos;
    return out;
}

const COLOR = vec3(1.0, 1.0, 1.0);

@fragment
fn fs_proj(vert: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(COLOR, 0.3);
}
