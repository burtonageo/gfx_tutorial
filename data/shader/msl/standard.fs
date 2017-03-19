#include <metal_stdlib>

using namespace metal;

struct VertexOutput {
    float4 vertex_position [[position]];
    float2 uv;
    float3 frag_position_world;
    float3 normal_camera;
};

struct FragmentOut {
	float4 main [[color(0)]];
};

fragment FragmentOut frag(VertexOutput vertices          [[stage_in]],
                          texture2d<float> color_texture [[texture(0)]]
                          // sampler color_texture_sampler  [[sampler(0)]]
                          ) {
	FragmentOut out;

	float4 frag_col = color_texture.read(uint2(vertices.uv));
	out.main = frag_col;

	return out;
}
