use std::ops::Range;

use bevy::{asset::UntypedAssetId, prelude::*};
use bevy_render::{
    mesh::allocator::MeshSlabs,
    render_phase::{
        BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem,
        PhaseItemBatchSetKey, PhaseItemExtraIndex,
    },
    render_resource::CachedRenderPipelineId,
    sync_world::MainEntity,
};

use crate::ExtractedOutline;

/// Hashable/orderable representation of an outline's appearance (everything in
/// [`crate::uniforms::OutlineUniform`] except the per-instance transform).
///
/// This is part of the [`OutlineBatchSetKey`], mirroring how Bevy's own opaque
/// phase keys its batch sets on the material bind group. A batch set draws with
/// a single outline bind group (that of its representative entity), so every
/// instance in it must share the same appearance — floats are stored as their
/// bit patterns so the key can derive `Eq`/`Ord`/`Hash`, and identical `f32`
/// values always share bit patterns, so this never merges visually different
/// outlines.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OutlineKey {
    pub intensity: u32,
    pub width: u32,
    pub priority: u32,
    pub color: [u32; 4],
}

impl OutlineKey {
    pub fn from_outline(outline: &ExtractedOutline) -> Self {
        Self {
            intensity: outline.intensity.to_bits(),
            width: outline.width.to_bits(),
            priority: outline.priority.to_bits(),
            color: outline.color.to_array().map(f32::to_bits),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OutlineBatchSetKey {
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub slabs: MeshSlabs,
    /// Outline appearance. Kept in the batch-set key (not the bin key) so that a
    /// multi-drawn batch set never spans instances that would need different
    /// outline bind groups.
    pub outline: OutlineKey,
}

impl PhaseItemBatchSetKey for OutlineBatchSetKey {
    fn indexed(&self) -> bool {
        self.slabs.index_slab_id.is_some()
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
