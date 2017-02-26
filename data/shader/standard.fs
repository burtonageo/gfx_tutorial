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
    vec4 v_color = texture(color_texture, v_tex_coord);

    for (uint i = uint(0); i < min(num_lights, MAX_LIGHTS); i++) {
        vec3 light_position = lights[i].position;
        vec4 light_color = lights[i].color;
        float light_power = lights[i].power;

        // ambient
        vec4 ambient = light_color * light_power * 0.0001;

        // diffuse
        vec3 norm = normalize(normal_camera);
        vec3 light_direction = normalize(light_position - frag_position_world);
        float diff = max(dot(norm, light_direction), 0.0);
        vec4 diffuse = diff * light_color;

        // specular
        vec4 specular_strength = vec4(vec3(light_power * 0.1), 1.0);
        vec3 view_direction = normalize(cam_position.xyz - frag_position_world);
        vec3 reflect_direction = reflect(-light_direction, norm);
        float spec = pow(max(dot(view_direction, reflect_direction), 0.0), 32.0);
        vec4 specular = specular_strength * spec * light_color;

        v_color *= (ambient + diffuse + specular);
    }

	Target0 = v_color;
}
