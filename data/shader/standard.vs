#version 150 core

in vec3 position;
in vec3 color;
out vec4 v_color;

layout (std140) uniform locals {
    mat4 mvp_transform;
};

void main() {
    v_color = vec4(color, 1.0);
    gl_Position = mvp_transform * vec4(position, 1.0);
    gl_ClipDistance[0] = 1.0;
}
