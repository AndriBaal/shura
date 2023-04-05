#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 output_color;

layout(set = 1, binding = 0) uniform vec4 input_color;
layout(set = 2, binding = 0) uniform vec4 shadow_color;

void main() {
  float shadow_alpha = 1.0 - shadow_color.a;
  float dist = distance(tex_coords, vec2(0.5)) * 2.0;
  output_color = mix(input_color * shadow_color.a, shadow_color, dist);
  // float test = pow(dist, 0.05);
  // if (dist < 0.8) {
  //   output_color = input_color;
  // } else {

  //   float test = (dist - 0.8) / 0.2;
  //   output_color = mix(input_color * shadow_color.a, shadow_color, test);
  // }




  // if (dist > 0.8){
  //   output_color = vec4(0.0, 0.0, 1.0, 1.0);
  // } else {
  //   output_color = vec4(0.0, 0.0, 1.0, 1.0);
  // }
  // output_color = mix(vec4(input_color.rgb, test), shadow_color, test);

  // float pct = distance(tex_coords, vec2(0.5)) * 2.0;
  // output_color = vec4(input_color.rgb, 1.0 - pct);


  // // float dist = length(tex_coords) * 2.0;
  // float dist = distance(tex_coords, vec2(0.5)) * 2.0;
  // // float alpha_target = 1.0 - shadow_color.a;
  // float test = pow(dist, 0.05);
  // vec4 input_target = vec4(input_color.rgb, input_color.a * test);
  // vec4 shadow_target = vec4(shadow_color.rgb, 1.0 - test);
  // // output_color = vec4(input_color.rgb, 1.0 - pow(dist, 0.2));
  // output_color = mix(input_color, shadow_color, 1.0);


  // float dist = distance(tex_coords, vec2(0.5)) * 2.0;
  // float scale = pow(dist, 0.1);
  // vec4 color = vec4(input_color.rgb, input_color.a * (1.0 - scale));
  // output_color = color;
  // output_color = mix(input_color, shadow_color, dist);



  // output_color =  mix(input_color, shadow_color, p);
  // output_color = vec4(input_color.rgb + shadow_color * p, 1.0 - dist);

  // float pct = distance(tex_coords, vec2(0.5)) * 2.0;
  // float p = pow(pct, 0.1);
  // vec4 color = mix(input_color, shadow_color, p);
  // output_color = vec4(color, 1.0 - p);



  // vec4 light = vec4(input_color.rgb, 1.0 - p);
  // output_color = mix(light, shadow_color, p);


  // // float dist = length(tex_coords) * 2.0;
  // float dist = distance(tex_coords, vec2(0.5)) * 2.0;
  // // output_color = vec4(input_color.rgb, 1.0 - pow(dist, 0.2));
  // output_color = vec4(0.0, 0.0, 0.0, dist - 0.1);
}


