use bevy::{
    core_pipeline::FullscreenShader,
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId,
            FragmentState, PipelineCache, RenderPipelineDescriptor,
            binding_types::{sampler, texture_2d},
        },
        renderer::RenderDevice,
    },
};
use bevy_render::render_resource::binding_types::{
    texture_depth_2d, texture_depth_2d_multisampled,
};
use wgpu_types::{
    ColorTargetState, ColorWrites, MultisampleState, PrimitiveState, SamplerBindingType,
    ShaderStages, TextureFormat, TextureSampleType,
};

use crate::shaders::COMPOSE_SHADER_HANDLE;

#[derive(Clone, Resource)]
pub struct ComposeOutputPipeline {
    /// Bind group layout used when the global depth texture is single-sampled
    /// (MSAA disabled).
    pub layout: BindGroupLayoutDescriptor,
    /// Bind group layout used when the global depth texture is multisampled
    /// (MSAA enabled). Only the global depth binding differs.
    pub layout_multisampled: BindGroupLayoutDescriptor,
    pub pipeline_id: CachedRenderPipelineId,
    pub hdr_pipeline_id: CachedRenderPipelineId,
    pub pipeline_id_multisampled: CachedRenderPipelineId,
    pub hdr_pipeline_id_multisampled: CachedRenderPipelineId,
}

/// Builds the compose bind group layout. When `multisampled` is set the global
/// depth texture (binding 4) is declared as a multisampled depth texture, which
/// is how Bevy's prepass exposes depth when MSAA is enabled. The outline depth
/// texture (binding 5) is owned by this plugin and is always single-sampled.
fn compose_layout(multisampled: bool) -> BindGroupLayoutDescriptor {
    let global_depth = if multisampled {
        texture_depth_2d_multisampled()
    } else {
        texture_depth_2d()
    };

    BindGroupLayoutDescriptor::new(
        "outline_compose_output_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                global_depth,
                texture_depth_2d(),
            ),
        ),
    )
}

impl FromWorld for ComposeOutputPipeline {
    fn from_world(world: &mut World) -> Self {
        let _render_device = world.resource::<RenderDevice>();

        let layout = compose_layout(false);
        let layout_multisampled = compose_layout(true);

        let vertex = world
            .resource::<FullscreenShader>()
            .clone()
            .to_vertex_state();

        // Builds a compose pipeline descriptor for the given output format and
        // MSAA state. The compose pass itself always writes to a single-sampled
        // target; `multisampled` only selects the layout and the `MULTISAMPLED`
        // shader def that switches how the global depth texture is read.
        let make_descriptor = |label: &'static str,
                               layout: BindGroupLayoutDescriptor,
                               format: TextureFormat,
                               multisampled: bool| {
            let shader_defs = if multisampled {
                vec!["MULTISAMPLED".into()]
            } else {
                vec![]
            };

            RenderPipelineDescriptor {
                label: Some(label.into()),
                layout: vec![layout],
                vertex: vertex.clone(),
                fragment: Some(FragmentState {
                    shader: COMPOSE_SHADER_HANDLE,
                    shader_defs,
                    entry_point: Some("fragment".into()),
                    targets: vec![Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                immediate_size: 0,
                zero_initialize_workgroup_memory: false,
            }
        };

        const LDR_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
        const HDR_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

        let ldr = make_descriptor(
            "outline_compose_output_pipeline",
            layout.clone(),
            LDR_FORMAT,
            false,
        );
        let hdr = make_descriptor(
            "outline_compose_output_pipeline_hdr",
            layout.clone(),
            HDR_FORMAT,
            false,
        );
        let ldr_ms = make_descriptor(
            "outline_compose_output_pipeline_msaa",
            layout_multisampled.clone(),
            LDR_FORMAT,
            true,
        );
        let hdr_ms = make_descriptor(
            "outline_compose_output_pipeline_hdr_msaa",
            layout_multisampled.clone(),
            HDR_FORMAT,
            true,
        );

        let cache = world.resource_mut::<PipelineCache>();
        let pipeline_id = cache.queue_render_pipeline(ldr);
        let hdr_pipeline_id = cache.queue_render_pipeline(hdr);
        let pipeline_id_multisampled = cache.queue_render_pipeline(ldr_ms);
        let hdr_pipeline_id_multisampled = cache.queue_render_pipeline(hdr_ms);

        Self {
            layout,
            layout_multisampled,
            pipeline_id,
            hdr_pipeline_id,
            pipeline_id_multisampled,
            hdr_pipeline_id_multisampled,
        }
    }
}
