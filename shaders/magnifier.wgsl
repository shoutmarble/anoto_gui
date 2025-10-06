#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var<uniform> uv_min: vec2<f32>;

@group(1) @binding(1)
var<uniform> uv_max: vec2<f32>;

@group(1) @binding(2)
var texture: texture_2d<f32>;

@group(1) @binding(3)
var texture_sampler: sampler;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    // Map the fragment coordinates from 0-1 to uv_min-uv_max range
    let uv = uv_min + (uv_max - uv_min) * uv;
    
    // Sample the texture
    let color = textureSample(texture, texture_sampler, uv);
    
    return color;
}