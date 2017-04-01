#version 150 core

in vec2 f_TexCoord;
out vec4 Target0;

layout (std140) uniform f_TextLocals {
	vec4 f_TextColor;
};

void main() {
	Target0 = f_TextColor;
}
