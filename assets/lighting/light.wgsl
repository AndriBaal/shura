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
    @location(3) scale: f32,
    @location(4) rotation: f32,
    @location(5) color: vec4<f32>,
    @location(6) circle_sector: vec2<f32>,
    @location(7) inner_size: f32,
    @location(8) inner_magnification: f32,
    @location(9) outer_magnification: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) circle_sector: vec2<f32>,
    @location(3) inner_size: f32,
    @location(4) inner_magnification: f32,
    @location(5) outer_magnification: f32,
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
        instance.scale * cos,
        instance.scale * sin,
        instance.scale * -sin,
        instance.scale * cos,
    );
    let pos = model.v_position * scale_rotation + instance.translation;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 0.0, 1.0);
    out.tex = model.tex;
    out.color = instance.color;
    out.circle_sector = instance.circle_sector;
    out.inner_size = instance.inner_size;
    out.inner_magnification = instance.inner_magnification;
    out.outer_magnification = instance.outer_magnification;
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

//    let end = -0.785398;
//    let start = -2.35619;

//    f: y=((10^(x)-1)/(10-1))

    let start = in.circle_sector.x;
    let end = in.circle_sector.y;

    if angle > start && angle < end {
        return vec4<f32>(in.color.xyz, 1.0 - dist);
    }
    
    let left_diff = start - angle;
    if angle < start && left_diff < SIDE_FALLOFF {
        let dist_to_left = left_diff / SIDE_FALLOFF;
        return vec4<f32>(in.color.xyz, (1.0 - dist) * (1.0 - dist_to_left));
    }
    
    let right_diff = angle - end;
    if angle > end && right_diff < SIDE_FALLOFF {
        let dist_to_right = right_diff / SIDE_FALLOFF;
        return vec4<f32>(in.color.xyz, (1.0 - dist) * (1.0 - dist_to_right));
    }


    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}


