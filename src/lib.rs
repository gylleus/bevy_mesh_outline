mod compose;
mod flood;
mod mask;
mod mask_node;
mod mask_pipeline;
mod queue;
mod render;
mod shaders;
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
use view::extract_outline_view_uniforms;

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

        app.add_systems(Update, propagate_outline_changes);

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

        app.add_observer(apply_recursively);
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<MeshOutlinePipeline>()
            .init_resource::<JumpFloodPipeline>()
            .init_resource::<ComposeOutputPipeline>();
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
) {
    let Ok(outline) = outline_instances.get(trigger.target()) else {
        return;
    };

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
