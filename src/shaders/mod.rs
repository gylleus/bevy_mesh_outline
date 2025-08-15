use bevy::prelude::*;

use bevy::asset::{Handle, load_internal_asset, weak_handle};
use bevy_render::render_resource::Shader;

pub(crate) const MASK_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("4c41a7eb-b802-4e76-97f1-3327d80743dd");

pub(crate) const FLOOD_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("a06a9919-18e3-4e91-a312-a1463bb6d719");

pub(crate) const COMPOSE_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("6fe0f3ef-e31f-40e7-a20a-ed002ac4bb3f");

pub(crate) fn load_shaders(app: &mut App) {
    load_internal_asset!(app, MASK_SHADER_HANDLE, "mask.wgsl", Shader::from_wgsl);
    load_internal_asset!(app, FLOOD_SHADER_HANDLE, "flood.wgsl", Shader::from_wgsl);
    load_internal_asset!(
        app,
        COMPOSE_SHADER_HANDLE,
        "compose_output.wgsl",
        Shader::from_wgsl
    );
}
