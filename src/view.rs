use bevy::{platform::collections::HashSet, prelude::*};
use bevy_render::{
    Extract, batching::gpu_preprocessing::GpuPreprocessingMode,
    render_phase::ViewBinnedRenderPhases, view::RetainedViewEntity,
};

use super::mask::MeshOutline3d;

pub(crate) fn update_views(
    mut outline_phases: ResMut<ViewBinnedRenderPhases<MeshOutline3d>>,
    query: Extract<Query<(Entity, &Camera), With<Camera3d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (main_entity, camera) in query.iter() {
        if !camera.is_active {
            continue;
        }

        let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);
        // Binned phases are retained across frames by default. This plugin
        // rebuilds its phase from scratch each frame in `queue_outline`, so
        // drop any existing phase and start from an empty one here.
        outline_phases.0.remove(&retained_view_entity);
        outline_phases.prepare_for_new_frame(
            retained_view_entity,
            GpuPreprocessingMode::PreprocessingOnly,
        );

        live_entities.insert(retained_view_entity);
    }
    outline_phases.retain(|view_entity, _| live_entities.contains(view_entity));
}
