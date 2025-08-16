use bevy::{
    core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT,
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    pbr::{
        MeshInputUniform, MeshPipeline, MeshPipelineKey, MeshUniform, RenderMeshInstances,
        SkinUniforms,
    },
    prelude::*,
};
use bevy_render::{
    batching::{
        GetBatchData, GetFullBatchData,
        gpu_preprocessing::{IndirectParametersCpuMetadata, UntypedPhaseIndirectParametersBuffers},
    },
    mesh::{MeshVertexBufferLayoutRef, RenderMesh, allocator::MeshAllocator},
    render_asset::RenderAssets,
    render_resource::{
        BindGroupLayout, BindGroupLayoutEntries, ColorTargetState, ColorWrites, CompareFunction,
        DepthStencilState, Face, FragmentState, RenderPipelineDescriptor, ShaderStages,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, TextureFormat,
        binding_types::uniform_buffer,
    },
    renderer::RenderDevice,
    sync_world::MainEntity,
};
use nonmax::NonMaxU32;

use crate::shaders::MASK_SHADER_HANDLE;

use super::{ExtractedOutline, ExtractedOutlines, uniforms::OutlineUniform};

#[derive(Resource)]
pub struct MeshMaskPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub outline_bind_group_layout: BindGroupLayout,
}

impl FromWorld for MeshMaskPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let outline_instance_bind_group_layout = render_device.create_bind_group_layout(
            "OutlineInstance",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (uniform_buffer::<OutlineUniform>(false),),
            ),
        );

        let mesh_pipeline = MeshPipeline::from_world(world);

        Self {
            mesh_pipeline,
            // instance_batch_size,
            outline_bind_group_layout: outline_instance_bind_group_layout,
        }
    }
}

impl SpecializedMeshPipeline for MeshMaskPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = MASK_SHADER_HANDLE;
        descriptor.fragment.as_mut().unwrap().shader = MASK_SHADER_HANDLE;

        descriptor.fragment = Some(FragmentState {
            shader: MASK_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "fragment".into(),
            targets: vec![
                // RT0: flood data (uv.xy, width, depth)
                Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
                // RT1: appearance data (color.rgb, priority)
                Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }),
            ],
        });

        descriptor.depth_stencil = Some(DepthStencilState {
            format: CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: default(),
            bias: default(),
        });
        descriptor.label = Some("outline_pipeline".into());
        descriptor
            .layout
            .push(self.outline_bind_group_layout.clone());
        descriptor.primitive.cull_mode = Some(Face::Back);

        Ok(descriptor)
    }
}

impl GetBatchData for MeshMaskPipeline {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
        SRes<SkinUniforms>,
        SRes<ExtractedOutlines>,
    );
    type CompareData = (AssetId<Mesh>, ExtractedOutline);

    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, _, mesh_allocator, skin_uniforms, outlines): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        tracing::info!("get_batch_data for outline pipeline");
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };

        let current_skin_index = skin_uniforms.skin_index(main_entity);
        let material_bind_group_index = mesh_instance.material_bindings_index;
        let outline = outlines.0.get(&main_entity)?;

        Some((
            MeshUniform::new(
                &mesh_instance.transforms,
                first_vertex_index,
                material_bind_group_index.slot,
                None,
                current_skin_index,
                Some(mesh_instance.tag),
            ),
            Some((mesh_instance.mesh_asset_id, outline.clone())),
        ))
    }
}

impl GetFullBatchData for MeshMaskPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (mesh_instances, _, _, _, outlines): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };

        let mesh_instance = mesh_instances.get(&main_entity)?;
        let outline = outlines.0.get(&main_entity)?;

        Some((
            mesh_instance.current_uniform_index,
            Some((mesh_instance.mesh_asset_id, outline.clone())),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, _, mesh_allocator, skin_uniforms, _outlines): &SystemParamItem<
            Self::Param,
        >,
        main_entity: MainEntity,
    ) -> Option<Self::BufferData> {
        tracing::info!("get_binned_batch_data for outline pipeline");
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };

        let current_skin_index = skin_uniforms.skin_index(main_entity);

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            first_vertex_index,
            mesh_instance.material_bindings_index.slot,
            None,
            current_skin_index,
            Some(mesh_instance.tag),
        ))
    }

    fn get_binned_index(
        (mesh_instances, _, _, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<NonMaxU32> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };

        mesh_instances
            .get(&main_entity)
            .map(|entity| entity.current_uniform_index)
    }

    fn write_batch_indirect_parameters_metadata(
        indexed: bool,
        base_output_index: u32,
        batch_set_index: Option<NonMaxU32>,
        phase_indirect_parameters_buffers: &mut UntypedPhaseIndirectParametersBuffers,
        indirect_parameters_offset: u32,
    ) {
        let indirect_parameters = IndirectParametersCpuMetadata {
            base_output_index,
            batch_set_index: match batch_set_index {
                Some(batch_set_index) => u32::from(batch_set_index),
                None => !0,
            },
        };

        if indexed {
            phase_indirect_parameters_buffers
                .indexed
                .set(indirect_parameters_offset, indirect_parameters);
        } else {
            phase_indirect_parameters_buffers
                .non_indexed
                .set(indirect_parameters_offset, indirect_parameters);
        }
    }
}
