#include <metal_stdlib>

using namespace metal;

struct VertexInput {
    float3 position  [[attribute(0)]];
    float2 tex_coord [[attribute(1)]];
    float3 normal    [[attribute(2)]];
};

struct VertexOutput {
    float4 vertex_position [[position]];
    float2 uv;
    float3 frag_position_world;
    float3 normal_camera;
};

struct VertexUniforms {
    float4x4 mvp_transform;
    float4x4 model_transform;
    float4x4 view_transform;
};

struct SharedUniforms {
    uint num_lights;
};

vertex VertexOutput vert(VertexInput vertices      		[[stage_in]]
                         //constant VertexUniforms& vertex_locals [[buffer(0)]],
                         //constant SharedUniforms& shared_locals [[buffer(1)]],
                         //texture2d<float> color_texture [[texture(0)]],
                         //sampler color_texture_sampler  [[sampler(0)]]
                         ) {
    VertexOutput out;

    out.vertex_position = float4(vertices.position, 1.0); // vertex_locals.mvp_transform * float4(vertices.position, 1.0);
    out.uv = vertices.tex_coord;
    // out.normal_camera = float3x3(transpose(inverse(vertex_locals.model_transform))) * vertices.normal;

    return out;
}
