@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) index: u32,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_diffuse, s_diffuse, in.tex);
    let save_radius = 32;
    let quality = 3.0;
    let directions = 16.0;
    let pi2 = 6.28318530718;

    let radius = vec2<f32>(0.005, 0.005);
    for(var d=0.0; d < pi2; d = d + pi2 / directions) {
        for(var i =1.0 / quality; i <= 1.0; i = i + 1.0 / quality) {
            color = color + textureSample(t_diffuse, s_diffuse, in.tex+vec2<f32>(cos(d),sin(d))*radius*i);
        }
    }
    color = color / (quality * directions - 15.0);
    return color;
}
