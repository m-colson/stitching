@group(0)
@binding(0)
var<uniform> mview: mat4x4<f32>;

@group(0)
@binding(1)
var<uniform> cview: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) proj_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) text_coord: vec2<f32>,
}

@vertex
fn vs_proj(@location(0) v_pos: vec4<f32>,  @location(1) color: vec4<f32>, @location(2) text_coord: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.proj_pos = cview * mview * v_pos;
    out.color = color;
    out.text_coord = text_coord;
    return out;
}

@fragment
fn fs_proj(vert: VertexOutput) -> @location(0) vec4<f32> {
    if abs(vert.text_coord.x) > 0.98 || abs(vert.text_coord.y) > 0.98 {
        return vec4(vert.color.xyz, 1.0);
    }
    return vec4(vert.color.xyz, 0.2);
}
