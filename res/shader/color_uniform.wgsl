// Color Shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
}

struct Color {
    color: vec4<f32>
}

@group(1) @binding(0)
var<uniform> color: Color;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    return color.color;
}
