use bevy::{
    color::palettes::css::{BLUE, RED, SILVER},
    core_pipeline::{bloom::Bloom, prepass::DepthPrepass},
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
        .add_systems(FixedUpdate, (rotate, oscillate_intensity))
        .run();
}

#[derive(Component)]
struct OutlineGlow {
    intensity: f32,
    period: f32,
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
        // Turn on bloom
        Bloom::default(),
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

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        // Add outline
        MeshOutline::new(10.0).with_color(Color::from(RED)),
        OutlineGlow {
            intensity: 20.0,
            period: 0.2,
        },
    ));
}

fn rotate(mut query: Query<&mut Transform, With<MeshOutline>>, time: Res<Time>) {
    for mut transform in &mut query {
        let rotation = Quat::from_rotation_y(time.delta_secs() / 2.)
            * Quat::from_rotation_x(time.delta_secs());

        transform.rotation *= rotation;
    }
}

fn oscillate_intensity(mut query: Query<(&mut MeshOutline, &OutlineGlow)>, time: Res<Time>) {
    for (mut outline, glow) in &mut query {
        let t = (time.elapsed_secs() / glow.period).sin() * 0.5 + 0.5; // Normalize to [0, 1]

        outline.intensity = glow.intensity * t;
    }
}
