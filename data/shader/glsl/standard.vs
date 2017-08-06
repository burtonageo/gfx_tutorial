#version 150 core

const int MAX_LIGHTS = 10;

in vec3 position;
in vec2 tex_coord;
in vec3 normal;

out vec2 v_tex_coord;
out vec3 frag_position_world;
out vec3 normal_camera;
out mat4 model_view_matrix;

layout (std140) uniform vert_locals {
    mat4 projection_matrix;
    mat4 model_matrix;
    mat4 view_matrix;
};

void main() {
    model_view_matrix = view_matrix * model_matrix;
    mat4 mvp = projection_matrix * model_view_matrix;
    v_tex_coord = tex_coord;
    gl_Position = mvp * vec4(position, 1.0);
    frag_position_world = (model_matrix * vec4(position, 1.0)).xyz;
    normal_camera = mat3(transpose(inverse(model_matrix))) * normal;

    gl_ClipDistance[0] = 1.0;
}
