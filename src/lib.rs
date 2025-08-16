mod compose;
mod flood;
mod mask;
mod mask_pipeline;
mod node;
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
use mask_pipeline::MeshMaskPipeline;
use node::MeshOutlineNode;
use queue::queue_outline;
use render::{OutlineBindGroups, SetOutlineBindGroup, prepare_outline_bind_groups};
use texture::prepare_flood_textures;
use view::update_views;

use crate::shaders::load_shaders;

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
        load_shaders(app);

        app.add_plugins((
            ExtractComponentPlugin::<MeshOutline>::default(),
            ExtractComponentPlugin::<OutlineCamera>::default(),
        ));
        app.register_type::<MeshOutline>();

        app.add_plugins(
            BinnedRenderPhasePlugin::<MeshOutline3d, MeshMaskPipeline>::new(
                RenderDebugFlags::default(),
            ),
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<DrawFunctions<MeshOutline3d>>()
            .init_resource::<SpecializedMeshPipelines<MeshMaskPipeline>>()
            .init_resource::<ViewBinnedRenderPhases<MeshOutline3d>>()
            .init_resource::<ExtractedOutlines>()
            .init_resource::<OutlineBindGroups>()
            .add_systems(
                ExtractSchedule,
                (update_views, extract_outlines_to_resource).after(extract_skins),
            )
            .add_systems(
                Render,
                (
                    queue_outline.in_set(RenderSet::QueueMeshes),
                    (
                        prepare_flood_settings,
                        prepare_flood_textures,
                        prepare_outline_bind_groups.after(prepare_flood_textures),
                    )
                        .in_set(RenderSet::PrepareBindGroups),
                    batch_and_prepare_binned_render_phase::<MeshOutline3d, MeshMaskPipeline>
                        .in_set(RenderSet::PrepareResources),
                ),
            )
            .add_render_command::<MeshOutline3d, DrawOutline>()
            .add_render_graph_node::<ViewNodeRunner<MeshOutlineNode>>(
                Core3d,
                OutlineNode::MeshOutlineNode,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    OutlineNode::MeshOutlineNode,
                    Node3d::Bloom,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<MeshMaskPipeline>()
            .init_resource::<JumpFloodPipeline>()
            .init_resource::<ComposeOutputPipeline>();
    }
}

/// Marker component for enabling a 3D camera to render mesh outlines.
#[derive(Debug, Component, Reflect, Clone, ExtractComponent)]
#[reflect(Component)]
pub struct OutlineCamera;

/// Adds a mesh outline effect to entity.
/// Should be added to the entity containing the Mesh3d component.
#[derive(Debug, Component, Reflect, Clone)]
#[reflect(Component)]
pub struct MeshOutline {
    pub intensity: f32,
    pub width: f32,
    pub priority: u32,
    pub color: Color,
}

impl MeshOutline {
    pub fn new(width: f32) -> Self {
        Self {
            intensity: 1.0,
            width,
            priority: 0,
            color: Color::BLACK,
        }
    }

    pub fn with_intensity(self, intensity: f32) -> Self {
        Self { intensity, ..self }
    }

    pub fn with_priority(self, priority: u32) -> Self {
        Self { priority, ..self }
    }

    pub fn with_color(self, color: Color) -> Self {
        Self { color, ..self }
    }
}

#[derive(Debug, Component, Reflect, Clone, PartialEq)]
pub struct ExtractedOutline {
    pub intensity: f32,
    pub width: f32,
    pub id: f32,
    pub priority: f32,
    pub color: Vec4,
    pub world_from_local: [Vec4; 3],
}

impl ExtractComponent for MeshOutline {
    type QueryData = (Entity, &'static MeshOutline, &'static GlobalTransform);

    type QueryFilter = With<Mesh3d>;
    type Out = ExtractedOutline;

    fn extract_component(
        (_entity, outline, transform): bevy::ecs::query::QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        let linear_color: LinearRgba = outline.color.into();
        Some(ExtractedOutline {
            intensity: outline.intensity,
            width: outline.width,
            id: 0.0,
            priority: outline.priority as f32,
            color: linear_color.to_vec4(),
            world_from_local: Affine3::from(&transform.affine()).to_transpose(),
        })
    }
}

#[derive(Resource, Clone, Default)]
pub struct ExtractedOutlines(MainEntityHashMap<ExtractedOutline>);

fn extract_outlines_to_resource(
    mut extracted_outlines: ResMut<ExtractedOutlines>,
    outlines: Query<(&MainEntity, &ExtractedOutline)>,
) {
    extracted_outlines.0.clear();
    // Counter for mesh_ids
    let mut current_id = 0.0;
    let total_outlines = outlines.iter().count();

    for (main_entity, outline) in outlines.iter() {
        let mut outline = outline.clone();
        current_id += (1.0 - 0.01) / total_outlines as f32;
        outline.id = current_id;
        extracted_outlines.0.insert(*main_entity, outline);
    }
}

#[derive(Copy, Clone, Debug, RenderLabel, Hash, PartialEq, Eq)]
pub enum OutlineNode {
    MeshOutlineNode,
}
