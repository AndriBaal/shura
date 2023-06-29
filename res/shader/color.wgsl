// Color Shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
