use bevy::{
    ecs::{
        query::ROQueryItem,
        system::{SystemParamItem, lifetimeless::SRes},
    },
    platform::collections::HashMap,
    prelude::*,
};
use bevy_render::{
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{BindGroup, BindGroupEntry, BufferInitDescriptor, PipelineCache},
    renderer::RenderDevice,
    sync_world::MainEntity,
};
use wgpu_types::BufferUsages;

use super::{ExtractedOutlines, mask_pipeline::MeshMaskPipeline, uniforms::OutlineUniform};

pub(crate) struct SetOutlineBindGroup<const I: usize>();

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetOutlineBindGroup<I> {
    type Param = SRes<OutlineBindGroups>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _entity_data: Option<()>,
        outline_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let outline_bind_groups = outline_bind_groups.into_inner();

        if let Some(bind_group) = outline_bind_groups.0.get(&item.main_entity()) {
            pass.set_bind_group(I, bind_group, &[]);
            RenderCommandResult::Success
        } else {
            // Bind group not ready yet, skip this frame
            RenderCommandResult::Skip
        }
    }
}

#[derive(Resource, Default)]
pub struct OutlineBindGroups(HashMap<MainEntity, BindGroup>);

pub fn prepare_outline_bind_groups(
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    outline_pipeline: Res<MeshMaskPipeline>,
    extracted_outlines: Res<ExtractedOutlines>,
    mut outline_bind_groups: ResMut<OutlineBindGroups>,
) {
    outline_bind_groups.0.clear();

    for (entity, outline) in extracted_outlines.0.iter() {
        let outline_uniform = OutlineUniform::from(outline);

        // Create buffer
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("outline_uniform_buffer"),
            contents: bytemuck::cast_slice(&[outline_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // Create bind group
        let bind_group = render_device.create_bind_group(
            Some("outline_bind_group"),
            &pipeline_cache.get_bind_group_layout(&outline_pipeline.outline_bind_group_layout),
            &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        );

        outline_bind_groups.0.insert(*entity, bind_group);
    }
}
