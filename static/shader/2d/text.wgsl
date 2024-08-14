@group(0) @binding(0)
var<uniform> camera: mat4x4<f32>;

@group(1) @binding(0)
var u_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var u_sampler: sampler;

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) v_tex: vec2<f32>,
}

struct InstanceInput {
    @location(2) i_translation: vec2<f32>,
    @location(3) i_scale_rotation: vec4<f32>,
    @location(4) i_tex_scale: vec2<f32>,
    @location(5) i_color: vec4<f32>,
    @location(6) i_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}-

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    let pos = model.v_position * mat2x2<f32>(instance.i_scale_rotation.xy, instance.i_scale_rotation.zw) + instance.i_translation;
    out.clip_position = camera * vec4<f32>(pos, 0.0, 1.0);
    out.tex = model.v_tex * instance.i_tex_scale;
    out.color = instance.i_color;
    out.index = instance.i_index;
    
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

