// Texture Shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) sprite: u32,
}

@group(1) @binding(0) 
var t_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;


@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(
        t_diffuse,
        s_diffuse,
        in.tex,
        in.sprite
    ).r * in.color;
}

