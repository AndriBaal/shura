struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = distance(in.tex, vec2<f32>(0.5, 0.5)) * 2.0;
    return vec4<f32>(in.color.xyz, 1.0 - dist);
}


