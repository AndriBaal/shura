#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 output_color;

layout(set = 1, binding = 0) uniform vec4 input_color;

void main() {
    // float test = distance(tex_coords, vec2(0.5, 1.0));
    // output_color = vec4(input_color.rgb, 1.0 - test);
    output_color = input_color;
}
