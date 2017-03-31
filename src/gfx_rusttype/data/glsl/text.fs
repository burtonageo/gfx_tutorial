#version 150 core

in vec2 f_TexCoord;
out vec4 Target0;

layout (std140) uniform f_TextLocals {
	uint f_TextColor;
};

vec4 unpackColor(uint packedColor) {
	return vec4(0.0, 0.0, 1.0, 0.0);
}

void main() {
	Target0 = unpackColor(f_TextColor);
}
