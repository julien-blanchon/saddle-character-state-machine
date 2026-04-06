use saddle_character_state_machine_example_support as support;

use bevy::prelude::*;
use saddle_character_state_machine::extensions::{CharacterAnimationFactsExt, conditions, keys};
use saddle_character_state_machine::*;
use saddle_pane::prelude::*;

#[derive(Component)]
struct StateNode {
    state_id: String,
}

#[derive(Component)]
struct PreviewHud;

#[derive(Component)]
struct PreviewCharacter;

#[derive(Resource, Default)]
struct DemoClock {
    elapsed: f32,
}

#[derive(Resource, Pane)]
#[pane(title = "Graph Preview")]
struct PreviewPane {
    #[pane(tab = "Drive", slider, min = 0.0, max = 1.4, step = 0.05)]
    speed: f32,
    #[pane(tab = "Drive")]
    grounded: bool,
    #[pane(tab = "Drive", slider, min = -4.0, max = 4.0, step = 0.1)]
    vertical_velocity: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    dot_graph_lines: String,
}

impl Default for PreviewPane {
    fn default() -> Self {
        Self {
            speed: 0.0,
            grounded: true,
            vertical_velocity: 0.0,
            current_state: "Idle".into(),
            dot_graph_lines: String::new(),
        }
    }
}

fn build_preview_definition() -> CharacterStateMachineDefinition {
    CharacterStateMachineDefinition::new("graph_preview", "Idle")
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
            StateDefinition::new("Emote")
                .transient()
                .with_binding("emote")
                .with_expected_duration(0.55),
        )
        .add_transition(
            TransitionDefinition::switch("idle_to_locomotion", "Idle", "Locomotion")
                .when(conditions::speed_at_least(0.25)),
        )
        .add_transition(
            TransitionDefinition::switch("locomotion_to_idle", "Locomotion", "Idle")
                .when(conditions::speed_at_most(0.1)),
        )
        .add_transition(
            TransitionDefinition::switch("leave_ground", "Grounded", "JumpStart")
                .when(conditions::grounded(false))
                .when(conditions::vertical_velocity_at_least(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("jump_to_airborne", "JumpStart", "Airborne")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::switch("airborne_to_land", "Airborne", "Land")
                .when(conditions::grounded(true))
                .when(conditions::vertical_velocity_at_most(0.0)),
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
            TransitionDefinition::push("emote_push", TransitionSource::Any, "Emote")
                .when(TransitionCondition::ActionRequested("emote".into()))
                .with_push_conflict_policy(PushConflictPolicy::ReplaceTop),
        )
        .add_transition(
            TransitionDefinition::pop("attack_complete", "Attack")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::pop("emote_complete", "Emote")
                .when(TransitionCondition::AnimationFinished),
        )
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, support::pane_plugins()))
        .add_plugins(CharacterStateMachinePlugin::always_on(Update))
        .init_resource::<DemoClock>()
        .init_resource::<PreviewPane>()
        .register_pane::<PreviewPane>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (drive_from_pane, highlight_active_state, update_hud),
        )
        .run();
}

fn state_position(state_id: &str) -> Vec2 {
    match state_id {
        "Grounded" => Vec2::new(-320.0, 180.0),
        "Idle" => Vec2::new(-180.0, 60.0),
        "Locomotion" => Vec2::new(120.0, 60.0),
        "JumpStart" => Vec2::new(-180.0, -120.0),
        "Airborne" => Vec2::new(0.0, -240.0),
        "Land" => Vec2::new(180.0, -120.0),
        "Attack" => Vec2::new(-320.0, -240.0),
        "Emote" => Vec2::new(320.0, -240.0),
        _ => Vec2::ZERO,
    }
}

fn state_color(state_id: &str) -> Color {
    match state_id {
        "Grounded" => Color::srgb(0.40, 0.40, 0.40),
        "Idle" => Color::srgb(0.28, 0.54, 0.90),
        "Locomotion" => Color::srgb(0.32, 0.78, 0.46),
        "JumpStart" => Color::srgb(0.97, 0.76, 0.25),
        "Airborne" => Color::srgb(0.98, 0.56, 0.28),
        "Land" => Color::srgb(0.73, 0.86, 0.95),
        "Attack" => Color::srgb(0.90, 0.22, 0.22),
        "Emote" => Color::srgb(0.95, 0.60, 0.85),
        _ => Color::srgb(0.5, 0.5, 0.5),
    }
}

fn setup(
    mut commands: Commands,
    mut library: ResMut<CharacterStateMachineLibrary>,
    mut pane: ResMut<PreviewPane>,
) {
    commands.spawn((Name::new("Camera"), Camera2d));

    let definition = build_preview_definition();
    let dot = definition.dot_graph();
    let line_count = dot.lines().count();
    pane.dot_graph_lines = format!("{line_count} lines (printed to console)");
    info!("DOT graph output:\n{dot}");

    let state_ids: Vec<String> = definition.states.iter().map(|s| s.id.0.clone()).collect();
    let definition_id = library.register(definition).unwrap();

    for state_id in &state_ids {
        let pos = state_position(state_id);
        let color = state_color(state_id);
        let size = if state_id == "Grounded" {
            Vec2::new(140.0, 44.0)
        } else {
            Vec2::new(130.0, 50.0)
        };

        commands
            .spawn((
                Name::new(format!("State: {state_id}")),
                StateNode {
                    state_id: state_id.clone(),
                },
                Sprite::from_color(color, size),
                Transform::from_xyz(pos.x, pos.y, 0.0),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text2d::new(state_id.clone()),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Transform::from_xyz(0.0, 0.0, 1.0),
                ));
            });
    }

    commands.spawn((
        Name::new("Graph Preview Character"),
        PreviewCharacter,
        CharacterStateMachine::new(definition_id),
        CharacterAnimationFacts::default().with_boolean(keys::GROUNDED, true),
        CharacterAnimationRequests::default(),
    ));

    commands.spawn((
        Name::new("Preview HUD"),
        PreviewHud,
        Text::new(
            "Graph Preview: visual state graph with live state highlighting\n\
             Pane: adjust speed, grounded, vertical_velocity to drive transitions\n\
             J: attack | E: emote",
        ),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(500.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.80)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn drive_from_pane(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut clock: ResMut<DemoClock>,
    pane: Res<PreviewPane>,
    mut query: Query<
        (
            &mut CharacterAnimationFacts,
            &mut CharacterAnimationRequests,
        ),
        With<PreviewCharacter>,
    >,
) {
    clock.elapsed += time.delta_secs();

    for (mut facts, mut requests) in &mut query {
        facts.set_speed(pane.speed);
        facts.set_grounded(pane.grounded);
        facts.set_vertical_velocity(pane.vertical_velocity);

        if keyboard.just_pressed(KeyCode::KeyJ) {
            requests.push("attack");
        }
        if keyboard.just_pressed(KeyCode::KeyE) {
            requests.push("emote");
        }
    }
}

fn highlight_active_state(
    runtime: Query<&CharacterStateMachineRuntime, With<PreviewCharacter>>,
    mut nodes: Query<(&StateNode, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    let Ok(runtime) = runtime.single() else {
        return;
    };
    let current = runtime
        .current_state
        .as_ref()
        .map(|state| state.0.as_str())
        .unwrap_or("");

    let pulse = (time.elapsed_secs() * 3.0).sin() * 0.08 + 1.0;

    for (node, mut sprite, mut transform) in &mut nodes {
        let base_color = state_color(&node.state_id);
        let is_active = node.state_id == current;
        let is_in_stack = runtime
            .state_stack
            .iter()
            .any(|frame| frame.state.0 == node.state_id);

        if is_active {
            sprite.color = Color::WHITE;
            let base_pos = state_position(&node.state_id);
            let base_size = if node.state_id == "Grounded" {
                Vec2::new(140.0, 44.0)
            } else {
                Vec2::new(130.0, 50.0)
            };
            sprite.custom_size = Some(base_size * pulse);
            transform.translation = Vec3::new(base_pos.x, base_pos.y, 2.0);
        } else if is_in_stack {
            sprite.color = base_color.with_alpha(0.85);
            let base_size = if node.state_id == "Grounded" {
                Vec2::new(140.0, 44.0)
            } else {
                Vec2::new(130.0, 50.0)
            };
            sprite.custom_size = Some(base_size);
            let base_pos = state_position(&node.state_id);
            transform.translation = Vec3::new(base_pos.x, base_pos.y, 1.0);
        } else {
            sprite.color = base_color.with_alpha(0.35);
            let base_size = if node.state_id == "Grounded" {
                Vec2::new(140.0, 44.0)
            } else {
                Vec2::new(130.0, 50.0)
            };
            sprite.custom_size = Some(base_size);
            let base_pos = state_position(&node.state_id);
            transform.translation = Vec3::new(base_pos.x, base_pos.y, 0.0);
        }
    }
}

fn update_hud(
    runtime: Query<
        (&CharacterStateMachineRuntime, &CharacterAnimationSelection),
        With<PreviewCharacter>,
    >,
    mut hud: Single<&mut Text, With<PreviewHud>>,
    mut pane: ResMut<PreviewPane>,
) {
    let Ok((runtime, _selection)) = runtime.single() else {
        return;
    };
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
    let last = runtime
        .last_transition
        .as_ref()
        .and_then(|trace| trace.transition_id.as_ref())
        .map(|transition| transition.0.as_str())
        .unwrap_or("none");

    hud.0 = format!(
        "Graph Preview: visual state graph with live state highlighting\n\
         Pane: adjust speed, grounded, vertical_velocity to drive transitions\n\
         J: attack | E: emote\n\
         \n\
         state: {current}\n\
         stack: {stack}\n\
         state time: {:.2}\n\
         last transition: {last}",
        runtime.state_elapsed_seconds,
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
}
