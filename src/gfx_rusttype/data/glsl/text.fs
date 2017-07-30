#version 150 core

in vec2 f_TexCoord;
out vec4 Target0;

uniform sampler2D f_TextSampler;

layout (std140) uniform f_TextLocals {
	vec4 f_TextColor;
};

void main() {
	Target0 = texture(f_TextSampler, f_TexCoord);
}
