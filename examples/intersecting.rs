use std::f32::consts::PI;

use bevy::{
    color::palettes::css::{BLUE, GREEN, RED, SILVER, YELLOW},
    core_pipeline::prepass::DepthPrepass,
    input::keyboard::KeyboardInput,
    prelude::*,
};
use bevy_mesh_outline::{MeshOutline, MeshOutlinePlugin, OutlineCamera};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            MeshOutlinePlugin,
        ))
        .init_resource::<PriorityToggle>()
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            FixedUpdate,
            (
                toggle_priority.run_if(on_event::<KeyboardInput>),
                update_outline_priorities,
                update_priority_display,
            ),
        )
        .run();
}

#[derive(Component)]
pub struct OutlinePriority(f32);

#[derive(Resource, Default)]
struct PriorityToggle {
    enabled: bool,
}

#[derive(Component)]
struct PriorityText;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.5, 1., 1.5).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
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
            .with_rotation(Quat::from_rotation_x(PI / 5.0) * Quat::from_rotation_y(PI / 3.0)),
        MeshOutline::new(10.0).with_color(Color::from(RED)),
        OutlinePriority(1.0),
    ));

    // Blue sphere with green outline, high priority
    commands.spawn((
        Mesh3d(meshes.add(Sphere::default())),
        MeshMaterial3d(materials.add(Color::from(BLUE))),
        Transform::from_xyz(-0.5, 1.0, 0.5),
        MeshOutline::new(10.0)
            .with_color(Color::from(GREEN))
            .with_highlight(10.0),
        OutlinePriority(10.0),
    ));
}

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Disable priority (Q)"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        PriorityText,
    ));
}

fn toggle_priority(
    input: Res<ButtonInput<KeyCode>>,
    mut priority_toggle: ResMut<PriorityToggle>,
) {
    if input.just_pressed(KeyCode::KeyQ) {
        priority_toggle.enabled = !priority_toggle.enabled;
    }
}

fn update_outline_priorities(
    priority_toggle: Res<PriorityToggle>,
    mut outline_query: Query<(&mut MeshOutline, &OutlinePriority)>,
) {
    for (mut outline, priority) in &mut outline_query {
        if priority_toggle.enabled {
            outline.priority = priority.0;
        } else {
            outline.priority = 0.0;
        }
    }
}

fn update_priority_display(
    priority_toggle: Res<PriorityToggle>,
    mut text_query: Single<&mut Text, With<PriorityText>>,
) {
    if priority_toggle.enabled {
        text_query.0 = "Disable priority (Q)".to_string();
    } else {
        text_query.0 = "Use priority (Q)".to_string();
    }
}
