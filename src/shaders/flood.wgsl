#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

// Jump Flood Algorithm for outline distance field generation
// Iteratively propagates outline seed information across the texture
// by checking neighbors at exponentially decreasing step sizes

struct JumpFloodUniform {
    @align(16)
    step_length: u32,
}

@group(0) @binding(0) var flood_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> instance: JumpFloodUniform;
@group(0) @binding(3) var depth_texture: texture_depth_2d;
@group(0) @binding(4) var color_texture: texture_2d<f32>;
@group(0) @binding(5) var appearance_texture: texture_2d<f32>;

fn calculate_priority(candidate_depth: f32, mesh_priority: f32) -> f32 {
    let depth_factor = (1.0 - candidate_depth) * 10.0;  // Closer is better
    return depth_factor + mesh_priority;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(flood_texture));
    let step = i32(instance.step_length);

    let current = textureSample(flood_texture, texture_sampler, in.uv);
    var best_candidate = current;
    var best_priority = -999999.0;
    
    // If current pixel has valid seed data, calculate its priority
    if (current.x >= 0.0) {
        let current_depth = current.w;
        let current_appearance = textureSample(appearance_texture, texture_sampler, current.xy);
        let current_mesh_priority = floor(current_appearance.w);  // Extract integer priority from packed float
        best_priority = calculate_priority(current_depth, current_mesh_priority);
    }

    // Check all 8 neighbors
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) {
                continue;
            }
            
            // Sample neighbors at current step distance (starts large, gets smaller each pass)
            let offset = vec2<f32>(f32(dx * step), f32(dy * step)) / dims;
            let neighbor_uv = in.uv + offset;
            let neighbor = textureSample(flood_texture, texture_sampler, neighbor_uv);
            
            // Skip invalid neighbors
            if (neighbor.x < 0.0) {
                continue;
            }

            let seed_pos = neighbor.xy;
            let outline_width = neighbor.z;
            let seed_depth = neighbor.w;
            
            // Calculate distance from current pixel to seed
            let dist = distance(in.uv * dims, seed_pos * dims);
            
            // Only consider pixels within outline range
            if (dist >= outline_width) {
                continue;
            }

            // Get appearance data for this seed
            let appearance = textureSample(appearance_texture, texture_sampler, seed_pos);
            let mesh_priority = floor(appearance.w);  // Extract integer priority from packed float
            
            // Calculate priority for this candidate
            let candidate_priority = calculate_priority(seed_depth, mesh_priority);
            
            // Update best candidate if this one has higher priority
            if (candidate_priority > best_priority) {
                best_candidate = neighbor;
                best_priority = candidate_priority;
            }
        }
    }

    return best_candidate;
}