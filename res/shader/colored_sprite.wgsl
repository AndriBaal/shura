// Colored Texture shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(2) @binding(0)
var<uniform> color: vec4<f32>;

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    let alpha = 1.0-color.w;

	return vec4<f32>(
        t.x * alpha + color.x * color.w,
        t.y * alpha + color.y * color.w,
        t.z * alpha + color.z * color.w,
        t.w
    );
}
