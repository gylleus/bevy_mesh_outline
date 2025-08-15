//! Plays an animation on a skinned glTF model of a fox.
//! Modified version of Bevy animated_mesh example.
//! https://github.com/bevyengine/bevy/blob/v0.16.0/examples/animation/animated_mesh.rs

use std::f32::consts::PI;

use bevy::{
    core_pipeline::prepass::DepthPrepass, pbr::CascadeShadowConfigBuilder, prelude::*,
    scene::SceneInstanceReady,
};
use bevy_mesh_outline::{MeshOutline, MeshOutlinePlugin, OutlineCamera};

const GLTF_PATH: &str = "Fox.glb";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshOutlinePlugin)
        .add_systems(Startup, setup_fox)
        .add_systems(Startup, setup_camera_and_environment)
        .run();
}

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

fn setup_fox(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(GLTF_PATH)),
    );

    let graph_handle = graphs.add(graph);

    let animation_to_play = AnimationToPlay {
        graph_handle,
        index,
    };

    let mesh_scene = SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH)));

    commands
        .spawn((animation_to_play, mesh_scene))
        // Triggered after scene finishes loading
        .observe(initialize_outlines)
        .observe(initialize_animations);
}

// Called when the scene is ready and meshes exist
fn initialize_outlines(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mesh_entities: Query<Entity, With<Mesh3d>>,
) {
    for child in children.iter_descendants(trigger.target()) {
        // If the current entity in scene is a mesh - add outline
        if let Ok(mesh_entity) = mesh_entities.get(child) {
            commands.entity(mesh_entity).insert(MeshOutline::new(10.0));
        }
    }
    commands.entity(trigger.observer()).despawn();
}

fn initialize_animations(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if let Ok(animation_to_play) = animations_to_play.get(trigger.target()) {
        for child in children.iter_descendants(trigger.target()) {
            // Setup animations
            if let Ok(mut player) = players.get_mut(child) {
                player.play(animation_to_play.index).repeat();

                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
        }
    }
    commands.entity(trigger.observer()).despawn();
}

// Spawn a camera and a simple environment with a ground plane and light.
fn setup_camera_and_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(100.0, 100.0, 150.0).looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
        OutlineCamera,
        DepthPrepass,
        Msaa::Off,
        Camera {
            hdr: true,
            ..default()
        },
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(500000.0, 500000.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Light
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 200.0,
            maximum_distance: 400.0,
            ..default()
        }
        .build(),
    ));
}
