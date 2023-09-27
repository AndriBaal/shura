// Model Vertex shader

struct Camera {
    view_proj: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) tex: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>
}

@vertex
fn main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = camera.view_proj * vec4<f32>(model.v_position, 0.0, 1.0);
    out.tex = model.tex;

    return out;
}
