#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var<uniform> uv_min: vec2<f32>;

@group(1) @binding(1)
var<uniform> uv_max: vec2<f32>;

@group(1) @binding(2)
var texture: texture_2d<f32>;

@group(1) @binding(3)
var texture_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fragment(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    // Use UV coordinates from vertex output
    let uv = in.uv;
    
    // Map the UV coordinates from 0-1 to uv_min-uv_max range
    let mapped_u = uv_min.x + (uv_max.x - uv_min.x) * uv.x;
    let mapped_v = uv_max.y - (uv_max.y - uv_min.y) * uv.y; // Flip V
    let mapped_uv = vec2<f32>(mapped_u, mapped_v);
    
    // Sample the texture
    let color = textureSample(texture, texture_sampler, mapped_uv);
    
    return color;
}