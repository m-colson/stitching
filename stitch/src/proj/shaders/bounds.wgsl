@group(0)
@binding(0)
var<uniform> mview: mat4x4<f32>;

@group(0)
@binding(1)
var<uniform> cview: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) proj_pos: vec4<f32>,
    @location(0) text_coord: vec2<f32>,
}

@vertex
fn vs_proj(@location(0) v_pos: vec4<f32>, @location(1) text_coord: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.proj_pos = cview * mview * v_pos;
    out.text_coord = text_coord;
    return out;
}

const COLOR = vec3(1.0, 0.1, 0.1);

@fragment
fn fs_proj(vert: VertexOutput) -> @location(0) vec4<f32> {
    if abs(vert.text_coord.x) > 0.98 || abs(vert.text_coord.y) > 0.98 {
        return vec4(COLOR, 1.0);
    }
    return vec4(COLOR, 0.2);
}
