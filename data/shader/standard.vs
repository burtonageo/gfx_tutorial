#version 150 core

const int MAX_LIGHTS = 10;

in vec3 position;
in vec2 tex_coord;
in vec3 normal;

out vec2 v_tex_coord;

out vec3 frag_position_world;
out vec3 normal_camera;

layout (std140) uniform vert_locals {
    mat4 mvp_transform;
    mat4 model_transform;
    mat4 view_transform;
};

void main() {
    v_tex_coord = tex_coord;
    gl_Position = mvp_transform * vec4(position, 1.0);
    frag_position_world = (model_transform * vec4(position, 1.0)).xyz;
    normal_camera = mat3(transpose(inverse(model_transform))) * normal;

    gl_ClipDistance[0] = 1.0;
}
