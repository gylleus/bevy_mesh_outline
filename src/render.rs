use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_render::{
    render_phase::{RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{BindGroup, BindGroupEntry, BufferInitDescriptor, PipelineCache},
    renderer::RenderDevice,
};
use wgpu_types::BufferUsages;

use super::{
    ExtractedOutlines,
    mask::{MeshOutline3d, OutlineKey},
    mask_pipeline::MeshMaskPipeline,
    uniforms::OutlineUniform,
};

pub(crate) struct SetOutlineBindGroup<const I: usize>();

impl<const I: usize> RenderCommand<MeshOutline3d> for SetOutlineBindGroup<I> {
    type Param = SRes<OutlineBindGroups>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        item: &MeshOutline3d,
        _view: (),
        _entity_data: Option<()>,
        outline_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let outline_bind_groups = outline_bind_groups.into_inner();

        // Every instance in this batch shares the same appearance (it's part of
        // the batch-set key), so a single bind group keyed by that appearance
        // serves the whole batch.
        if let Some(bind_group) = outline_bind_groups.0.get(&item.batch_set_key.outline) {
            pass.set_bind_group(I, bind_group, &[]);
            RenderCommandResult::Success
        } else {
            // Bind group not ready yet, skip this frame
            RenderCommandResult::Skip
        }
    }
}

/// One outline uniform bind group per distinct appearance, cached across frames.
///
/// Because meshes only batch when they share an appearance (see
/// [`crate::mask::OutlineBatchSetKey`]), keying the bind groups by appearance
/// rather than by entity means we allocate O(distinct appearances) GPU resources
/// per frame instead of one buffer + bind group per outlined entity every frame
/// — and in the common case of a stable set of appearances, zero per frame.
#[derive(Resource, Default)]
pub struct OutlineBindGroups(HashMap<OutlineKey, BindGroup>);

pub fn prepare_outline_bind_groups(
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    outline_pipeline: Res<MeshMaskPipeline>,
    extracted_outlines: Res<ExtractedOutlines>,
    mut outline_bind_groups: ResMut<OutlineBindGroups>,
    mut live_keys: Local<HashSet<OutlineKey>>,
) {
    live_keys.clear();

    for outline in extracted_outlines.0.values() {
        let key = OutlineKey::from_outline(outline);
        if !live_keys.insert(key) {
            // Already built (or reused) a bind group for this appearance.
            continue;
        }

        // Only touches the GPU for appearances we haven't cached yet.
        outline_bind_groups.0.entry(key).or_insert_with(|| {
            let outline_uniform = OutlineUniform::from(outline);

            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("outline_uniform_buffer"),
                contents: bytemuck::cast_slice(&[outline_uniform]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            render_device.create_bind_group(
                Some("outline_bind_group"),
                &pipeline_cache.get_bind_group_layout(&outline_pipeline.outline_bind_group_layout),
                &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            )
        });
    }

    // Drop bind groups for appearances no longer in use so the cache stays
    // bounded by the appearances actually on screen.
    outline_bind_groups
        .0
        .retain(|key, _| live_keys.contains(key));
}
