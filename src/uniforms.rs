#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bevy_render::render_resource::AsBindGroup;
use bytemuck::{Pod, Zeroable};

use super::ExtractedOutline;

#[derive(Debug, Clone, AsBindGroup, ShaderType, Pod, Zeroable, Copy)]
#[repr(C)]
pub struct OutlineUniform {
    pub intensity: f32,
    pub width: f32,
    pub id: f32,
    pub priority: f32,
    pub outline_color: Vec4,
}

impl From<&ExtractedOutline> for OutlineUniform {
    fn from(outline: &ExtractedOutline) -> Self {
        OutlineUniform {
            intensity: outline.intensity,
            width: outline.width,
            id: outline.id,
            priority: outline.priority,
            outline_color: outline.color,
        }
    }
}
