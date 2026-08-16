use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_render::{
    render_phase::{RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{BindGroup, BindGroupEntry, BufferInitDescriptor, PipelineCache},
    renderer::RenderDevice,
};
use wgpu_types::BufferUsages;

use super::{
    ExtractedOutlines,
    mask::{MeshOutline3d, OutlineKey},
    mask_pipeline::MeshMaskPipeline,
    uniforms::OutlineUniform,
};

pub(crate) struct SetOutlineBindGroup<const I: usize>();

impl<const I: usize> RenderCommand<MeshOutline3d> for SetOutlineBindGroup<I> {
    type Param = SRes<OutlineBindGroups>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        item: &MeshOutline3d,
        _view: (),
        _entity_data: Option<()>,
        outline_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let outline_bind_groups = outline_bind_groups.into_inner();

        // Every instance in this batch shares the same appearance (it's part of
        // the batch-set key), so a single bind group keyed by that appearance
        // serves the whole batch.
        if let Some(bind_group) = outline_bind_groups.0.get(&item.batch_set_key.outline) {
            pass.set_bind_group(I, bind_group, &[]);
            RenderCommandResult::Success
        } else {
            // Bind group not ready yet, skip this frame
            RenderCommandResult::Skip
        }
    }
}

/// One outline uniform bind group per distinct appearance, cached across frames.
///
/// Because meshes only batch when they share an appearance (see
/// [`crate::mask::OutlineBatchSetKey`]), keying the bind groups by appearance
/// rather than by entity means we allocate O(distinct appearances) GPU resources
/// per frame instead of one buffer + bind group per outlined entity every frame
/// — and in the common case of a stable set of appearances, zero per frame.
#[derive(Resource, Default)]
pub struct OutlineBindGroups(HashMap<OutlineKey, BindGroup>);

pub fn prepare_outline_bind_groups(
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    outline_pipeline: Res<MeshMaskPipeline>,
    extracted_outlines: Res<ExtractedOutlines>,
    mut outline_bind_groups: ResMut<OutlineBindGroups>,
    mut live_keys: Local<HashSet<OutlineKey>>,
) {
    live_keys.clear();

    for outline in extracted_outlines.0.values() {
        let key = OutlineKey::from_outline(outline);
        if !live_keys.insert(key) {
            // Already built (or reused) a bind group for this appearance.
            continue;
        }

        // Only touches the GPU for appearances we haven't cached yet.
        outline_bind_groups.0.entry(key).or_insert_with(|| {
            let outline_uniform = OutlineUniform::from(outline);

            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("outline_uniform_buffer"),
                contents: bytemuck::cast_slice(&[outline_uniform]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            render_device.create_bind_group(
                Some("outline_bind_group"),
                &pipeline_cache.get_bind_group_layout(&outline_pipeline.outline_bind_group_layout),
                &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            )
        });
    }

    // Drop bind groups for appearances no longer in use so the cache stays
    // bounded by the appearances actually on screen.
    outline_bind_groups
        .0
        .retain(|key, _| live_keys.contains(key));
}

#[cfg(test)]
mod tests {
    use bevy::{
        camera::RenderTarget,
        core_pipeline::prepass::DepthPrepass,
        prelude::*,
        render::{
            RenderPlugin, pipelined_rendering::PipelinedRenderingPlugin,
            render_resource::TextureFormat,
        },
        window::{ExitCondition, WindowPlugin},
        winit::WinitPlugin,
    };
    use bevy_render::RenderApp;

    use crate::{ExtractedOutline, MeshOutline, MeshOutlinePlugin, OutlineCamera};

    use super::{OutlineBindGroups, OutlineKey};

    /// An outline whose appearance changes every frame must still have a bind
    /// group for that frame's appearance, or `SetOutlineBindGroup` skips it.
    #[test]
    #[ignore = "requires a GPU adapter; run with: cargo test -- --ignored"]
    fn animated_outline_keeps_its_bind_group() {
        let mut app = App::new();

        app.add_plugins(
            DefaultPlugins
                .build()
                // Its event loop panics off the main thread; frames are driven
                // manually below.
                .disable::<WinitPlugin>()
                // Keeps the render app in this `App`, so it can be inspected.
                .disable::<PipelinedRenderingPlugin>()
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        )
        .add_plugins(MeshOutlinePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_outline);

        app.finish();
        app.cleanup();

        for frame in 0..8 {
            app.update();

            // Let the renderer settle first.
            if frame < 2 {
                continue;
            }

            let render_world = app.sub_app_mut(RenderApp).world_mut();
            let outlines: Vec<ExtractedOutline> = render_world
                .query::<&ExtractedOutline>()
                .iter(render_world)
                .cloned()
                .collect();
            let bind_groups = render_world.resource::<OutlineBindGroups>();

            let mut checked = 0;
            for outline in &outlines {
                assert!(
                    bind_groups
                        .0
                        .contains_key(&OutlineKey::from_outline(outline)),
                    "frame {frame}: no bind group for the outline being drawn \
                     (width {}), so its draw is skipped",
                    outline.width,
                );
                checked += 1;
            }
            assert_eq!(checked, 1, "frame {frame}: expected one extracted outline");
        }
    }

    fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut images: ResMut<Assets<Image>>,
    ) {
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
        ));

        commands.spawn((PointLight::default(), Transform::from_xyz(8.0, 16.0, 8.0)));

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_xyz(0.0, 1.0, 0.0),
            MeshOutline::new(10.0),
        ));
    }

    fn animate_outline(mut outlines: Query<&mut MeshOutline>) {
        for mut outline in outlines.iter_mut() {
            outline.width -= 0.25;
        }
    }
}
