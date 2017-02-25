#version 150 core

const int MAX_LIGHTS = 10;

in vec2 v_tex_coord;
in vec3 frag_position_world;
in vec3 normal_camera;

out vec4 Target0;

uniform sampler2D color_texture;

struct Light {
    vec4 color;
    vec3 position;
    float power;
};

layout (std140) uniform main_camera {
    vec4 cam_position;
};

layout (std140) uniform shared_locals {
    uint num_lights;
};

layout (std140) uniform lights_array {
    Light lights[MAX_LIGHTS];
};

void main() {
    vec3 light_position = lights[0].position;
    vec4 light_color = lights[0].color;
    float light_power = lights[0].power;

    vec4 v_color = texture(color_texture, v_tex_coord);

    // ambient
	vec4 ambient = light_color * 0.1;

    // diffuse
    vec3 norm = normalize(normal_camera);
    vec3 light_direction = normalize(light_position - frag_position_world);
    float diff = max(dot(norm, light_direction), 0.0);
    vec4 diffuse = diff * light_color;

    // specular
	vec4 specular_strength = vec4(0.5, 0.5, 0.5, 1.0);
    vec3 view_direction = normalize(cam_position.xyz - frag_position_world);
    vec3 reflect_direction = reflect(-light_direction, norm);
    float spec = pow(max(dot(view_direction, reflect_direction), 0.0), 32.0);
    vec4 specular = specular_strength * spec * light_color;

	Target0 = (ambient + diffuse + specular) * v_color;
}
