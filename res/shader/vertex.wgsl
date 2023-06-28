// Model Vertex shader

struct Camera {
    view_proj: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex: vec2<f32>,
}

struct InstanceInput {
    @location(5) position: vec2<f32>,
    @location(6) rotation: vec4<f32>,
    // SHURA_MARKER_INSTANCE_INPUT
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    // SHURA_MARKER_VERTEX_OUTPUT
}

@vertex
fn main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex = model.tex;
    // SHURA_MARKER_VARIABLE_ASSIGNMENT

    let pos = model.position * mat2x2<f32>(instance.rotation.xy, instance.rotation.zw) + instance.position;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 0.0, 1.0);

    return out;
}
