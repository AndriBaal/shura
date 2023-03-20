// Transparent Sprite shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
}

struct Transperancy {
    @align(16) transparency: f32,
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(2) @binding(0)
var<uniform> t: Transperancy;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return vec4<f32>(color.rgb, color.a - t.transparency);
}
