use std::f32::consts::PI;

use bevy::{
    color::palettes::css::{BLUE, GREEN, RED, SILVER, YELLOW},
    core_pipeline::prepass::DepthPrepass,
    prelude::*,
};
use bevy_mesh_outline::{MeshOutline, MeshOutlinePlugin, OutlineCamera};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            MeshOutlinePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, (rotate,))
        .run();
}

#[derive(Component)]
pub struct Rotation {
    velocity: Vec3,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 2., 3.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        // Mark camera for outline rendering
        OutlineCamera,
        DepthPrepass,
        Msaa::Off,
        Camera {
            hdr: true,
            ..default()
        },
    ));

    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
    ));

    // Yellow cube with red outline, low priority
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(YELLOW))),
        Transform::from_xyz(0.0, 1.0, 0.0)
            .with_rotation(Quat::from_rotation_x(PI / 4.0) * Quat::from_rotation_y(PI / 3.0)),
        MeshOutline::new(10.0)
            .with_color(Color::from(RED))
            .with_priority(1.0),
    ));

    // Blue sphere with green outline, high priority
    commands.spawn((
        Mesh3d(meshes.add(Sphere::default())),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        Transform::from_xyz(-0.5, 1.0, 0.5),
        MeshOutline::new(10.0)
            .with_color(Color::from(GREEN))
            .with_priority(5.0)
            .with_highlight(10.0),
    ));

    // Another cube with blue outline, medium priority, overlapping the sphere
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::default())),
    //     MeshMaterial3d(materials.add(Color::from(GREEN))),
    //     Transform::from_xyz(-0.3, 1.2, 0.3).with_scale(Vec3::splat(0.8)),
    //     MeshOutline::new(15.0)
    //         .with_color(Color::from(BLUE))
    //         .with_priority(3.0),
    // ));
}

fn rotate(mut query: Query<(&mut Transform, &Rotation)>, time: Res<Time>) {
    for (mut transform, rotation) in &mut query {
        let rotation = Quat::from_rotation_y(rotation.velocity.y * time.delta_secs())
            * Quat::from_rotation_x(rotation.velocity.x * time.delta_secs())
            * Quat::from_rotation_z(rotation.velocity.z * time.delta_secs());

        transform.rotation *= rotation;
    }
}
