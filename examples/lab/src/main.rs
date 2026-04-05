#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use saddle_character_state_machine_example_support as support;

use std::f32::consts::PI;

use bevy::{
    animation::{AnimatedBy, AnimationTargetId, animated_field},
    prelude::*,
};
use saddle_character_state_machine::*;
use saddle_pane::prelude::*;

#[derive(Component)]
struct LabCharacter;

#[derive(Component)]
struct LabOverlay;

#[derive(Resource, Default)]
struct AirState {
    airborne_time: f32,
}

#[derive(Resource, Default)]
struct FiredEvents {
    count: usize,
    last_event_id: Option<String>,
}

#[derive(Resource, Pane)]
#[pane(title = "State Machine Lab")]
struct LabPane {
    #[pane(tab = "Drive", slider, min = 0.0, max = 1.4, step = 0.05)]
    move_speed: f32,
    #[pane(tab = "Drive", slider, min = 0.5, max = 4.0, step = 0.1)]
    translation_speed: f32,
    #[pane(tab = "Drive", slider, min = 1.0, max = 8.0, step = 0.1)]
    jump_velocity: f32,
    #[pane(tab = "Drive", slider, min = 0.0, max = 0.6, step = 0.02)]
    jump_hold_seconds: f32,
    #[pane(tab = "Drive", slider, min = 4.0, max = 24.0, step = 0.5)]
    gravity: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    current_binding: String,
    #[pane(tab = "Runtime", monitor)]
    queued_requests: String,
}

impl Default for LabPane {
    fn default() -> Self {
        Self {
            move_speed: 1.0,
            translation_speed: 1.8,
            jump_velocity: 4.8,
            jump_hold_seconds: 0.24,
            gravity: 15.0,
            current_state: "Idle".into(),
            current_binding: "idle".into(),
            queued_requests: "none".into(),
        }
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LabSystems {
    Control,
    Overlay,
}

const RIG_NAME: &str = "Rig";

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        CharacterStateMachinePlugin::always_on(Update),
        support::pane_plugins(),
    ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::CharacterStateMachineLabE2EPlugin);
    app.insert_resource(AirState::default());
    app.init_resource::<FiredEvents>();
    app.init_resource::<LabPane>();
    app.register_pane::<LabPane>();
    app.configure_sets(
        Update,
        (
            LabSystems::Control,
            CharacterStateMachineSystems::GatherFacts,
            CharacterStateMachineSystems::ResolveTransitions,
            CharacterStateMachineSystems::ApplyAnimation,
            CharacterStateMachineSystems::Cleanup,
            LabSystems::Overlay,
        )
            .chain(),
    );
    #[cfg(feature = "e2e")]
    app.configure_sets(Update, saddle_bevy_e2e::E2ESet.before(LabSystems::Control));
    app.add_systems(Startup, setup);
    app.add_systems(Update, control_character.in_set(LabSystems::Control));
    app.add_systems(Update, collect_animation_events.in_set(LabSystems::Overlay));
    app.add_systems(Update, update_overlay.in_set(LabSystems::Overlay));

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut library: ResMut<CharacterStateMachineLibrary>,
) {
    commands.spawn((
        Name::new("Lab Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Lab Sun"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 24_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Lab Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(24.0, 24.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.17, 0.19, 0.18),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    let definition_id = library.register(build_definition()).unwrap();
    let (graph_handle, bindings) = build_demo_graph(&mut animations, &mut graphs);

    let root = commands
        .spawn((
            Name::new("State Machine Hero"),
            LabCharacter,
            CharacterStateMachine::new(definition_id),
            CharacterAnimationFacts {
                grounded: true,
                ..default()
            },
            CharacterAnimationRequests::default(),
            AnimationPlayer::default(),
            BevyAnimationBridge {
                player_entity: None,
                graph_handle,
                bindings,
            },
            Visibility::default(),
            Transform::from_translation(Vec3::ZERO),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Name::new(RIG_NAME),
            Mesh3d(meshes.add(Capsule3d::new(0.38, 1.2))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.91, 0.54, 0.23),
                perceptual_roughness: 0.86,
                ..default()
            })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            AnimationTargetId::from_name(&Name::new(RIG_NAME)),
            AnimatedBy(root),
        ));
    });

    commands.spawn((
        Name::new("Character State Machine Overlay"),
        LabOverlay,
        Text::new("Character state machine lab"),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(430.0),
            padding: UiRect::all(px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.04, 0.06, 0.80)),
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
    mut air_state: ResMut<AirState>,
    pane: Res<LabPane>,
    mut query: Query<
        (
            &mut Transform,
            &mut CharacterAnimationFacts,
            &mut CharacterAnimationRequests,
        ),
        With<LabCharacter>,
    >,
) {
    for (mut transform, mut facts, mut requests) in &mut query {
        let moving = keyboard.pressed(KeyCode::KeyW);
        facts.speed = if moving { pane.move_speed } else { 0.0 };
        facts.locomotion_mode = if moving {
            LocomotionMode::Run
        } else {
            LocomotionMode::Idle
        };
        if moving {
            transform.translation.x = (transform.translation.x
                + pane.translation_speed * time.delta_secs())
            .clamp(-4.0, 4.0);
        }

        if keyboard.just_pressed(KeyCode::KeyJ) {
            requests.push("attack");
        }
        if keyboard.just_pressed(KeyCode::KeyR) {
            requests.push("reload");
        }
        if keyboard.just_pressed(KeyCode::KeyE) {
            requests.push("emote");
        }
        if keyboard.just_pressed(KeyCode::Space) && facts.grounded {
            facts.grounded = false;
            facts.vertical_velocity = pane.jump_velocity;
            air_state.airborne_time = pane.jump_hold_seconds;
        }

        if !facts.grounded {
            if air_state.airborne_time > 0.0 {
                air_state.airborne_time -= time.delta_secs();
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

fn collect_animation_events(
    mut events: MessageReader<AnimationEventFired>,
    mut fired: ResMut<FiredEvents>,
) {
    for event in events.read() {
        fired.count += 1;
        fired.last_event_id = Some(event.event_id.0.clone());
    }
}

fn update_overlay(
    runtime: Single<
        (&CharacterStateMachineRuntime, &CharacterAnimationSelection),
        With<LabCharacter>,
    >,
    facts: Single<&CharacterAnimationFacts, With<LabCharacter>>,
    mut overlay: Single<&mut Text, With<LabOverlay>>,
    mut pane: ResMut<LabPane>,
) {
    let (runtime, selection) = *runtime;
    let facts = *facts;
    let current = runtime
        .current_state
        .as_ref()
        .map(|state| state.0.as_str())
        .unwrap_or("none");
    let previous = runtime
        .previous_state
        .as_ref()
        .map(|state| state.0.as_str())
        .unwrap_or("none");
    let stack = runtime
        .state_stack
        .iter()
        .map(|frame| frame.state.0.as_str())
        .collect::<Vec<_>>()
        .join(" -> ");
    let queued = runtime
        .queued_requests
        .iter()
        .map(|request| request.0.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let last_transition = runtime
        .last_transition
        .as_ref()
        .and_then(|trace| trace.transition_id.as_ref())
        .map(|transition| transition.0.as_str())
        .unwrap_or("none");
    let last_reason = runtime
        .last_transition
        .as_ref()
        .and_then(|trace| trace.reason.as_ref())
        .map(|reason| format!("{reason:?}"))
        .unwrap_or_else(|| "none".to_string());

    overlay.0 = format!(
        "Character State Machine Lab\ncurrent: {current}\nprevious: {previous}\nstack: {stack}\npending: {}\nqueued: {queued}\nstate time: {:.2}\nnormalized: {:.2}\nspeed: {:.2}\ngrounded: {}\nvertical velocity: {:.2}\nlast transition: {last_transition}\nlast reason: {last_reason}",
        runtime
            .pending_request
            .as_ref()
            .map(|request| request.0.as_str())
            .unwrap_or("none"),
        runtime.state_elapsed_seconds,
        runtime.normalized_time,
        facts.speed,
        facts.grounded,
        facts.vertical_velocity,
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.current_binding = selection
        .binding
        .as_ref()
        .map(|binding| binding.0.clone())
        .unwrap_or_else(|| "none".into());
    pane.queued_requests = if queued.is_empty() {
        "none".into()
    } else {
        queued
    };
}

fn build_definition() -> CharacterStateMachineDefinition {
    CharacterStateMachineDefinition::new("lab", "Idle")
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
                .with_binding("locomotion")
                .with_expected_duration(0.65)
                .with_event("footstep_left", 0.25)
                .with_event("footstep_right", 0.75),
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

fn build_demo_graph(
    animations: &mut Assets<AnimationClip>,
    graphs: &mut Assets<AnimationGraph>,
) -> (Handle<AnimationGraph>, Vec<BevyAnimationBinding>) {
    let target = AnimationTargetId::from_name(&Name::new(RIG_NAME));
    let clips = [
        animations.add(idle_clip(target)),
        animations.add(locomotion_clip(target)),
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
        BevyAnimationBinding::new("locomotion", nodes[1].index() as u32, Some(0.65)),
        BevyAnimationBinding::new("jump_start", nodes[2].index() as u32, Some(0.22))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("airborne", nodes[3].index() as u32, Some(0.55)),
        BevyAnimationBinding::new("land", nodes[4].index() as u32, Some(0.18))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("attack", nodes[5].index() as u32, Some(0.35))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("reload", nodes[6].index() as u32, Some(0.8))
            .with_repeat(PlaybackRepeat::Never),
        BevyAnimationBinding::new("emote", nodes[7].index() as u32, Some(0.55))
            .with_repeat(PlaybackRepeat::Never),
    ];
    (graph_handle, bindings)
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

fn locomotion_clip(target: AnimationTargetId) -> AnimationClip {
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
