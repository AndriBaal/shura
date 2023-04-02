#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 output_color;

layout(set = 1, binding = 0) uniform vec4 input_color;

void main() {
  float pct = 0.0;
  pct = distance(tex_coords, vec2(0.5)) * 2.0;
  output_color = vec4(input_color.rgb, 1.0 - pow(pct, 0.1));
}
