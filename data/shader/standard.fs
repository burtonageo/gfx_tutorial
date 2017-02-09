#version 150 core

const int MAX_LIGHTS = 10;

in vec4 v_color;
in vec2 v_uv;

in vec3 position_world;
in vec3 light_direction_camera;
in vec3 eye_direction_camera;
in vec3 normal_camera;

out vec4 Target0;

struct Light {
    vec4 color;
    vec3 position;
    float power;
};

layout (std140) uniform shared_locals {
    int num_lights;
};

layout (std140) uniform lights_array {
    Light lights[MAX_LIGHTS];
};

void main() {
    vec3 light_position = lights[0].position;
    vec4 light_color = lights[0].color;
    float light_power = lights[0].power;

	vec3 ambient = vec3(0.1, 0.1, 0.3) * v_color.xyz;
	vec3 specular = vec3(0.2, 0.2, 0.3);

	float distance = length(light_position - position_world);
	float distance_squared = distance * distance;

	vec3 n = normalize(normal_camera);
	vec3 l = normalize(light_direction_camera);
	float cos_theta = clamp(dot(n, l), 0.0, 1.0);

	vec3 e = normalize(eye_direction_camera);
	vec3 rd = reflect(-l, n); // Direction in which the light is reflected
	float cos_alpha = clamp(dot(e, rd), 0.0, 1.0);

    Target0 = vec4(ambient, 1.0) +
			  (v_color * light_color * light_power * cos_theta / distance_squared) +
			  (vec4(specular, 1.0) * light_color * light_power * pow(cos_alpha, 5) / distance_squared);
}
