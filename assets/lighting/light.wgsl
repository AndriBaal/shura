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
    @location(2) translation: vec2<f32>,
    @location(3) rotation: f32,
    @location(4) color: vec4<f32>,
    @location(5) circle_sector: vec2<f32>,
    @location(6) inner_radius: f32,
    @location(7) outer_radius: f32,
    @location(8) inner_magnification: f32,
    @location(9) outer_magnification: f32,
    @location(10) side_falloff_magnification: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) circle_sector: vec2<f32>,
    @location(3) inner_radius: f32,
    @location(4) inner_magnification: f32,
    @location(5) outer_magnification: f32,
    @location(6) side_falloff_magnification: f32,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let cos = cos(instance.rotation);
    let sin = sin(instance.rotation);
    let scale_rotation = mat2x2<f32>(
        instance.outer_radius * cos,
        instance.outer_radius * sin,
        instance.outer_radius * -sin,
        instance.outer_radius * cos,
    );
    let pos = model.v_position * scale_rotation + instance.translation;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 0.0, 1.0);
    out.tex = model.tex;
    out.color = instance.color;
    out.circle_sector = instance.circle_sector;
    out.inner_radius = instance.inner_radius;
    out.inner_magnification = instance.inner_magnification;
    out.outer_magnification = instance.outer_magnification;
    out.side_falloff_magnification = instance.side_falloff_magnification;
    return out;
}

const CENTER = vec2<f32>(0.5, 0.5);
const SIDE_FALLOFF = 0.15;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = distance(in.tex, CENTER) * 2.0;
    let dx = in.tex.x - CENTER.x;
    let dy = in.tex.y - CENTER.y;
    let angle = atan2(dy, dx);

    var strength = (pow(in.outer_magnification, dist) - 1.0) / (in.outer_magnification - 1.0);
    if dist < in.inner_radius {
        strength *= (pow(in.inner_magnification, dist / in.inner_radius) - 1.0) / (in.inner_magnification - 1.0);
    }
    strength *= in.color.w;

    let start = in.circle_sector.x;
    let end = in.circle_sector.y;

    if angle > start && angle < end {
        return vec4<f32>(in.color.xyz, 1.0 - strength);
    }
    
    let left_diff = start - angle;
    if angle < start && left_diff < SIDE_FALLOFF {
        let dist_to_left = left_diff / SIDE_FALLOFF;
        let left_strength = (pow(in.side_falloff_magnification, dist_to_left) - 1.0) / (in.side_falloff_magnification - 1.0);
        return vec4<f32>(in.color.xyz, (1.0 - strength) * (1.0 - left_strength));
    }
    
    let right_diff = angle - end;
    if angle > end && right_diff < SIDE_FALLOFF {
        let dist_to_right = right_diff / SIDE_FALLOFF;
        let right_strength = (pow(in.side_falloff_magnification, dist_to_right) - 1.0) / (in.side_falloff_magnification - 1.0);
        return vec4<f32>(in.color.xyz, (1.0 - strength) * (1.0 - right_strength));
    }


    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}


