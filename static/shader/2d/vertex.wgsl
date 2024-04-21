struct Camera {
    view_proj: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) tex: vec2<f32>,
}


struct InstanceInput {
    @location(2) i_position: vec2<f32>,
    @location(3) i_scale: vec2<f32>,
    @location(4) i_rotation: f32,
    @location(5) a_position: vec2<f32>,
    @location(6) a_scale: vec2<f32>,
    @location(7) color: vec4<f32>,
    @location(8) index: u32,
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
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let cos = cos(instance.i_rotation);
    let sin = sin(instance.i_rotation);
    let scale_rotation = mat2x2<f32>(
        instance.i_scale.x * cos,
        instance.i_scale.x * sin,
        instance.i_scale.y * -sin,
        instance.i_scale.y * cos,
    );
    let pos = model.v_position * scale_rotation + instance.i_position;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 0.0, 1.0);
    out.tex = model.tex * instance.a_scale + instance.a_position;
    out.color = instance.color;
    out.index = instance.index;
    return out;
}
