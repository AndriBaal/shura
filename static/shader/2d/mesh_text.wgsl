@group(0) @binding(0)
var<uniform> u_camera: mat4x4<f32>;

@group(1) @binding(0) 
var u_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var u_sampler: sampler;

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) v_tex: vec2<f32>,
    @location(2) v_color: vec4<f32>,
    @location(3) v_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = u_camera * vec4<f32>(model.v_position, 0.0, 1.0);
    out.color = model.v_color;
    out.index = model.v_index;
    out.tex = model.v_tex;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(
        u_diffuse,
        u_sampler,
        in.tex,
        in.index
    ).r * in.color;
}
