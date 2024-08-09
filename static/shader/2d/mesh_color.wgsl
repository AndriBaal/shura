@group(0) @binding(0)
var<uniform> u_camera: mat4x4<f32>;

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) v_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u_camera * vec4<f32>(model.v_position, 0.0, 1.0);
    out.color = model.v_color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
