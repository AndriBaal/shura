// Texture Shader

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) tex: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) glyph: u32,
}

struct InstanceInput {
    @location(4) i_position: vec2<f32>,
    @location(5) i_rotation: vec4<f32>,
    @location(6) a_position: vec2<f32>,
    @location(7) a_scale: vec2<f32>,
    @location(8) color: vec4<f32>,
    @location(9) index: u32,
}

struct Camera {
    view_proj: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) sprite: u32,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let pos = model.v_position * mat2x2<f32>(instance.i_rotation.xy, instance.i_rotation.zw) + instance.i_position;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 0.0, 1.0);
    out.color = model.color;
    out.sprite = model.glyph;
    out.tex = model.tex;

    return out;
}


@group(1) @binding(0) 
var t_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let value = textureSample(
        t_diffuse,
        s_diffuse,
        in.tex,
        in.sprite
    ).r;
    return value * in.color * in.color;

}

