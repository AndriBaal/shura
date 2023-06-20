#version 450

layout(location=0) in vec2 m_position;
layout(location=1) in vec2 m_tex;

layout(location=0) out vec2 v_tex;
layout(location=1) out ivec2 v_sprite;

layout(set=0, binding=0) 
uniform Camera {
    mat4 u_view_proj;
};

layout(location=5) in vec2 i_position;
layout(location=6) in vec4 i_rotation;
layout(location=7) in ivec2 i_sprite;

void main() {
    v_tex = m_tex;
    v_sprite = i_sprite;
    vec2 pos = m_position * mat2(i_rotation.xy, i_rotation.zw) + i_position;
	gl_Position = u_view_proj * vec4(pos, 0.0, 1.0);
}

