use std::ops::Range;

use bevy::{asset::UntypedAssetId, prelude::*};
use bevy_render::{
    mesh::allocator::SlabId,
    render_phase::{
        BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem,
        PhaseItemBatchSetKey, PhaseItemExtraIndex,
    },
    render_resource::CachedRenderPipelineId,
    sync_world::MainEntity,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OutlineBatchSetKey {
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub vertex_slab: SlabId,
    pub index_slab: Option<SlabId>,
}

impl PhaseItemBatchSetKey for OutlineBatchSetKey {
    fn indexed(&self) -> bool {
        self.index_slab.is_some()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OutlineBinKey {
    pub asset_id: UntypedAssetId,
}

pub(crate) struct MeshOutline3d {
    pub batch_set_key: OutlineBatchSetKey,
    pub entity: Entity,
    pub main_entity: MainEntity,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for MeshOutline3d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    fn main_entity(&self) -> bevy::render::sync_world::MainEntity {
        self.main_entity
    }

    fn draw_function(&self) -> bevy::render::render_phase::DrawFunctionId {
        self.batch_set_key.draw_function
    }

    fn batch_range(&self) -> &std::ops::Range<u32> {
        &self.batch_range
    }

    fn batch_range_mut(&mut self) -> &mut std::ops::Range<u32> {
        &mut self.batch_range
    }

    fn extra_index(&self) -> bevy::render::render_phase::PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    fn batch_range_and_extra_index_mut(
        &mut self,
    ) -> (
        &mut Range<u32>,
        &mut bevy::render::render_phase::PhaseItemExtraIndex,
    ) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for MeshOutline3d {
    type BinKey = OutlineBinKey;
    type BatchSetKey = OutlineBatchSetKey;

    fn new(
        batch_set_key: Self::BatchSetKey,
        _key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            entity: representative_entity.0,
            main_entity: representative_entity.1,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for MeshOutline3d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}
