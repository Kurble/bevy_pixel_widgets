#version 450

layout(set = 0, binding = 0) uniform texture2D t_Color;
layout(set = 0, binding = 1) uniform sampler s_Color;

layout(location = 0) in vec2 v_Uv;
layout(location = 1) in vec4 v_Color;
layout(location = 2) in float v_Mode;

layout(location = 0) out vec4 Target0;

void main() {
    vec4 color = texture(sampler2D(t_Color, s_Color), v_Uv);
    color.x = mix(color.x, 1.0, v_Mode);
    color.y = mix(color.y, 1.0, v_Mode);
    color.z = mix(color.z, 1.0, v_Mode);
    color.w = mix(color.w, 1.0, v_Mode);
    Target0 = v_Color * color;
}