//! Throwaway FPS benchmark: spawn many identical outlined cubes (same mesh +
//! same appearance -> the case batching collapses), render uncapped, and print
//! the average frame time over a fixed measurement window, then exit.
//!
//! Count is configurable: `BENCH_COUNT=20000 cargo run --release --example bench`.

use std::time::Instant;

use bevy::{
    color::palettes::css::RED,
    core_pipeline::prepass::DepthPrepass,
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use bevy_mesh_outline::{MeshOutline, MeshOutlinePlugin, OutlineCamera};

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let count: usize = std::env::var("BENCH_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    let warmup = env_u32("BENCH_WARMUP", 60);
    let measure_frames = env_u32("BENCH_MEASURE", 240);

    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(MeshOutlinePlugin)
        .insert_resource(BenchConfig {
            count,
            warmup,
            measure: measure_frames,
        })
        .init_resource::<Bench>()
        .add_systems(Startup, setup)
        .add_systems(Update, measure)
        .run();
}

#[derive(Resource)]
struct BenchConfig {
    count: usize,
    warmup: u32,
    measure: u32,
}

#[derive(Resource, Default)]
struct Bench {
    frame: u32,
    start: Option<Instant>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<BenchConfig>,
) {
    // When set, spawn the same scene with NO outline effect at all, to measure
    // the baseline per-frame cost of the plain scene for comparison.
    let no_outline = std::env::var("BENCH_NO_OUTLINE").is_ok();

    let side = (cfg.count as f32).sqrt().ceil() as i32;
    let spacing = 0.7_f32;
    let extent = side as f32 * spacing;

    let mut camera = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, extent * 0.6, extent * 1.1).looking_at(Vec3::ZERO, Vec3::Y),
        DepthPrepass,
        Msaa::Off,
    ));
    if !no_outline {
        camera.insert(OutlineCamera);
    }
    commands.spawn((
        PointLight {
            intensity: 10_000_000.0,
            range: 10_000.0,
            ..default()
        },
        Transform::from_xyz(extent, extent, extent),
    ));

    // One shared mesh + material so all instances are the same render mesh.
    let cube = meshes.add(Cuboid::default());
    let mat = materials.add(Color::srgb(0.5, 0.5, 0.5));

    let mut n = 0usize;
    'outer: for x in 0..side {
        for z in 0..side {
            if n >= cfg.count {
                break 'outer;
            }
            let mut cube_entity = commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(
                    (x as f32 - side as f32 / 2.0) * spacing,
                    0.0,
                    (z as f32 - side as f32 / 2.0) * spacing,
                )
                .with_scale(Vec3::splat(0.4)),
            ));
            if !no_outline {
                // Identical appearance for every cube -> a single batch set.
                cube_entity.insert(MeshOutline::new(2.0).with_color(Color::from(RED)));
            }
            n += 1;
        }
    }
    info!("spawned {n} outlined cubes (all in frustum)");
}

fn measure(mut bench: ResMut<Bench>, mut exit: MessageWriter<AppExit>, cfg: Res<BenchConfig>) {
    bench.frame += 1;
    if bench.frame == cfg.warmup {
        bench.start = Some(Instant::now());
    }
    if bench.frame == cfg.warmup + cfg.measure {
        let elapsed = bench.start.unwrap().elapsed();
        let secs = elapsed.as_secs_f64();
        let ms = secs * 1000.0 / cfg.measure as f64;
        let fps = cfg.measure as f64 / secs;
        println!(
            "BENCH_RESULT count={} frames={} elapsed={:.3}s avg_frame={:.3}ms fps={:.1}",
            cfg.count, cfg.measure, secs, ms, fps
        );
        exit.write(AppExit::Success);
    }
}
