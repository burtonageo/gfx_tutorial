#include <metal_stdlib>

using namespace metal;

struct VertexInput {
	float3 pos [[position]];
	float2 uv  [[tex_coord]];
	float3 norm [[normal]];
};

struct VertexOutput {
	float2 uv;
	float3 frag_position_world;
	float3 normal_camera;
};

vertex Fragment vert(Vertex vertices      		  [[stage_in]],
				     texture2d<float> color_texture [[texture(0)]],
				     sampler color_texture_sampler  [[sampler(0)]]) {
	Fragment out;

	float4 frag_col = color_texture.sample(color_texture_sampler, vertices.uv);
	out.main = frag_col;

	return out;
}
