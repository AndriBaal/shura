// Texture Shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) sprite: vec2<i32>  
}

@group(0) @binding(0) 
var t_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var sprite_amount: vec2<i32>;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let index = in.sprite.y * sprite_amount.y + in.sprite.x;
    let outval = textureSample(
        t_diffuse,
        s_diffuse,
        fragment.tex,
        index
    ).rgb;

    return vec4<f32>(outval.x, outval.y, outval.z, 1.0);
}

