// Texture Shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@group(2) @binding(0)
var<uniform> transparent: Uniforms;

struct Uniforms {
    @size(16) transparent: f32,   
};

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    var pixel = textureSample(t_diffuse, s_diffuse, in.tex);
    pixel.a *= transparent.transparent;
    return pixel;
}