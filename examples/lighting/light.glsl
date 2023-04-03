// #version 450

// layout(location = 0) in vec2 tex_coords;
// layout(location = 0) out vec4 output_color;

// layout(set = 1, binding = 0) uniform vec4 input_color;

// void main() {
//   float pct = 0.0;
//   pct = distance(tex_coords, vec2(0.5)) * 2.0;
//   output_color = vec4(input_color.rgb, 1.0 - pow(pct, 0.1));
// }

#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 output_color;

layout(set = 1, binding = 0) uniform vec4 input_color;
layout(set = 2, binding = 0) uniform vec4 shadow_color;

void main() {

  // float dist = length(tex_coords) * 2.0;
  float dist = distance(tex_coords, vec2(0.5)) * 2.0;
  // output_color = vec4(input_color.rgb, 1.0 - pow(dist, 0.2));
  output_color = vec4(0.0, 0.0, 0.0, dist - 0.1);
}

