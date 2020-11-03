#version 450

layout(location = 0) in vec2 Vertex_Position;
layout(location = 1) in vec2 Vertex_Uv;
layout(location = 2) in vec4 Vertex_Color;
layout(location = 3) in uint Vertex_Mode;

layout(location = 0) out vec2 v_Uv;
layout(location = 1) out vec4 v_Color;
layout(location = 2) flat out uint v_Mode;

void main() {
    v_Uv = Vertex_Uv;
    v_Color = Vertex_Color;
    v_Mode = Vertex_Mode;
    gl_Position = vec4(Vertex_Position.x, -Vertex_Position.y, 0.0, 1.0);
}