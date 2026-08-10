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
use bevy_render::render_resource::binding_types::texture_depth_2d;
use wgpu_types::{
    ColorTargetState, ColorWrites, MultisampleState, PrimitiveState, SamplerBindingType,
    ShaderStages, TextureFormat, TextureSampleType,
};

use crate::shaders::COMPOSE_SHADER_HANDLE;

#[derive(Clone, Resource)]
pub struct ComposeOutputPipeline {
    pub layout: BindGroupLayoutDescriptor,
    pub pipeline_id: CachedRenderPipelineId,
    pub hdr_pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for ComposeOutputPipeline {
    fn from_world(world: &mut World) -> Self {
        let _render_device = world.resource::<RenderDevice>();

        let layout = BindGroupLayoutDescriptor::new(
            "outline_compose_output_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_depth_2d(),
                    texture_depth_2d(),
                ),
            ),
        );

        let target = Some(ColorTargetState {
            format: TextureFormat::Rgba8UnormSrgb,
            blend: None,
            write_mask: ColorWrites::ALL,
        });
        let hdr_target = Some(ColorTargetState {
            format: TextureFormat::Rgba16Float,
            blend: None,
            write_mask: ColorWrites::ALL,
        });

        let descriptor = RenderPipelineDescriptor {
            label: Some("outline_compose_output_pipeline".into()),
            layout: vec![layout.clone()],
            // This will setup a fullscreen triangle for the vertex state
            vertex: world
                .resource::<FullscreenShader>()
                .clone()
                .to_vertex_state(),
            fragment: Some(FragmentState {
                shader: COMPOSE_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: Some("fragment".into()),
                targets: vec![target],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            immediate_size: 0,
            zero_initialize_workgroup_memory: false,
        };
        let mut hdr_descriptor = descriptor.clone();
        hdr_descriptor.fragment.as_mut().unwrap().targets = vec![hdr_target];

        let (pipeline_id, hdr_pipeline_id) = {
            let cache = world.resource_mut::<PipelineCache>();
            (
                cache.queue_render_pipeline(descriptor),
                cache.queue_render_pipeline(hdr_descriptor),
            )
        };
        Self {
            layout,
            pipeline_id,
            hdr_pipeline_id,
        }
    }
}
