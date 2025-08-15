use bevy::{core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT, prelude::*};
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{Texture, TextureDescriptor},
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use super::OutlineCamera;

#[derive(Clone, Component)]
pub struct FloodTextures {
    pub flip: bool,
    pub input: CachedTexture,
    pub output: CachedTexture,
    pub outline_depth_texture: Texture,
    pub outline_color_storage: CachedTexture,
    pub appearance_texture: CachedTexture,
}

impl FloodTextures {
    pub fn input(&self) -> &CachedTexture {
        if self.flip { &self.output } else { &self.input }
    }

    pub fn output(&self) -> &CachedTexture {
        if self.flip { &self.input } else { &self.output }
    }

    pub fn flip(&mut self) {
        self.flip = !self.flip;
    }
}

pub fn prepare_flood_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    cameras: Query<(Entity, &ExtractedCamera), With<OutlineCamera>>,
) {
    for (entity, camera) in cameras.iter() {
        let Some(target_size) = camera.physical_target_size else {
            continue;
        };

        let size = Extent3d {
            width: target_size.x,
            height: target_size.y,
            depth_or_array_layers: 1,
        };

        let texture_descriptor = TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        // Create the depth texture
        let depth_texture = render_device.create_texture(&TextureDescriptor {
            label: Some("outline depth texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: CORE_3D_DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT  // For using as depth buffer
        | TextureUsages::TEXTURE_BINDING, // For sampling in composite pass
            view_formats: &[],
        });

        let color_storage_texture_descriptor = TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        commands.entity(entity).insert(FloodTextures {
            flip: false,
            input: texture_cache.get(&render_device, texture_descriptor.clone()),
            output: texture_cache.get(&render_device, texture_descriptor.clone()),
            outline_depth_texture: depth_texture,
            outline_color_storage: texture_cache
                .get(&render_device, color_storage_texture_descriptor),
            appearance_texture: texture_cache
                .get(&render_device, texture_descriptor),
        });
        texture_cache.update();
    }
}
