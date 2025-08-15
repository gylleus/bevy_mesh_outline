use bevy::{
    color::palettes::css::{SILVER, YELLOW},
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
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
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            FixedUpdate,
            (
                rotate,
                update_size.run_if(on_event::<KeyboardInput>),
                update_width_display,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 2., 3.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        OutlineCamera,
        NormalPrepass,
        DepthPrepass,
        Msaa::Off,
        Camera {
            hdr: true,
            order: 0,
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

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(YELLOW))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        MeshOutline::new(5.0),
    ));
}

fn rotate(mut query: Query<&mut Transform, With<MeshOutline>>, time: Res<Time>) {
    for mut transform in &mut query {
        let rotation = Quat::from_rotation_y(time.delta_secs() / 2.)
            * Quat::from_rotation_x(time.delta_secs());

        transform.rotation *= rotation;
    }
}

fn update_size(input: Res<ButtonInput<KeyCode>>, mut outline: Single<&mut MeshOutline>) {
    let mut delta = 0.0;
    let change_speed = 0.1;

    if input.pressed(KeyCode::KeyQ) {
        delta -= change_speed;
    } else if input.pressed(KeyCode::KeyW) {
        delta += change_speed;
    }

    outline.width += delta;
}

#[derive(Component)]
struct WidthText;

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Decrease width (Q)\nIncrease width (W)\nCurrent width: 5.0"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        WidthText,
    ));
}

fn update_width_display(
    outline_query: Single<&MeshOutline>,
    mut text_query: Single<&mut Text, With<WidthText>>,
) {
    let width = outline_query.width;
    text_query.0 = format!(
        "Decrease width (Q)\nIncrease width (W)\nCurrent width: {:.1}",
        width
    );
}
