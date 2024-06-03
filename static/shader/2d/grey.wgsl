@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex) * in.color;
    let gray = dot(vec3<f32>(0.299, 0.587, 0.114), color.rgb);

    return vec4<f32>(gray, gray, gray, color.a);
}
