#include <metal_stdlib>

using namespace metal;

struct VertexOut {
	float3 pos [[position]];
	float2 uv  [[tex_coord]];
	float3 norm [[normal]];
};

struct FragmentOut {
	float4 main [[color]];
};

fragment Fragment frag(Vertex vertices      		  [[stage_in]],
                       texture2d<float> color_texture [[texture(0)]],
                       sampler color_texture_sampler  [[sampler(0)]]) {
	Fragment out;

	float4 frag_col = color_texture.sample(color_texture_sampler, vertices.uv);
	out.main = frag_col;

	return out;
}
