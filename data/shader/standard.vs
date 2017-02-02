#version 150 core

const int MAX_LIGHTS = 10;

in vec3 position;
in vec3 color;
in vec3 normal;

out vec4 v_color;
out vec3 position_world;
out vec3 light_direction_camera;
out vec3 eye_direction_camera;
out vec3 normal_camera;

struct Light {
    vec4 color;
    vec3 position;
    float power;
};

layout (std140) uniform vert_locals {
    mat4 mvp_transform;
    mat4 model_transform;
    mat4 view_transform;
};

layout (std140) uniform shared_locals {
    int num_lights;
};

layout (std140) uniform lights_array {
    Light lights[MAX_LIGHTS];
};

void main() {
    v_color = vec4(color, 1.0);
    vec3 light_position = lights[0].position;

    gl_Position = mvp_transform * vec4(position, 1.0);
    position_world = (model_transform * vec4(position, 1.0)).xyz;

    vec3 vertpos_camera = (view_transform * model_transform * vec4(position, 1.0)).xyz;
    light_direction_camera = vec3(0.0, 0.0, 0.0) - vertpos_camera;

    vec3 lightpos_camera = (view_transform * vec4(light_position, 1.0)).xyz;
    light_direction_camera = lightpos_camera + eye_direction_camera;

    normal_camera = (view_transform * model_transform * vec4(normal, 0.0)).xyz;

    gl_ClipDistance[0] = 1.0;
}
