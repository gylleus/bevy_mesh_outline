use bevy::{core_pipeline::prepass::ViewPrepassTextures, ecs::query::QueryItem, prelude::*};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::ViewBinnedRenderPhases,
    render_resource::{
        BindGroupEntries, LoadOp, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureViewDescriptor,
    },
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};
use wgpu_types::ImageSubresourceRange;

use crate::MeshOutline3d;

use super::{
    compose::ComposeOutputPipeline,
    flood::{FloodSettings, JumpFloodPass},
    texture::FloodTextures,
};

#[derive(Default)]
pub struct OutlineMaskNode;
impl ViewNode for OutlineMaskNode {
    type ViewQuery = (
        Entity,
        &'static ExtractedView,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static FloodTextures,
        &'static ViewPrepassTextures,
        &'static FloodSettings,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            view_entity,
            extracted_view,
            camera,
            view_target,
            flood_textures,
            prepass_textures,
            flood_settings,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        if let Some(target_size) = camera.physical_target_size {
            let current_size = flood_textures.input.texture.size();
            if current_size.width != target_size.x || current_size.height != target_size.y {
                // Trigger texture recreation
                // commands.entity(view).remove::<FloodTextures>();
                tracing::warn!("Texture size mismatch, recreating flood textures");
            }
        }

        let Some(outline_phases) = world.get_resource::<ViewBinnedRenderPhases<MeshOutline3d>>()
        else {
            tracing::warn!("No outline phases found in the world");
            return Ok(());
        };

        let Some(outline_phase) = outline_phases.get(&extracted_view.retained_view_entity) else {
            tracing::warn!(
                "No outline phases found for view {:?}",
                extracted_view.retained_view_entity
            );
            return Ok(());
        };

        let Some(mut jump_flood_pass) = JumpFloodPass::new(world) else {
            return Ok(());
        };
        let mut flood_textures = flood_textures.clone();
        let Some(global_depth) = prepass_textures.depth.as_ref() else {
            return Ok(());
        };

        // let color_attachments = [Some(target.get_color_attachment())];

        render_context.command_encoder().clear_texture(
            &flood_textures.input.texture,
            &ImageSubresourceRange::default(),
        );
        render_context.command_encoder().clear_texture(
            &flood_textures.output.texture,
            &ImageSubresourceRange::default(),
        );
        render_context.command_encoder().clear_texture(
            &flood_textures.outline_color_storage.texture,
            &ImageSubresourceRange::default(),
        );

        render_context.command_encoder().clear_texture(
            &flood_textures.outline_depth_texture,
            &ImageSubresourceRange::default(),
        );

        let color_attachment = RenderPassColorAttachment {
            view: &flood_textures.output.default_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(wgpu_types::Color {
                    r: -1.0,
                    g: -1.0,
                    b: -1.0,
                    a: 0.0,
                }),
                store: StoreOp::Store,
            },
        };

        let outline_depth_view = flood_textures
            .outline_depth_texture
            .create_view(&TextureViewDescriptor::default());

        {
            let mut init_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("outline_flood_init"),
                color_attachments: &[Some(color_attachment)],
                // depth_stencil_attachment: depth_stencil_attachment,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &outline_depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        // load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                init_pass.set_camera_viewport(viewport);
            }

            if let Err(err) = outline_phase.render(&mut init_pass, world, view_entity) {
                error!("Error encountered while rendering the outline flood init phase {err:?}");
            }
        }

        let Some(compose_pipeline) = world.get_resource::<ComposeOutputPipeline>() else {
            tracing::warn!("No compose pipeline found in the world");
            return Ok(());
        };
        let pipeline_cache = world.resource::<PipelineCache>();

        // Get the pipeline from the cache
        let Some(pipeline) = pipeline_cache.get_render_pipeline(compose_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        // Flooding!

        let outline_width: f32 = flood_settings.width;

        let passes = if outline_width > 0.0 {
            ((outline_width * 2.0).ceil() as u32 / 2 + 1)
                .next_power_of_two()
                .trailing_zeros()
                + 1
        } else {
            0
        };

        for size in (0..passes).rev() {
            flood_textures.flip();
            jump_flood_pass.execute(
                render_context,
                flood_textures.input(),
                flood_textures.output(),
                &outline_depth_view,
                &flood_textures.outline_color_storage.default_view,
                size,
            );
        }

        let bind_group = render_context.render_device().create_bind_group(
            "compose_output_bind_group",
            &compose_pipeline.layout,
            // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
            &BindGroupEntries::sequential((
                // The original scene color
                post_process.source,
                // Use the sampler created for the pipeline
                &jump_flood_pass.pipeline.sampler,
                // The outline colors
                &flood_textures.outline_color_storage.default_view,
                // Make sure to use the source view
                &flood_textures.output.default_view,
                // Global depth texture
                &global_depth.texture.default_view,
                // Use the outline depth texture
                &outline_depth_view,
            )),
        );

        // Composite pass
        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("post_process_pass"),
                color_attachments: &[Some(view_target.get_color_attachment())],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        Ok(())
    }
}
