#import bevy_pbr::{
    view_transformations::{ndc_to_uv},
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var flood_texture: texture_2d<f32>;
@group(0) @binding(3) var appearance_texture: texture_2d<f32>;
// Global scene depth from the prepass. When the camera has MSAA enabled this
// texture is multisampled and must be read with textureLoad.
#ifdef MULTISAMPLED
@group(0) @binding(4) var depth_texture: texture_depth_multisampled_2d;
#else
@group(0) @binding(4) var depth_texture: texture_depth_2d;
#endif
// Outline depth is owned by the plugin and is always single-sampled.
@group(0) @binding(5) var outline_depth_texture: texture_depth_2d;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(screen_texture, texture_sampler, in.uv);
    let flood_data = textureSample(flood_texture, texture_sampler, in.uv);
    let seed_uv = flood_data.xy;

    // Early return if no outline data
    if seed_uv.x <= 0.0 || seed_uv.y <= 0.0 {
        return color;
    }

    // Get depths
#ifdef MULTISAMPLED
    // Multisampled depth textures can't be sampled; read sample 0 by pixel.
    let current_depth = textureLoad(depth_texture, vec2<i32>(in.clip_position.xy), 0);
#else
    let current_depth = textureSample(depth_texture, texture_sampler, in.uv);
#endif
    let outline_depth = textureSample(outline_depth_texture, texture_sampler, seed_uv);

    // Get appearance data for this outline
    let appearance = textureSample(appearance_texture, texture_sampler, seed_uv);
    let outline_color = appearance.rgb;
    
    // Only render outline when it's behind the current geometry
    if outline_depth > current_depth {
        // Apply outline color
        color = vec4<f32>(outline_color, 1.0);
    }
    
    return color;
}