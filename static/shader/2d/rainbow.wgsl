struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}


struct Times {
    @size(8) total_time: f32,
    @size(8) delta_time: f32
}

@group(1) @binding(0) 
var<uniform> total_time: Times;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let col = 0.5 + 0.5*cos(total_time.total_time+in.tex.xyx+vec3<f32>(0.0,2.0,4.0));
    return vec4<f32>(col.xyz, 1.0);
}
