#version 150 core

in vec4 v_color;
in vec3 position_world;
in vec3 light_direction_camera;
in vec3 eye_direction_camera;
in vec3 normal_camera;

out vec4 Target0;

layout (std140) uniform locals {
    mat4 mvp_transform;
    mat4 model_transform;
    mat4 view_transform;

    vec4 light_color;
    vec3 light_position;
    float light_power;
};

void main() {
	vec3 ambient = vec3(0.1, 0.1, 0.4) * v_color.xyz;
	vec3 specular = vec3(0.1, 0.3, 1.0);

	float distance = length(light_position - position_world);
	float distance_squared = distance * distance;

	vec3 n = normalize(normal_camera);
	vec3 l = normalize(light_direction_camera);
	float cos_theta = clamp(dot(n, l), 0, 1);

	vec3 e = normalize(eye_direction_camera);
	vec3 rd = reflect(-l, n); // Direction in which the light is reflected
	float cos_alpha = clamp(dot(e, rd), 0, 1);

    Target0 = vec4(ambient, 0.8) +
			  v_color * light_color * light_power * cos_theta / distance_squared +
			  vec4(specular, 1.0) * light_color * light_power * pow(cos_alpha, 5) / distance_squared;
}
