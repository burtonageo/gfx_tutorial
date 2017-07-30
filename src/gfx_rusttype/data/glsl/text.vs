#version 150 core

in vec3 v_Pos;
in vec2 v_Tex;

out vec2 f_TexCoord;

void main() {
	gl_Position = vec4(v_Pos, 1.0);
	f_TexCoord = v_Tex;
}
