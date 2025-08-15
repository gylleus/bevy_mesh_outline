use bevy::{
    ecs::component::Tick,
    pbr::{MeshPipelineKey, RenderMeshInstances},
    prelude::*,
};
use bevy_render::{
    mesh::{RenderMesh, allocator::MeshAllocator},
    render_asset::RenderAssets,
    render_phase::{BinnedRenderPhaseType, DrawFunctions, ViewBinnedRenderPhases},
    render_resource::{PipelineCache, SpecializedMeshPipelines},
    view::{ExtractedView, RenderVisibleEntities},
};

use crate::{
    DrawOutline,
    mask::{OutlineBatchSetKey, OutlineBinKey},
};

use super::{ExtractedOutline, MeshOutline3d, OutlineCamera, mask_pipeline::MeshOutlinePipeline};

pub fn queue_outline(
    outlined_meshes: Query<&ExtractedOutline>,
    draw_functions: Res<DrawFunctions<MeshOutline3d>>,
    mut mask_phases: ResMut<ViewBinnedRenderPhases<MeshOutline3d>>,
    mesh_outline_pipeline: Res<MeshOutlinePipeline>,
    mut mesh_outline_pipelines: ResMut<SpecializedMeshPipelines<MeshOutlinePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mesh_allocator: Res<MeshAllocator>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    views: Query<(Entity, &ExtractedView, &RenderVisibleEntities, &Msaa), With<OutlineCamera>>,
    mut change_tick: Local<Tick>,
) {
    // Get the id for our custom draw function
    let draw_function = draw_functions.read().id::<DrawOutline>();

    for (_view_entity, view, visible_entities, msaa) in views.iter() {
        let Some(mask_phase) = mask_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        // Create the key based on the view. In this case we only care about MSAA and HDR
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::NORMAL_PREPASS
            | MeshPipelineKey::DEPTH_PREPASS
            | MeshPipelineKey::from_hdr(view.hdr);

        for &(render_entity, main_entity) in visible_entities.get::<Mesh3d>().iter() {
            if !outlined_meshes.get(render_entity).is_ok() {
                // tracing::warn!(target: "bevy_mesh_outline", "No outline found for entity {:?}", render_entity);
                continue;
            }
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(main_entity)
            else {
                tracing::warn!(target: "bevy_mesh_outline", "No mesh instance found for entity {:?}", main_entity);
                continue;
            };

            let (vertex_slab, index_slab) = mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);

            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                tracing::warn!(target: "bevy_mesh_outline", "No mesh found for entity {:?}", main_entity);
                continue;
            };

            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology())
                | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

            let pipeline_id = mesh_outline_pipelines
                .specialize(
                    &pipeline_cache,
                    &mesh_outline_pipeline,
                    mesh_key,
                    &mesh.layout,
                )
                // This should never with this example, but if your pipeline specialization
                // can fail you need to handle the error here
                .expect("Failed to specialize mesh pipeline");

            let next_change_tick = change_tick.get() + 1;
            change_tick.set(next_change_tick);

            mask_phase.add(
                OutlineBatchSetKey {
                    pipeline: pipeline_id,
                    draw_function,
                    vertex_slab: vertex_slab.unwrap_or_default(),
                    index_slab,
                },
                OutlineBinKey {
                    asset_id: mesh_instance.mesh_asset_id.untyped(),
                },
                (render_entity, main_entity),
                mesh_instance.current_uniform_index,
                BinnedRenderPhaseType::UnbatchableMesh,
                *change_tick,
            );
        }
    }
}
