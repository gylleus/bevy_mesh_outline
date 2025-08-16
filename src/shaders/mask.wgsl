#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::morph,
    forward_io::{Vertex},
    view_transformations::{position_world_to_clip, ndc_to_uv, position_world_to_ndc,frag_coord_to_uv},
}

struct Instance {
    intensity: f32,
    width: f32,
    id: f32,
    priority: f32,
    outline_color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) mesh_id: f32,
    @location(2) outline_width: f32,
    @location(3) priority: f32,
    @location(4) outline_color: vec3<f32>,
};

struct FragmentOutput {
    @location(0) flood_data: vec4<f32>,
    @location(1) appearance_data: vec4<f32>,
}

@group(2) @binding(0) var<uniform> outline_instance: Instance;


#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[vertex.instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;

    let weight_count = bevy_pbr::morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph(vertex_index, bevy_pbr::morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph(vertex_index, bevy_pbr::morph::normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph(vertex_index, bevy_pbr::morph::tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}
#endif


@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

    #ifdef MORPH_TARGETS
        var vertex = morph_vertex(vertex_no_morph);
    #else
        var vertex = vertex_no_morph;
    #endif

    #ifdef SKINNED
        var world_from_local = skinning::skin_model(vertex.joint_indices, vertex.joint_weights,
            vertex.instance_index 
        );
    #else
        let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    #endif

    #ifdef VERTEX_POSITIONS
        out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
        out.position = position_world_to_clip(out.world_position.xyz);
    #endif

    out.outline_width = outline_instance.width;
    out.mesh_id = outline_instance.id;
    out.priority = outline_instance.priority;
    out.outline_color = outline_instance.outline_color;

    return out;
}

@fragment
fn fragment(vertex: VertexOutput) -> FragmentOutput {
    let uv = frag_coord_to_uv(vertex.position.xy);
    let depth = vertex.position.z;
    
    var output: FragmentOutput;
    // RT0: seed_uv.xy, outline_width, depth
    output.flood_data = vec4<f32>(uv, vertex.outline_width, depth);
    // RT1: outline_color.rgb, priority + mesh_id
    output.appearance_data = vec4<f32>(vertex.outline_color * outline_instance.intensity, vertex.priority + vertex.mesh_id);

    return output;
}