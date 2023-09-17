struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) tex: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) glyph: u32,
}

struct InstanceInput {
    @location(4) i_position: vec2<f32>,
    @location(5) rotation: vec4<f32>,
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
fn main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let pos = model.v_position * mat2x2<f32>(instance.rotation.xy, instance.rotation.zw) + instance.i_position;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 0.0, 1.0);
    out.color = model.color;
    out.sprite = model.glyph;

    return out;
}
