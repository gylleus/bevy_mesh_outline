use bevy::{math::Affine3A, platform::collections::HashSet, prelude::*};
use bevy_render::{
    Extract, batching::gpu_preprocessing::GpuPreprocessingMode,
    render_phase::ViewBinnedRenderPhases, render_resource::ShaderType, sync_world::RenderEntity,
    view::RetainedViewEntity,
};

use super::mask::MeshOutline3d;

#[derive(Clone, Component, ShaderType)]
pub(crate) struct OutlineViewUniform {
    #[align(16)]
    clip_from_world: Mat4,
    world_from_view_a: [Vec4; 2],
    world_from_view_b: f32,
    aspect: f32,
    scale: Vec2,
}

pub(crate) fn extract_outline_view_uniforms(
    mut commands: Commands,
    mut outline_phases: ResMut<ViewBinnedRenderPhases<MeshOutline3d>>,
    query: Extract<Query<(Entity, &RenderEntity, &Camera, &GlobalTransform), With<Camera3d>>>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    fn transpose_3x3(m: &Affine3A) -> ([Vec4; 2], f32) {
        let transpose_3x3 = m.matrix3.transpose();
        (
            [
                (transpose_3x3.x_axis, transpose_3x3.y_axis.x).into(),
                (transpose_3x3.y_axis.yz(), transpose_3x3.z_axis.xy()).into(),
            ],
            transpose_3x3.z_axis.z,
        )
    }

    for (main_entity, entity, camera, transform) in query.iter() {
        if !camera.is_active {
            continue;
        }

        if let Some(size) = camera.logical_viewport_size() {
            let view_from_world = transform.compute_matrix().inverse();
            let (world_from_view_a, world_from_view_b) = transpose_3x3(&transform.affine());
            commands.entity(entity.id()).insert(OutlineViewUniform {
                clip_from_world: camera.clip_from_view() * view_from_world,
                world_from_view_a,
                world_from_view_b,
                aspect: size.x / size.y,
                scale: 2.0 / size,
            });

            let retained_view_entity = RetainedViewEntity::new(main_entity.into(), None, 0);
            outline_phases.prepare_for_new_frame(
                retained_view_entity,
                GpuPreprocessingMode::PreprocessingOnly,
            );

            live_entities.insert(retained_view_entity);
        }
    }
    outline_phases.retain(|view_entity, _| live_entities.contains(view_entity));
}
