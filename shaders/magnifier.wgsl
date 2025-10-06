#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var<uniform> uv_rect: vec4<f32>;

@group(1) @binding(1)
var texture: texture_2d<f32>;

@group(1) @binding(2)
var texture_sampler: sampler;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    // Map the fragment coordinates to UV coordinates within the uv_rect
    let uv = (uv - uv_rect.xy) / (uv_rect.zw - uv_rect.xy);
    
    // Sample the texture
    let color = textureSample(texture, texture_sampler, uv);
    
    return color;
}