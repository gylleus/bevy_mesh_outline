mod compose;
mod flood;
mod mask;
mod mask_node;
mod mask_pipeline;
mod queue;
mod render;
mod texture;
mod uniforms;
mod view;

use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    math::Affine3,
    pbr::{DrawMesh, SetMeshBindGroup, SetMeshViewBindGroup, extract_skins},
    prelude::*,
    scene::SceneInstanceReady,
};
use bevy_render::{
    Render, RenderApp, RenderDebugFlags, RenderSet,
    batching::gpu_preprocessing::batch_and_prepare_binned_render_phase,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphApp, RenderLabel, ViewNodeRunner},
    render_phase::{
        AddRenderCommand, BinnedRenderPhasePlugin, DrawFunctions, SetItemPipeline,
        ViewBinnedRenderPhases,
    },
    render_resource::SpecializedMeshPipelines,
    sync_world::{MainEntity, MainEntityHashMap},
};
use compose::ComposeOutputPipeline;
use flood::{JumpFloodPipeline, prepare_flood_settings};
use mask::MeshOutline3d;
use mask_node::OutlineMaskNode;
use mask_pipeline::MeshOutlinePipeline;
use queue::queue_outline;
use rand::Rng;
use render::{OutlineBindGroups, SetOutlineBindGroup, prepare_outline_bind_groups};
use texture::prepare_flood_textures;
use tiny_bail::or_return_quiet;
use view::extract_outline_view_uniforms;

use crate::camera;

// use super::camera::{self};

pub(crate) type DrawOutline = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetOutlineBindGroup<2>,
    DrawMesh,
);

pub struct MeshOutlinePlugin;

impl Plugin for MeshOutlinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<MeshOutline>::default(),
            ExtractComponentPlugin::<OutlineCamera>::default(),
        ));
        app.register_type::<MeshOutline>();

        app.add_plugins(
            BinnedRenderPhasePlugin::<MeshOutline3d, MeshOutlinePipeline>::new(
                RenderDebugFlags::default(),
            ),
        );

        app.add_systems(
            Update,
            (propagate_outline_changes, scale_outline_to_screen_size),
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<DrawFunctions<MeshOutline3d>>()
            .init_resource::<SpecializedMeshPipelines<MeshOutlinePipeline>>()
            .init_resource::<ViewBinnedRenderPhases<MeshOutline3d>>()
            .init_resource::<ExtractedOutlines>()
            .init_resource::<OutlineBindGroups>()
            .add_systems(
                ExtractSchedule,
                (extract_outline_view_uniforms, extract_outlines_for_batch).after(extract_skins),
            )
            .add_systems(
                Render,
                (
                    // prepare_mesh_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    queue_outline.in_set(RenderSet::QueueMeshes),
                    (
                        prepare_flood_settings,
                        prepare_flood_textures,
                        prepare_outline_bind_groups.after(prepare_flood_textures),
                    )
                        .in_set(RenderSet::PrepareBindGroups),
                    batch_and_prepare_binned_render_phase::<MeshOutline3d, MeshOutlinePipeline>
                        .in_set(RenderSet::PrepareResources),
                ),
            )
            .add_render_command::<MeshOutline3d, DrawOutline>()
            .add_render_graph_node::<ViewNodeRunner<OutlineMaskNode>>(
                Core3d,
                OutlineNode::MeshOutlineMaskPass,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    OutlineNode::MeshOutlineMaskPass,
                    Node3d::Bloom,
                ),
            );

        // app.add_systems(Update, apply_recursively.r);
        app.add_observer(apply_recursively);
        // .add_render_command()
    }

    fn finish(&self, app: &mut App) {
        // We need to get the render app from the main app
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        // The pipeline needs the RenderDevice to be created and it's only available once plugins
        // are initialized
        // render_app.init_resource::<StencilPipeline>();
        render_app
            .init_resource::<MeshOutlinePipeline>()
            .init_resource::<JumpFloodPipeline>()
            .init_resource::<ComposeOutputPipeline>();

        // let render_app = app.sub_app_mut(RenderApp);
        // let render_device = render_app.world().resource::<RenderDevice>();
        // let instance_buffer = BatchedInstanceBuffer::<MeshUniform>::new(render_device);

        // render_app.insert_resource(instance_buffer).add_systems(
        //     Render,
        //     write_batched_instance_buffer::<MeshOutlinePipeline>
        //         .in_set(RenderSet::PrepareResourcesFlush),
        // );

        // let gpu_preprocessing_support = render_app.world().resource::<GpuPreprocessingSupport>();
        // if gpu_preprocessing_support.is_available() {
        //     render_app.add_systems(
        //         Render,
        //         add_dummy_phase_buffers.in_set(RenderSet::PrepareResourcesCollectPhaseBuffers),
        //     );
        // }
    }
}

#[derive(Debug, Component, Reflect, Clone, ExtractComponent)]
#[reflect(Component)]
pub struct OutlineCamera;

#[derive(Debug, Component, Reflect, Clone)]
#[reflect(Component)]
pub struct MeshOutline {
    pub highlight: f32,
    pub width: f32,
    pub scaled_width: f32,
    pub id: f32,
}

impl MeshOutline {
    pub fn new(width: f32) -> Self {
        let rng = &mut rand::rng();
        Self {
            highlight: 0.0,
            width,
            scaled_width: width,
            id: rng.random(),
        }
    }
}

#[derive(Debug, Component, Reflect, Clone, PartialEq)]
pub struct ExtractedOutline {
    pub highlight: f32,
    pub width: f32,
    pub id: f32,
    pub world_from_local: [Vec4; 3],
}

impl ExtractComponent for MeshOutline {
    type QueryData = (Entity, &'static MeshOutline, &'static GlobalTransform);

    type QueryFilter = With<Mesh3d>;
    type Out = ExtractedOutline;

    fn extract_component(
        (_entity, outline, transform): bevy::ecs::query::QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        Some(ExtractedOutline {
            highlight: outline.highlight,
            width: outline.scaled_width,
            id: outline.id,
            world_from_local: Affine3::from(&transform.affine()).to_transpose(),
        })
    }
}

fn apply_recursively(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    outline_instances: Query<&MeshOutline>,
    meshes: Query<&Mesh3d>,
    children: Query<&Children>,
    // spawner: Res<SceneSpawner>,
) {
    let outline = or_return_quiet!(outline_instances.get(trigger.target()));

    for child in children.iter_descendants(trigger.target()) {
        if meshes.contains(child) {
            commands.entity(child).insert(outline.clone());
        }
    }
}

#[derive(Resource, Clone, Default)]
pub struct ExtractedOutlines(MainEntityHashMap<ExtractedOutline>);

fn extract_outlines_for_batch(
    mut extracted_outlines: ResMut<ExtractedOutlines>,
    outlines: Query<(&MainEntity, &ExtractedOutline)>,
) {
    extracted_outlines.0.clear();
    for (main_entity, outline) in outlines.iter() {
        extracted_outlines.0.insert(*main_entity, outline.clone());
    }
}

#[derive(Copy, Clone, Debug, RenderLabel, Hash, PartialEq, Eq)]
pub enum OutlineNode {
    MeshOutlineMaskPass,
}

fn propagate_outline_changes(
    mut commands: Commands,
    changed: Query<(Entity, &MeshOutline), Changed<MeshOutline>>,
    meshes: Query<&Mesh3d>,
    children: Query<&Children>,
) {
    for (entity, outline) in changed.iter() {
        for child in children.iter_descendants(entity) {
            if meshes.contains(child) {
                commands.entity(child).insert(outline.clone());
            }
        }
    }
}

const OUTLINE_TARGET_PIXEL_SCALE: Vec2 = Vec2::new(1.0 / 2560.0, 1.0 / 1440.0);

fn scale_outline_to_screen_size(
    mut outlines: Query<(&mut MeshOutline, &Transform)>,
    camera: Single<(&Projection, &Transform), With<OutlineCamera>>,
    window: Single<&Window>,
) {
    let (projection, camera_transform) = camera.into_inner();

    for (mut outline, transform) in outlines.iter_mut() {
        let mut scaled_factor =
            OUTLINE_TARGET_PIXEL_SCALE * window.resolution.physical_size().as_vec2();

        match projection {
            Projection::Orthographic(orthographic) => {
                let min_zoom_scale = 0.5;
                let normalized_zoom = (orthographic.scale
                    - camera::ORTHOGRAPHIC_PROJECTION_SCALE_MIN)
                    / (camera::ORTHOGRAPHIC_PROJECTION_SCALE_MAX
                        - camera::ORTHOGRAPHIC_PROJECTION_SCALE_MIN);

                let zoom_scale = 1.0 - (1.0 - min_zoom_scale) * normalized_zoom;
                scaled_factor *= zoom_scale;
            }
            Projection::Perspective(perspective) => {
                // Calculate distance from camera to mesh
                let distance = (camera_transform.translation - transform.translation).length();

                // Calculate scale based on distance and FOV
                // For perspective projection, objects appear smaller as they get farther away
                // The scale factor should be inversely proportional to distance
                let fov_scale = perspective.fov / (std::f32::consts::PI / 4.0); // Normalize to 45 degrees
                let distance_scale = 1.0 / (distance * 0.1); // Scale inversely with distance

                scaled_factor *= distance_scale * fov_scale;
            }
            Projection::Custom(_) => {
                // For custom projections, use a default scaling factor
                // This could be extended to handle specific custom projection types
                scaled_factor *= 1.0;
            }
        }

        // Use average scaling instead of minimum to avoid aspect ratio issues
        let average_scale = (scaled_factor.x + scaled_factor.y) * 0.5;
        outline.scaled_width = outline.width * average_scale;

        // Use a softer minimum and avoid ceil() for smoother scaling
        outline.scaled_width = outline.scaled_width.max(3.0);

        // Optional: round to nearest 0.5 for smoother transitions
        // outline.scaled_width = (outline.scaled_width * 2.0).round() * 0.5;
    }
}
