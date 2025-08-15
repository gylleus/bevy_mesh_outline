use bevy::{
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        render_resource::{
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            DynamicUniformBuffer, FragmentState, Operations, PipelineCache,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
            RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderType,
            binding_types::{sampler, texture_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::CachedTexture,
    },
};
use bevy_render::render_resource::{TextureView, binding_types::texture_depth_2d};
use wgpu_types::{
    ColorTargetState, ColorWrites, FilterMode, MultisampleState, PrimitiveState,
    SamplerBindingType, ShaderStages, TextureFormat, TextureSampleType,
};

use crate::shaders::FLOOD_SHADER_HANDLE;

use super::{ExtractedOutline, OutlineCamera};

#[derive(ShaderType)]
pub struct JumpFloodUniform {
    #[align(16)]
    pub size: u32,
}

#[derive(Component, Default, Clone)]
pub struct FloodSettings {
    pub width: f32,
}

pub fn prepare_flood_settings(
    mut commands: Commands,
    outlines: Query<&ExtractedOutline>,
    cameras: Query<Entity, With<OutlineCamera>>,
) {
    let mut max_size: f32 = 0.0;
    for outline in outlines.iter() {
        max_size = max_size.max(outline.width);
    }
    let settings = FloodSettings { width: max_size };

    for entity in cameras.iter() {
        commands.entity(entity).insert(settings.clone());
    }
}

#[derive(Resource)]
pub struct JumpFloodPipeline {
    pub layout: BindGroupLayout,
    pub sampler: Sampler,
    pub pipeline_id: CachedRenderPipelineId,
    pub lookup_buffer: DynamicUniformBuffer<JumpFloodUniform>,
    pub lookup_offsets: Vec<u32>,
}

impl FromWorld for JumpFloodPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "outline_jump_flood_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }), // flood_texture
                    sampler(SamplerBindingType::Filtering),                    // texture_sampler
                    uniform_buffer::<JumpFloodUniform>(true),                  // instance
                    texture_depth_2d(),                                        // depth_texture
                    texture_2d(TextureSampleType::Float { filterable: true }), // color_texture
                    texture_2d(TextureSampleType::Float { filterable: true }), // appearance_texture
                ),
            ),
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("outline_jump_flood_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: FLOOD_SHADER_HANDLE,
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                });

        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let mut uniform_buffer = DynamicUniformBuffer::new_with_alignment(
            render_device.limits().min_uniform_buffer_offset_alignment as u64,
        );
        let mut offsets = Vec::new();
        for bit in 0..32 {
            offsets.push(uniform_buffer.push(&JumpFloodUniform { size: 1 << bit }));
        }
        uniform_buffer.write_buffer(render_device, render_queue);

        Self {
            layout,
            sampler,
            pipeline_id,
            lookup_buffer: uniform_buffer,
            lookup_offsets: offsets,
        }
    }
}

pub struct JumpFloodPass<'w> {
    pub pipeline: &'w JumpFloodPipeline,
    render_pipeline: &'w RenderPipeline,
}

impl<'w> JumpFloodPass<'w> {
    pub fn new(world: &'w World) -> Option<Self> {
        let pipeline = world.resource::<JumpFloodPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let render_pipeline = pipeline_cache.get_render_pipeline(pipeline.pipeline_id)?;

        Some(Self {
            pipeline,
            render_pipeline,
        })
    }

    pub fn execute(
        &mut self,
        render_context: &mut RenderContext<'_>,
        input: &CachedTexture,
        output: &CachedTexture,
        depth_texture: &TextureView,
        color_texture: &TextureView,
        appearance_texture: &TextureView,
        size: u32,
    ) {
        let bind_group = render_context.render_device().create_bind_group(
            "outline_jump_flood_bind_group",
            &self.pipeline.layout,
            &BindGroupEntries::sequential((
                &input.default_view,
                &self.pipeline.sampler,
                self.pipeline.lookup_buffer.binding().unwrap(),
                depth_texture,
                color_texture,
                appearance_texture,
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("outline_jump_flood_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &output.default_view,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(self.render_pipeline);
        render_pass.set_bind_group(
            0,
            &bind_group,
            &[self.pipeline.lookup_offsets[size as usize]],
        );
        render_pass.draw(0..3, 0..1);
    }
}
