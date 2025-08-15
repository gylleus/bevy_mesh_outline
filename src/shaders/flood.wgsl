#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct JumpFloodUniform {
    @align(16)
    step_length: u32,
}

@group(0) @binding(0) var flood_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> instance: JumpFloodUniform;
@group(0) @binding(3) var depth_texture: texture_depth_2d;
@group(0) @binding(4) var color_texture: texture_2d<f32>;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(flood_texture));
    
    let aspect_ratio = min(dims.x / dims.y, 2.0);
    let step = i32(instance.step_length);

    let current = textureSample(flood_texture, texture_sampler, in.uv);

    var candidate_seed = current;
    var closest_dist = 999999.0;
    var current_depth = 0.0;
    
    let current_screen = in.uv;
    
    if (current.x >= 0.0) {
        let current_pos_screen = current.xy * vec2<f32>(1.0, aspect_ratio);
        closest_dist = distance(current_screen, current_pos_screen);
        current_depth = textureSample(depth_texture, texture_sampler, current.xy);
    } else {
        textureSample(depth_texture, texture_sampler, in.uv);
    }

    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) {
                continue;
            }
            
            let offset = vec2<f32>(f32(dx * step), f32(dy * step)) / dims;
            let neighbor_uv = in.uv + offset;

            let neighbor = textureSample(flood_texture, texture_sampler, neighbor_uv);
            
            // Convert width from pixels to UV space, accounting for aspect ratio
            let width_pixels = floor(neighbor.b);
            let max_dist = width_pixels;  // Convert to UV space

            let candidate_pos = neighbor.xy;
            if (neighbor.x < 0.0) {
                continue;
            }

            // Calculate distance in aspect-ratio-corrected space
            let neighbor_screen = candidate_pos * dims;
            let dist = distance(current_screen * dims, neighbor_screen);
            
            // Scale max_dist by aspect ratio for vertical dimension
            let adjusted_max_dist = max_dist;

            if (dist < adjusted_max_dist) {
                let candidate_depth = textureSample(depth_texture, texture_sampler, candidate_pos);
                let depth_diff = candidate_depth - current_depth;

                if (abs(fract(neighbor.b) - fract(candidate_seed.b)) > 0.001) {
                    if (depth_diff > 0.01) {
                        closest_dist = dist;
                        current_depth = candidate_depth;
                        candidate_seed = neighbor;
                    }
                } else if (dist < closest_dist) {
                    closest_dist = dist;
                    current_depth = candidate_depth;
                    candidate_seed = neighbor;
                }
            }       
        }
    }

    return candidate_seed;
}