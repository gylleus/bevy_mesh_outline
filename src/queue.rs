use bevy::{
    pbr::{MeshPipelineKey, RenderMeshInstances, ViewKeyCache},
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

use super::{ExtractedOutline, MeshOutline3d, OutlineCamera, mask_pipeline::MeshMaskPipeline};

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn queue_outline(
    outlined_meshes: Query<(), With<ExtractedOutline>>,
    draw_functions: Res<DrawFunctions<MeshOutline3d>>,
    mut outline_phases: ResMut<ViewBinnedRenderPhases<MeshOutline3d>>,
    mesh_outline_pipeline: Res<MeshMaskPipeline>,
    mut mesh_outline_pipelines: ResMut<SpecializedMeshPipelines<MeshMaskPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    mesh_allocator: Res<MeshAllocator>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    // Per-view base pipeline key (msaa, hdr/target format, prepass bits, ...).
    view_key_cache: Res<ViewKeyCache>,
    views: Query<(&ExtractedView, &RenderVisibleEntities), With<OutlineCamera>>,
) {
    let draw_function = draw_functions.read().id::<DrawOutline>();

    for (view, visible_entities) in views.iter() {
        // The phase was reset to empty for this frame in `update_views`; here we
        // rebuild it from the currently visible, currently outlined meshes.
        let Some(outline_phase) = outline_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        // `RenderVisibleEntities::get` now returns an optional class; iterate all
        // visible mesh entities and keep only the outlined ones.
        let Some(visible_meshes) = visible_entities.get::<Mesh3d>() else {
            continue;
        };

        for (&render_entity, &main_entity) in visible_meshes.iter_visible() {
            if outlined_meshes.get(render_entity).is_err() {
                continue;
            }
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(main_entity)
            else {
                tracing::warn!(target: "bevy_mesh_outline", "No mesh instance found for entity {:?}", main_entity);
                continue;
            };

            let Some(mesh_slabs) = mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id()) else {
                tracing::warn!(target: "bevy_mesh_outline", "No mesh slabs found for entity {:?}", main_entity);
                continue;
            };

            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id()) else {
                tracing::warn!(target: "bevy_mesh_outline", "No mesh found for entity {:?}", main_entity);
                continue;
            };

            // Keep the camera's real MSAA bits in the key so the mask pipeline's
            // mesh view bind group layout (group 0) matches the view's actual
            // `mesh_view_bind_group`, which is keyed on the camera MSAA. The mask
            // pass itself is forced to render single-sampled in
            // `MeshMaskPipeline::specialize`.
            let mut mesh_key = view_key;
            mesh_key |= MeshPipelineKey::from_primitive_topology_and_strip_index(
                mesh.primitive_topology(),
                mesh.index_format(),
            ) | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

            let Ok(pipeline_id) = mesh_outline_pipelines.specialize(
                &pipeline_cache,
                &mesh_outline_pipeline,
                mesh_key,
                &mesh.layout,
            ) else {
                tracing::warn!(target: "bevy_mesh_outline", "Failed to specialize mesh pipeline");
                continue;
            };

            outline_phase.add(
                OutlineBatchSetKey {
                    pipeline: pipeline_id,
                    draw_function,
                    slabs: mesh_slabs,
                },
                OutlineBinKey {
                    asset_id: mesh_instance.mesh_asset_id().untyped(),
                },
                (render_entity, main_entity),
                mesh_instance.current_uniform_index,
                BinnedRenderPhaseType::UnbatchableMesh,
            );
        }
    }
}
