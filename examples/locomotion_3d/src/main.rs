use std::f32::consts::PI;

use bevy::{
    animation::{AnimatedBy, AnimationTargetId, animated_field},
    prelude::*,
};
use saddle_character_state_machine::*;
use saddle_character_state_machine_example_support as support;
use saddle_pane::prelude::*;

const RIG_NAME: &str = "Rig";

#[derive(Component)]
struct DemoCharacter;

#[derive(Component)]
struct DemoHud;

#[derive(Resource, Default)]
struct JumpState {
    upward_time: f32,
}

#[derive(Resource, Pane)]
#[pane(title = "Locomotion 3D")]
struct LocomotionPane {
    #[pane(tab = "Movement", slider, min = 0.0, max = 1.0, step = 0.05)]
    walk_speed: f32,
    #[pane(tab = "Movement", slider, min = 0.2, max = 1.4, step = 0.05)]
    run_speed: f32,
    #[pane(tab = "Movement", slider, min = 0.5, max = 4.0, step = 0.1)]
    translation_speed: f32,
    #[pane(tab = "Movement", slider, min = 1.0, max = 8.0, step = 0.1)]
    jump_velocity: f32,
    #[pane(tab = "Movement", slider, min = 0.0, max = 0.6, step = 0.02)]
    jump_hold_seconds: f32,
    #[pane(tab = "Movement", slider, min = 4.0, max = 24.0, step = 0.5)]
    gravity: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    active_bindings: String,
}

impl Default for LocomotionPane {
    fn default() -> Self {
        Self {
            walk_speed: 0.55,
            run_speed: 1.0,
            translation_speed: 1.8,
            jump_velocity: 4.6,
            jump_hold_seconds: 0.24,
            gravity: 16.0,
            current_state: "Idle".into(),
            active_bindings: "idle:1.00".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// State machine definition: hierarchical states, blend trees, transitions.
// ---------------------------------------------------------------------------

fn build_locomotion_definition() -> CharacterStateMachineDefinition {
    CharacterStateMachineDefinition::new("locomotion_3d", "Idle")
        .with_fallback_state("Idle")
        .with_default_blend(BlendDefinition::new(0.18).with_easing(BlendEasing::SineInOut))
        // -- Grounded states --
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
        // -- Airborne states --
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
        // -- Action overlays --
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
        // -- Transitions --
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

// ---------------------------------------------------------------------------
// Animation bridge: maps state machine bindings to Bevy animation clips.
// ---------------------------------------------------------------------------

fn build_locomotion_bridge(
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
    ];

    BevyAnimationBridge {
        player_entity: None,
        graph_handle,
        bindings,
    }
}

// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, support::pane_plugins()))
        .add_plugins(CharacterStateMachinePlugin::always_on(Update))
        .insert_resource(JumpState::default())
        .init_resource::<LocomotionPane>()
        .register_pane::<LocomotionPane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (control_character, update_hud))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut library: ResMut<CharacterStateMachineLibrary>,
) {
    // -- 3D scene: camera, light, ground --
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.8, 10.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 24_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(24.0, 24.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.20, 0.18),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    // -- Register the state machine definition --
    let definition_id = library
        .register(build_locomotion_definition())
        .unwrap();
    let bridge = build_locomotion_bridge(&mut animations, &mut graphs);

    // -- Spawn the character entity with state machine + animation components --
    let root = commands
        .spawn((
            Name::new("Locomotion Demo Character"),
            CharacterStateMachine::new(definition_id),
            CharacterAnimationFacts {
                grounded: true,
                ..default()
            },
            CharacterAnimationRequests::default(),
            AnimationPlayer::default(),
            bridge,
            Visibility::default(),
            Transform::from_translation(Vec3::ZERO),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Name::new(RIG_NAME),
            Mesh3d(meshes.add(Capsule3d::new(0.38, 1.2))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.82, 0.47, 0.24),
                perceptual_roughness: 0.86,
                ..default()
            })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            AnimationTargetId::from_name(&Name::new(RIG_NAME)),
            AnimatedBy(root),
        ));
    });

    commands.entity(root).insert(DemoCharacter);

    commands.spawn((
        DemoHud,
        Text::new("W: walk | Shift: run | Space: jump"),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.05, 0.08, 0.74)),
        TextFont {
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn control_character(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut jump_state: ResMut<JumpState>,
    pane: Res<LocomotionPane>,
    mut query: Query<(&mut Transform, &mut CharacterAnimationFacts), With<DemoCharacter>>,
) {
    for (mut transform, mut facts) in &mut query {
        let moving = keyboard.pressed(KeyCode::KeyW);
        let sprinting = keyboard.pressed(KeyCode::ShiftLeft);
        facts.speed = if moving {
            if sprinting {
                pane.run_speed
            } else {
                pane.walk_speed
            }
        } else {
            0.0
        };
        facts.locomotion_mode = if moving {
            LocomotionMode::Run
        } else {
            LocomotionMode::Idle
        };
        if moving {
            transform.translation.x += time.delta_secs() * pane.translation_speed;
        }

        if keyboard.just_pressed(KeyCode::Space) && facts.grounded {
            facts.grounded = false;
            facts.vertical_velocity = pane.jump_velocity;
            jump_state.upward_time = pane.jump_hold_seconds;
        }

        if !facts.grounded {
            if jump_state.upward_time > 0.0 {
                jump_state.upward_time -= time.delta_secs();
                facts.vertical_velocity = pane.jump_velocity;
            } else {
                facts.vertical_velocity -= pane.gravity * time.delta_secs();
            }

            transform.translation.y += facts.vertical_velocity * time.delta_secs();
            if transform.translation.y <= 0.0 {
                transform.translation.y = 0.0;
                facts.grounded = true;
                facts.vertical_velocity = -1.0;
            }
        } else {
            facts.vertical_velocity = 0.0;
        }
    }
}

fn update_hud(
    runtime: Single<
        (&CharacterStateMachineRuntime, &CharacterAnimationSelection),
        With<DemoCharacter>,
    >,
    mut text: Single<&mut Text, With<DemoHud>>,
    mut pane: ResMut<LocomotionPane>,
) {
    let (runtime, selection) = *runtime;
    let current = runtime
        .current_state
        .as_ref()
        .map(|state| state.0.as_str())
        .unwrap_or("none");
    let stack = runtime
        .state_stack
        .iter()
        .map(|frame| frame.state.0.as_str())
        .collect::<Vec<_>>()
        .join(" -> ");

    let active = selection
        .active_bindings
        .iter()
        .map(|binding| format!("{}:{:.2}", binding.binding.0, binding.weight))
        .collect::<Vec<_>>()
        .join(", ");

    text.0 = format!(
        "W: walk | Shift: run | Space: jump\nstate: {current}\nstack: {stack}\nactive: {active}\nnormalized: {:.2}",
        runtime.normalized_time
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.active_bindings = active;
}

// ---------------------------------------------------------------------------
// Procedural animation clips (capsule-scale demonstrations)
// ---------------------------------------------------------------------------

fn idle_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(&mut clip, target, &[
        (0.0, Vec3::ZERO),
        (0.55, Vec3::new(0.0, 0.08, 0.0)),
        (1.1, Vec3::ZERO),
    ]);
    clip
}

fn walk_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(&mut clip, target, &[
        (0.0, Vec3::ZERO),
        (0.18, Vec3::new(0.0, 0.12, 0.0)),
        (0.36, Vec3::ZERO),
        (0.54, Vec3::new(0.0, 0.12, 0.0)),
        (0.72, Vec3::ZERO),
    ]);
    add_scale_curve(&mut clip, target, &[
        (0.0, Vec3::ONE),
        (0.18, Vec3::new(1.01, 0.97, 1.01)),
        (0.36, Vec3::new(0.99, 1.03, 0.99)),
        (0.54, Vec3::new(1.01, 0.97, 1.01)),
        (0.72, Vec3::ONE),
    ]);
    clip
}

fn run_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(&mut clip, target, &[
        (0.0, Vec3::ZERO),
        (0.16, Vec3::new(0.0, 0.22, 0.0)),
        (0.32, Vec3::ZERO),
        (0.48, Vec3::new(0.0, 0.22, 0.0)),
        (0.65, Vec3::ZERO),
    ]);
    add_scale_curve(&mut clip, target, &[
        (0.0, Vec3::ONE),
        (0.16, Vec3::new(1.02, 0.94, 1.02)),
        (0.32, Vec3::new(0.98, 1.06, 0.98)),
        (0.48, Vec3::new(1.02, 0.94, 1.02)),
        (0.65, Vec3::ONE),
    ]);
    clip
}

fn jump_start_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(&mut clip, target, &[
        (0.0, Vec3::ZERO),
        (0.12, Vec3::new(0.0, 0.3, 0.0)),
        (0.22, Vec3::new(0.0, 0.6, -0.1)),
    ]);
    add_rotation_curve(&mut clip, target, &[
        (0.0, Quat::IDENTITY),
        (0.22, Quat::from_rotation_x(-0.3)),
    ]);
    clip
}

fn airborne_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_translation_curve(&mut clip, target, &[
        (0.0, Vec3::new(0.0, 0.22, 0.0)),
        (0.27, Vec3::new(0.0, -0.08, 0.0)),
        (0.55, Vec3::new(0.0, 0.22, 0.0)),
    ]);
    add_scale_curve(&mut clip, target, &[
        (0.0, Vec3::new(0.96, 1.06, 0.96)),
        (0.27, Vec3::new(1.04, 0.92, 1.04)),
        (0.55, Vec3::new(0.96, 1.06, 0.96)),
    ]);
    clip
}

fn land_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_scale_curve(&mut clip, target, &[
        (0.0, Vec3::ONE),
        (0.08, Vec3::new(1.12, 0.84, 1.12)),
        (0.18, Vec3::ONE),
    ]);
    clip
}

fn attack_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_rotation_curve(&mut clip, target, &[
        (0.0, Quat::IDENTITY),
        (0.12, Quat::from_rotation_z(-0.8)),
        (0.2, Quat::from_rotation_z(0.6)),
        (0.35, Quat::IDENTITY),
    ]);
    clip
}

fn reload_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_rotation_curve(&mut clip, target, &[
        (0.0, Quat::IDENTITY),
        (0.4, Quat::from_rotation_y(0.45)),
        (0.8, Quat::IDENTITY),
    ]);
    add_translation_curve(&mut clip, target, &[
        (0.0, Vec3::ZERO),
        (0.4, Vec3::new(0.1, 0.08, 0.0)),
        (0.8, Vec3::ZERO),
    ]);
    clip
}

fn emote_clip(target: AnimationTargetId) -> AnimationClip {
    let mut clip = AnimationClip::default();
    add_rotation_curve(&mut clip, target, &[
        (0.0, Quat::IDENTITY),
        (0.18, Quat::from_rotation_y(PI / 8.0)),
        (0.36, Quat::from_rotation_y(-PI / 8.0)),
        (0.55, Quat::IDENTITY),
    ]);
    clip
}

fn add_translation_curve(clip: &mut AnimationClip, target: AnimationTargetId, points: &[(f32, Vec3)]) {
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
