@group(0) @binding(0)
var<uniform> u_camera: mat4x4<f32>;

struct VertexInput {
    @location(0) v_position: vec2<f32>,
}

struct InstanceInput {
    @location(1) i_translation: vec2<f32>,
    @location(2) i_scale_rotation: vec4<f32>,
    @location(3) i_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    let pos = model.v_position * mat2x2<f32>(instance.i_scale_rotation.xy, instance.i_scale_rotation.zw) + instance.i_translation;
    out.clip_position = u_camera * vec4<f32>(pos, 0.0, 1.0);
    out.color = instance.i_color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
