// Texture Shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) sprites: vec2<i32>
}

@group(1) @binding(0) 
var t_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var<uniform> sprite_amount: Uniforms;

@group(2) @binding(0)
var<uniform> sprite_index: Uniforms;

struct Uniforms {
    @size(16) index: vec2<i32>,   
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let index = sprite_index.index.y * sprite_amount.index.x + sprite_index.index.x;
    return textureSample(
        t_diffuse,
        s_diffuse,
        in.tex,
        index
    );
}

