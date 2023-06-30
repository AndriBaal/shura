struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = distance(in.tex_coords, vec2<f32>(0.5, 0.5)) * 2.0;
    return vec4<f32>(in.color.xyz, 1.0 - dist);
    // return vec4<f32>(in.color.xyz, pow(0.01, dist) - 0.01);
}


