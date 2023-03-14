#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 gl_FragColor;

layout(set = 1, binding = 0) uniform texture2D texture;
layout(set = 1, binding = 1) uniform sampler texture_sampler;
layout(set = 2, binding = 0) uniform vec4 input_color;

void main() {
  vec4 t = texture(sampler2D(texture, texture_sampler), tex_coords);
  t.rgb *= (1.0 - input_color.a);
  t.rgb += input_color.rgb * input_color.a;
  gl_FragColor = t;
}
