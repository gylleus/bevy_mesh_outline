//! Smoke test for outline rendering with MSAA disabled.
//!
//! The compose pass picks a different pipeline and bind group layout depending on
//! whether the camera uses MSAA. This renders a frame headlessly to an off-screen
//! image with `Msaa::Off` and an `OutlineCamera`, exercising the full
//! mask -> jump-flood -> compose pipeline on the non-MSAA path so a regression
//! there surfaces as a wgpu validation error.
//!
//! It needs a real GPU adapter, so it is `#[ignore]`d by default (the CI runners
//! used by this repo have no GPU). Run it locally with:
//!
//! ```sh
//! cargo test --test msaa_disabled -- --ignored
//! ```

use bevy::{
    camera::RenderTarget,
    core_pipeline::prepass::DepthPrepass,
    prelude::*,
    render::{RenderPlugin, render_resource::TextureFormat},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use bevy_mesh_outline::{MeshOutline, MeshOutlinePlugin, OutlineCamera};

#[test]
#[ignore = "requires a GPU adapter; run with: cargo test -- --ignored"]
fn outline_renders_with_msaa_disabled() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .build()
            // Winit would create an event loop on this (non-main) test thread,
            // which panics on macOS. We drive frames manually with `update()`.
            .disable::<WinitPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                // Ensure pipelines are ready when the frames below run.
                synchronous_pipeline_compilation: true,
                ..default()
            }),
    )
    .add_plugins(MeshOutlinePlugin)
    .add_systems(Startup, setup);

    // `App::update` does not run plugin `finish`/`cleanup` (only `App::run`
    // does). Do it manually so the renderer is initialized and `RenderDevice`
    // and the outline pipelines exist before the frames below.
    app.finish();
    app.cleanup();

    // Run several frames so pipelines finish compiling and the outline pass
    // actually executes. A validation error on the non-MSAA path surfaces here.
    for _ in 0..8 {
        app.update();
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Render to an off-screen image instead of a window.
    let target = images.add(Image::new_target_texture(
        64,
        64,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));

    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(3.0, 2.0, 3.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        OutlineCamera,
        DepthPrepass,
        // The behaviour under test: outlines must work with MSAA disabled.
        Msaa::Off,
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(8.0, 16.0, 8.0)));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 1.0, 0.0),
        MeshOutline::new(10.0),
    ));
}
