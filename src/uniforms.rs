use bevy::render::render_resource::ShaderType;
use bevy::prelude::*;
use bevy_render::render_resource::AsBindGroup;
use bytemuck::{Pod, Zeroable};

use super::ExtractedOutline;

#[derive(Debug, Clone, AsBindGroup, ShaderType, Pod, Zeroable, Copy)]
#[repr(C)]
pub struct OutlineUniform {
    // pub color: Vec4,
    // pub color: LinearRgba,
    pub highlight: f32,
    pub width: f32,
    pub id: f32,
    pub instance_index: u32,
    pub world_from_local: [Vec4; 3],
}

impl From<&ExtractedOutline> for OutlineUniform {
    fn from(outline: &ExtractedOutline) -> Self {
        OutlineUniform {
            // color: outline.color.into(),
            highlight: outline.highlight,
            width: outline.width,
            id: outline.id,
            instance_index: 12,
            world_from_local: outline.world_from_local,
        }
    }
}
