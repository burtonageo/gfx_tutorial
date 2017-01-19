#version 150 core

in vec4 v_color;
in vec3 v_normal;

out vec4 Target0;

void main() {
    Target0 = v_color;
}
