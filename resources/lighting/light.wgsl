struct ShadowLine {
    light_center: vec2<f32>,
    start: vec2<f32>,
    end: vec2<f32>,
};


@group(0) @binding(0)
var<uniform> u_camera: mat4x4<f32>;

@group(1) @binding(0)
var<storage, read> u_shadows: array<ShadowLine>;

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) v_tex: vec2<f32>,
}

struct InstanceInput {
    @location(2) i_translation: vec2<f32>,
    @location(3) i_scale_rotation: vec4<f32>,
    @location(4) i_color: vec4<f32>,
    @location(5) i_circle_sector: vec2<f32>,
    @location(6) i_inner_radius: f32,
    @location(7) i_inner_magnification: f32,
    @location(8) i_outer_magnification: f32,
    @location(9) i_side_falloff_magnification: f32,
    @location(10) i_shadow_range: vec2<u32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) circle_sector: vec2<f32>,
    @location(3) inner_radius: f32,
    @location(4) inner_magnification: f32,
    @location(5) outer_magnification: f32,
    @location(6) side_falloff_magnification: f32,
    @location(7) shadow_range: vec2<u32>,
    @location(8) test: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let pos = model.v_position * mat2x2<f32>(instance.i_scale_rotation.xy, instance.i_scale_rotation.zw) + instance.i_translation;
    out.position = u_camera * vec4<f32>(pos, 0.0, 1.0);
    out.tex = model.v_tex;
    out.color = instance.i_color;
    out.circle_sector = instance.i_circle_sector;
    out.inner_radius = instance.i_inner_radius;
    out.inner_magnification = instance.i_inner_magnification;
    out.outer_magnification = instance.i_outer_magnification;
    out.side_falloff_magnification = instance.i_side_falloff_magnification;
    out.shadow_range = instance.i_shadow_range;
    out.test = pos;
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


    let start = in.circle_sector.x;
    let end = in.circle_sector.y;


    for (var i: u32 = in.shadow_range.x; i < in.shadow_range.y; i = i + 1) {
        let shadow = u_shadows[i];
        let intersects = lines_intersect(shadow.light_center, in.test, shadow.start, shadow.end);
        if intersects {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }

    if angle > start && angle < end || start == end {
        var strength = (pow(in.outer_magnification, dist) - 1.0) / (in.outer_magnification - 1.0);
        if dist < in.inner_radius {
            strength *= (pow(in.inner_magnification, dist / in.inner_radius) - 1.0) / (in.inner_magnification - 1.0);
        }
        strength *= in.color.w;
        return vec4<f32>(in.color.xyz, 1.0 - strength);
    }

    // let left_diff = start - angle;
    // if angle < start && left_diff < SIDE_FALLOFF {
    //     let dist_to_left = left_diff / SIDE_FALLOFF;
    //     let left_strength = (pow(in.side_falloff_magnification, dist_to_left) - 1.0) / (in.side_falloff_magnification - 1.0);
    //     return vec4<f32>(in.color.xyz, (1.0 - strength) * (1.0 - left_strength));
    // }
    
    // let right_diff = angle - end;
    // if angle > end && right_diff < SIDE_FALLOFF {
    //     let dist_to_right = right_diff / SIDE_FALLOFF;
    //     let right_strength = (pow(in.side_falloff_magnification, dist_to_right) - 1.0) / (in.side_falloff_magnification - 1.0);
    //     return vec4<f32>(in.color.xyz, (1.0 - strength) * (1.0 - right_strength));
    // }

    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

fn cross(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn lines_intersect(p1: vec2<f32>, p2: vec2<f32>, q1: vec2<f32>, q2: vec2<f32>) -> bool {
    let r = p2 - p1;
    let s = q2 - q1;

    let rxs = cross(r, s);
    
    let qp = q1 - p1;

    let qpxr = cross(qp, r);

    if rxs == 0.0 && qpxr == 0.0 {
        let r_dot_r = dot(r, r);
        let qp_dot_r = dot(qp, r);
        let t0 = qp_dot_r / r_dot_r;
        let t1 = t0 + dot(s, r) / r_dot_r;
        
        return (t0 >= 0.0 && t0 <= 1.0) || (t1 >= 0.0 && t1 <= 1.0);
    }

    if rxs == 0.0 && qpxr != 0.0 {
        return false;
    }

    let t = cross(qp, s) / rxs;
    let u = qpxr / rxs;

    return (t >= 0.0 && t <= 1.0) && (u >= 0.0 && u <= 1.0);
}
