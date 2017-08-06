#version 150 core

const int MAX_LIGHTS = 10;

in vec2 v_tex_coord;
in vec3 frag_position_world;
in vec3 normal_camera;
in mat4 model_view_matrix;

out vec4 Target0;

uniform sampler2D color_texture;

struct Light {
    vec4 color;
    vec3 position;
    float power;
};

layout (std140) uniform shared_locals {
    uint num_lights;
};

layout (std140) uniform lights_array {
    Light lights[MAX_LIGHTS];
};

vec4 extract_camera_position(mat4 model_view) {
    mat4 view_model = inverse(model_view);
    return view_model[3];
}

void main() {
    vec4 total_lighting = vec4(0.0, 0.0, 0.0, 0.0);
    vec4 cam_position = extract_camera_position(model_view_matrix);

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

        total_lighting += (ambient + diffuse + specular);
    }

	Target0 = texture(color_texture, v_tex_coord) * total_lighting;
}
