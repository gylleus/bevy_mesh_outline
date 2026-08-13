use bevy::{platform::collections::HashSet, prelude::*};
use bevy_render::{
    Extract,
    batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    render_phase::ViewBinnedRenderPhases,
    view::{NoIndirectDrawing, RetainedViewEntity},
};

use super::mask::MeshOutline3d;

#[allow(clippy::type_complexity)]
pub(crate) fn update_views(
    mut outline_phases: ResMut<ViewBinnedRenderPhases<MeshOutline3d>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    query: Extract<Query<(Entity, &Camera, Has<NoIndirectDrawing>), With<Camera3d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (main_entity, camera, no_indirect_drawing) in query.iter() {
        if !camera.is_active {
            continue;
        }

        // Choose the preprocessing mode exactly as `bevy_core_pipeline::core_3d`
        // does for the opaque/alpha-mask phases (GPU culling + indirect mode when
        // available, otherwise plain preprocessing). The outline phase reuses the
        // same meshes and shares this view's retained entity, so the modes must
        // match for the batch tiers and indirect-parameter buffers to line up.
        let gpu_preprocessing_mode = gpu_preprocessing_support.min(if !no_indirect_drawing {
            GpuPreprocessingMode::Culling
        } else {
            GpuPreprocessingMode::PreprocessingOnly
        });

        let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);
        // Binned phases are retained across frames by default. This plugin
        // rebuilds its phase from scratch each frame in `queue_outline`, so
        // drop any existing phase and start from an empty one here.
        outline_phases.0.remove(&retained_view_entity);
        outline_phases.prepare_for_new_frame(retained_view_entity, gpu_preprocessing_mode);

        live_entities.insert(retained_view_entity);
    }
    outline_phases.retain(|view_entity, _| live_entities.contains(view_entity));
}
