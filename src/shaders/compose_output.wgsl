#import bevy_pbr::{
    view_transformations::{ndc_to_uv},
}


@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var outline_texture: texture_2d<f32>;
@group(0) @binding(3) var flood_texture: texture_2d<f32>;
@group(0) @binding(4) var depth_texture: texture_depth_2d;
@group(0) @binding(5) var outline_depth_texture: texture_depth_2d;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(screen_texture, texture_sampler, in.uv);
    let outline_vals = textureSample(flood_texture, texture_sampler, in.uv);
    var uv = outline_vals.xy;

    if uv.x <= 0 || uv.y <= 0 {
        return color;
    }

    let depth = textureSample(depth_texture, texture_sampler, in.uv);
    let outline_depth = textureSample(outline_depth_texture, texture_sampler, uv);

    let highlight_strength = outline_vals.a;

    if outline_depth > depth {
        if highlight_strength > 20.0 {
            color = vec4<f32>(1.0, 0.0, 0.0, 1.0) * (highlight_strength - 20.0);
        } else if highlight_strength > 10.0 {
            let c = 0.5;
            color = vec4<f32>(c,c,c, 1.0) * (highlight_strength - 10.0);
        } else {
            color = vec4<f32>(1.0, 1.0, 0.0, 1.0) * (highlight_strength);
        }
    }

    return color;
}