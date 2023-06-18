#version 450

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=0) out vec2 v_tex_coords;

layout(set=0, binding=0) 
uniform Camera {
    mat4 u_view_proj;
};

layout(location=5) in vec2 position;
layout(location=6) in vec4 rotation;

void main() {
    v_tex_coords = a_tex_coords;
    vec2 pos = a_position * mat2(rotation.xy, rotation.zw) + position;
	gl_Position = u_view_proj * vec4(pos, 0.0, 1.0);
}

