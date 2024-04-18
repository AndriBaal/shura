struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(in.tex, center) * 2.0;
    let dx = in.tex.x - center.x;
    let dy = in.tex.y - center.y;

    let start = -3.14159;
    let end = 3.14159;

    let test = atan2(dy, dx);
    if test > start && test < end {
        return vec4<f32>(in.color.xyz, 1.0 - dist);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}


