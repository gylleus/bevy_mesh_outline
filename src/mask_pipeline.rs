use bevy::{
    core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT,
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    mesh::MeshVertexBufferLayoutRef,
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
    mesh::{RenderMesh, allocator::MeshAllocator},
    render_asset::RenderAssets,
    render_resource::{
        BindGroupLayoutDescriptor, BindGroupLayoutEntries, ColorTargetState, ColorWrites,
        CompareFunction, DepthStencilState, Face, FragmentState, MultisampleState,
        RenderPipelineDescriptor, ShaderStages, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, TextureFormat, binding_types::uniform_buffer,
    },
    sync_world::MainEntity,
};
use nonmax::NonMaxU32;

use crate::shaders::MASK_SHADER_HANDLE;

use super::{ExtractedOutline, ExtractedOutlines, uniforms::OutlineUniform};

#[derive(Resource)]
pub struct MeshMaskPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub outline_bind_group_layout: BindGroupLayoutDescriptor,
}

/// Initializes the [`MeshMaskPipeline`] resource.
///
/// [`MeshMaskPipeline`] wraps the base [`MeshPipeline`], so this runs in
/// `RenderStartup` after `MeshPipelineSystems` and reads the finished
/// [`MeshPipeline`] resource.
pub fn init_mesh_mask_pipeline(mut commands: Commands, mesh_pipeline: Res<MeshPipeline>) {
    let outline_instance_bind_group_layout = BindGroupLayoutDescriptor::new(
        "OutlineInstance",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (uniform_buffer::<OutlineUniform>(false),),
        ),
    );

    commands.insert_resource(MeshMaskPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        outline_bind_group_layout: outline_instance_bind_group_layout,
    });
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
            entry_point: Some("fragment".into()),
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
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::GreaterEqual),
            stencil: default(),
            bias: default(),
        });

        // The mask pass always renders to single-sampled targets (the jump-flood
        // seed data cannot be meaningfully multisample-resolved). We keep the
        // camera's MSAA in `key` so the view bind group layout matches, but force
        // the pipeline itself to a single sample. Reading the (multisampled)
        // prepass depth texture from the mesh view bind group is unaffected by
        // the pipeline's own sample count.
        descriptor.multisample = MultisampleState::default();

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
    // Actual draw-call batching is driven by the binned phase's bin key (mesh
    // asset id + outline appearance, see `queue_outline`). These compare-data
    // types are only consulted on the sorted / multi-draw paths, which this
    // binned phase never takes; they just need to stay internally consistent.
    type BatchSetCompareData = ExtractedOutline;
    type BatchCompareData = AssetId<Mesh>;

    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, _, mesh_allocator, skin_uniforms, outlines): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(
        Self::BufferData,
        Option<(Self::BatchSetCompareData, Self::BatchCompareData)>,
    )> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id()) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };

        let current_skin_index = skin_uniforms.skin_index(main_entity);
        let material_bind_group_index = mesh_instance.material_bindings_index();
        let outline = outlines.0.get(&main_entity)?;

        Some((
            MeshUniform::new(
                &mesh_instance.transforms,
                first_vertex_index,
                material_bind_group_index.slot,
                None,
                current_skin_index,
                None,
                Some(mesh_instance.tag()),
            ),
            Some((outline.clone(), mesh_instance.mesh_asset_id())),
        ))
    }
}

impl GetFullBatchData for MeshMaskPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (mesh_instances, _, _, _, outlines): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<(
        NonMaxU32,
        Option<(Self::BatchSetCompareData, Self::BatchCompareData)>,
    )> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_index_and_compare_data` should never be called in CPU mesh uniform \
                building mode"
            );
            return None;
        };

        let mesh_instance = mesh_instances.get(&main_entity)?;
        let outline = outlines.0.get(&main_entity)?;

        Some((
            NonMaxU32::new(mesh_instance.gpu_specific.current_uniform_index())?,
            Some((outline.clone(), mesh_instance.mesh_asset_id())),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, _, mesh_allocator, skin_uniforms, _outlines): &SystemParamItem<
            Self::Param,
        >,
        main_entity: MainEntity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id()) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };

        let current_skin_index = skin_uniforms.skin_index(main_entity);

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            first_vertex_index,
            mesh_instance.material_bindings_index().slot,
            None,
            current_skin_index,
            None,
            Some(mesh_instance.tag()),
        ))
    }

    fn get_binned_index(
        (mesh_instances, _, _, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<NonMaxU32> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            tracing::error!(
                "`get_binned_index` should never be called in CPU mesh uniform \
                building mode"
            );
            return None;
        };

        mesh_instances
            .get(&main_entity)
            .and_then(|entity| NonMaxU32::new(entity.gpu_specific.current_uniform_index()))
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
