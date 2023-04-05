#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 gl_FragColor;

layout(set = 1, binding = 0) uniform texture2D texture;
layout(set = 1, binding = 1) uniform sampler texture_sampler;
layout(set = 2, binding = 0) uniform vec4 shadow_color;

void main() {
    vec4 t = texture(sampler2D(texture, texture_sampler), tex_coords);
    if (t.a <= 0.3) {
        t = vec4(1.0, 0.0, 0.0, 1.0);
    }
    gl_FragColor = t;
}
