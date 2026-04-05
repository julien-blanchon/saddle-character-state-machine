use std::f32::consts::PI;

use bevy::{
    animation::{AnimatedBy, AnimationTargetId, animated_field},
    prelude::*,
};
use saddle_character_state_machine::*;

pub const RIG_NAME: &str = "Rig";

pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

pub fn format_active_bindings(selection: &CharacterAnimationSelection) -> String {
    if selection.active_bindings.is_empty() {
        return "none".into();
    }

    selection
        .active_bindings
        .iter()
        .map(|binding| format!("{}:{:.2}", binding.binding.0, binding.weight))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn build_showcase_definition(
    id: impl Into<CharacterStateMachineDefinitionId>,
) -> CharacterStateMachineDefinition {
    CharacterStateMachineDefinition::new(id, "Idle")
        .with_fallback_state("Idle")
        .with_default_blend(BlendDefinition::new(0.18).with_easing(BlendEasing::SineInOut))
        .add_state(StateDefinition::new("Grounded").with_binding("idle"))
        .add_state(
            StateDefinition::new("Idle")
                .with_parent("Grounded")
                .with_binding("idle"),
        )
        .add_state(
            StateDefinition::new("Locomotion")
                .with_parent("Grounded")
                .with_blend_tree_1d(
                    BlendTree1D::new(BlendTreeParameter::Speed)
                        .with_point(0.25, "walk")
                        .with_point(1.0, "run"),
                )
                .with_expected_duration(0.65),
        )
        .add_state(
            StateDefinition::new("JumpStart")
                .transient()
                .with_binding("jump_start")
                .with_expected_duration(0.22),
        )
        .add_state(
            StateDefinition::new("Airborne")
                .with_binding("airborne")
                .with_expected_duration(0.55),
        )
        .add_state(
            StateDefinition::new("Land")
                .transient()
                .with_binding("land")
                .with_expected_duration(0.18),
        )
        .add_state(
            StateDefinition::new("Attack")
                .transient()
                .with_binding("attack")
                .with_expected_duration(0.35),
        )
        .add_state(
            StateDefinition::new("Reload")
                .transient()
                .with_binding("reload")
                .with_expected_duration(0.8)
                .non_interruptible(),
        )
        .add_state(
            StateDefinition::new("Emote")
                .transient()
                .with_binding("emote")
                .with_expected_duration(0.55),
        )
        .add_transition(
            TransitionDefinition::switch("idle_to_locomotion", "Idle", "Locomotion")
                .when(TransitionCondition::SpeedAtLeast(0.25)),
        )
        .add_transition(
            TransitionDefinition::switch("locomotion_to_idle", "Locomotion", "Idle")
                .when(TransitionCondition::SpeedAtMost(0.1)),
        )
        .add_transition(
            TransitionDefinition::switch("leave_ground", "Grounded", "JumpStart")
                .when(TransitionCondition::Grounded(false))
                .when(TransitionCondition::VerticalVelocityAtLeast(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("jump_to_airborne", "JumpStart", "Airborne")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::switch("airborne_to_land", "Airborne", "Land")
                .when(TransitionCondition::Grounded(true))
                .when(TransitionCondition::VerticalVelocityAtMost(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("land_to_idle", "Land", "Idle")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::push("attack_push", TransitionSource::Any, "Attack")
                .when(TransitionCondition::ActionRequested("attack".into()))
                .with_priority(10),
        )
        .add_transition(
            TransitionDefinition::push("reload_push", TransitionSource::Any, "Reload")
                .when(TransitionCondition::ActionRequested("reload".into()))
                .with_priority(20)
                .with_push_conflict_policy(PushConflictPolicy::Reject),
        )
        .add_transition(
            TransitionDefinition::push("emote_push", TransitionSource::Any, "Emote")
                .when(TransitionCondition::ActionRequested("emote".into()))
                .with_push_conflict_policy(PushConflictPolicy::ReplaceTop),
        )
        .add_transition(
            TransitionDefinition::pop("attack_complete", "Attack")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::pop("reload_complete", "Reload")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::pop("emote_complete", "Emote")
                .when(TransitionCondition::AnimationFinished),
        )
}

pub fn setup_basic_3d_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.8, 10.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 24_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(24.0, 24.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.20, 0.18),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));
}

/// Builds the showcase animation bridge with locomotion blend-tree and overlay clips wired up.
pub fn build_animation_bridge(
    animations: &mut Assets<AnimationClip>,
    graphs: &mut Assets<AnimationGraph>,
) -> BevyAnimationBridge {
    let target = AnimationTargetId::from_name(&Name::new(RIG_NAME));
    let clips = [
        animations.add(idle_clip(target)),
        animations.add(walk_clip(target)),
        animations.add(run_clip(target)),
        animations.add(jump_start_clip(target)),
        animations.add(airborne_clip(target)),
        animations.add(land_clip(target)),
        animations.add(attack_clip(target)),
        animations.add(reload_clip(target)),
        animations.add(emote_clip(target)),
        animations.add(aim_overlay_clip(target)),
    ];
    let (graph, nodes) = AnimationGraph::from_clips(clips);
    let graph_handle = graphs.add(graph);
    let bindings = vec![
        BevyAnimationBinding::new("idle", nodes[0].index() as u32, Some(1.1)),
        BevyAnimationBinding::new("walk", nodes[1].index() as u32, Some(0.72)),
        BevyAnimationBinding::new("run", nodes[2].index() as u32, Some(0.65)),
        BevyAnimationBinding::new("jump_start", nodes[3].index() as u32, Some(0.22))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("airborne", nodes[4].index() as u32, Some(0.55)),
        BevyAnimationBinding::new("land", nodes[5].index() as u32, Some(0.18))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("attack", nodes[6].index() as u32, Some(0.35))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("reload", nodes[7].index() as u32, Some(0.8))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("emote", nodes[8].index() as u32, Some(0.55))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("aim_overlay", nodes[9].index() as u32, Some(0.65)),
    ];
    BevyAnimationBridge {
        player_entity: None,
        graph_handle,
        bindings,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_demo_character(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    definition_id: impl Into<CharacterStateMachineDefinitionId>,
    bridge: BevyAnimationBridge,
    color: Color,
    position: Vec3,
    name: &str,
) -> Entity {
    let root = commands
        .spawn((
            Name::new(name.to_string()),
            CharacterStateMachine::new(definition_id.into()),
            CharacterAnimationFacts {
                grounded: true,
                ..default()
            },
            CharacterAnimationRequests::default(),
            AnimationPlayer::default(),
            bridge,
            Visibility::default(),
            Transform::from_translation(position),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Name::new(RIG_NAME),
            Mesh3d(meshes.add(Capsule3d::new(0.38, 1.2))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.86,
                ..default()
            })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            AnimationTargetId::from_name(&Name::new(RIG_NAME)),
            AnimatedBy(root),
        ));
    });

    root
}

fn idle_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.0, 1.0, 0.0)),
            (0.55, Vec3::new(0.0, 1.08, 0.0)),
            (1.1, Vec3::new(0.0, 1.0, 0.0)),
        ],
    );
    clip
}

fn walk_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.0, 1.0, 0.0)),
            (0.18, Vec3::new(0.0, 1.12, 0.0)),
            (0.36, Vec3::new(0.0, 1.0, 0.0)),
            (0.54, Vec3::new(0.0, 1.12, 0.0)),
            (0.72, Vec3::new(0.0, 1.0, 0.0)),
        ],
    );
    add_scale_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(1.0, 1.0, 1.0)),
            (0.18, Vec3::new(1.01, 0.97, 1.01)),
            (0.36, Vec3::new(0.99, 1.03, 0.99)),
            (0.54, Vec3::new(1.01, 0.97, 1.01)),
            (0.72, Vec3::new(1.0, 1.0, 1.0)),
        ],
    );
    clip
}

fn run_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.0, 1.0, 0.0)),
            (0.16, Vec3::new(0.0, 1.22, 0.0)),
            (0.32, Vec3::new(0.0, 1.0, 0.0)),
            (0.48, Vec3::new(0.0, 1.22, 0.0)),
            (0.65, Vec3::new(0.0, 1.0, 0.0)),
        ],
    );
    add_scale_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(1.0, 1.0, 1.0)),
            (0.16, Vec3::new(1.02, 0.94, 1.02)),
            (0.32, Vec3::new(0.98, 1.06, 0.98)),
            (0.48, Vec3::new(1.02, 0.94, 1.02)),
            (0.65, Vec3::new(1.0, 1.0, 1.0)),
        ],
    );
    clip
}

fn jump_start_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.0, 1.0, 0.0)),
            (0.12, Vec3::new(0.0, 1.3, 0.0)),
            (0.22, Vec3::new(0.0, 1.6, -0.1)),
        ],
    );
    add_rotation_curve(
        &mut clip,
        target,
        &[(0.0, Quat::IDENTITY), (0.22, Quat::from_rotation_x(-0.3))],
    );
    clip
}

fn airborne_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.0, 1.22, 0.0)),
            (0.27, Vec3::new(0.0, 0.92, 0.0)),
            (0.55, Vec3::new(0.0, 1.22, 0.0)),
        ],
    );
    add_scale_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.96, 1.06, 0.96)),
            (0.27, Vec3::new(1.04, 0.92, 1.04)),
            (0.55, Vec3::new(0.96, 1.06, 0.96)),
        ],
    );
    clip
}

fn land_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.0, 1.0, 0.0)),
            (0.18, Vec3::new(0.0, 1.0, 0.0)),
        ],
    );
    add_scale_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(1.0, 1.0, 1.0)),
            (0.08, Vec3::new(1.12, 0.84, 1.12)),
            (0.18, Vec3::new(1.0, 1.0, 1.0)),
        ],
    );
    clip
}

fn attack_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_rotation_curve(
        &mut clip,
        target,
        &[
            (0.0, Quat::IDENTITY),
            (0.12, Quat::from_rotation_z(-0.8)),
            (0.2, Quat::from_rotation_z(0.6)),
            (0.35, Quat::IDENTITY),
        ],
    );
    clip
}

fn reload_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_rotation_curve(
        &mut clip,
        target,
        &[
            (0.0, Quat::IDENTITY),
            (0.4, Quat::from_rotation_y(0.45)),
            (0.8, Quat::IDENTITY),
        ],
    );
    add_translation_curve(
        &mut clip,
        target,
        &[
            (0.0, Vec3::new(0.0, 1.0, 0.0)),
            (0.4, Vec3::new(0.1, 1.08, 0.0)),
            (0.8, Vec3::new(0.0, 1.0, 0.0)),
        ],
    );
    clip
}

fn emote_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_rotation_curve(
        &mut clip,
        target,
        &[
            (0.0, Quat::IDENTITY),
            (0.18, Quat::from_rotation_y(PI / 8.0)),
            (0.36, Quat::from_rotation_y(-PI / 8.0)),
            (0.55, Quat::IDENTITY),
        ],
    );
    clip
}

fn aim_overlay_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_rotation_curve(
        &mut clip,
        target,
        &[
            (0.0, Quat::from_rotation_z(-0.10)),
            (0.32, Quat::from_rotation_z(0.14)),
            (0.65, Quat::from_rotation_z(-0.10)),
        ],
    );
    clip
}

fn add_translation_curve(
    clip: &mut AnimationClip,
    target: AnimationTargetId,
    points: &[(f32, Vec3)],
) {
    clip.add_curve_to_target(
        target,
        AnimatableCurve::new(
            animated_field!(Transform::translation),
            UnevenSampleAutoCurve::new(points.iter().copied()).unwrap(),
        ),
    );
}

fn add_rotation_curve(clip: &mut AnimationClip, target: AnimationTargetId, points: &[(f32, Quat)]) {
    clip.add_curve_to_target(
        target,
        AnimatableCurve::new(
            animated_field!(Transform::rotation),
            UnevenSampleAutoCurve::new(points.iter().copied()).unwrap(),
        ),
    );
}

fn add_scale_curve(clip: &mut AnimationClip, target: AnimationTargetId, points: &[(f32, Vec3)]) {
    clip.add_curve_to_target(
        target,
        AnimatableCurve::new(
            animated_field!(Transform::scale),
            UnevenSampleAutoCurve::new(points.iter().copied()).unwrap(),
        ),
    );
}
