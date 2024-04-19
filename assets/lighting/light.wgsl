struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}

const CENTER = vec2<f32>(0.5, 0.5);
const SIDE_FALLOFF = 0.15;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = distance(in.tex, CENTER) * 2.0;
    let dx = in.tex.x - CENTER.x;
    let dy = in.tex.y - CENTER.y;
    let angle = atan2(dy, dx);

    // let start = 3.14 + 0.785398;
    // let end = 3.14 + 2.35619;

    let end = -0.785398;
    let start = -2.35619;

    if angle > start && angle < end {
        return vec4<f32>(in.color.xyz, 1.0 - dist);
    }
    
    let left_diff = start - angle;
    if angle < start && left_diff < SIDE_FALLOFF {
        let test = left_diff / SIDE_FALLOFF;
        return vec4<f32>(in.color.xyz, (1.0 - dist) * (1.0 - test));
    }     
    
    let right_diff = angle - end;
    if angle > end && right_diff < SIDE_FALLOFF {
        let test = right_diff / SIDE_FALLOFF;
        return vec4<f32>(in.color.xyz, (1.0 - dist) * (1.0 - test));
    }


    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}


