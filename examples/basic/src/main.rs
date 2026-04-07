use saddle_character_state_machine_example_support as support;

use bevy::prelude::*;
use saddle_character_state_machine::extensions::{CharacterAnimationFactsExt, conditions, keys};
use saddle_character_state_machine::*;
use saddle_pane::prelude::*;

#[derive(Component)]
struct DemoSprite;

#[derive(Component)]
struct DemoLabel;

#[derive(Resource, Pane)]
#[pane(title = "State Machine Basic")]
struct BasicPane {
    #[pane(tab = "Drive", slider, min = 0.0, max = 1.5, step = 0.05)]
    move_speed: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    current_binding: String,
    #[pane(tab = "Runtime", monitor)]
    last_transition: String,
}

impl Default for BasicPane {
    fn default() -> Self {
        Self {
            move_speed: 1.0,
            current_state: "Idle".into(),
            current_binding: "idle".into(),
            last_transition: "none".into(),
        }
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, support::pane_plugins()))
        .add_plugins(CharacterStateMachinePlugin::always_on(Update))
        .init_resource::<BasicPane>()
        .register_pane::<BasicPane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard_drive, paint_sprite, update_label))
        .run();
}

fn setup(mut commands: Commands, mut library: ResMut<CharacterStateMachineLibrary>) {
    commands.spawn((Name::new("Basic Camera"), Camera2d));

    // State graph:
    //   Idle  <--speed-->  Move
    //     \                  /
    //      +-- Pulse (push) --+   (Space triggers one-shot overlay, pops when done)
    let definition = CharacterStateMachineDefinition::new("basic_demo", "Idle")
        .with_fallback_state("Idle")
        .add_state(StateDefinition::new("Idle").with_binding("idle"))
        .add_state(StateDefinition::new("Move").with_binding("move"))
        .add_state(
            StateDefinition::new("Pulse")
                .transient()
                .with_binding("pulse")
                .with_expected_duration(0.4),
        )
        .add_transition(
            TransitionDefinition::switch("idle_to_move", "Idle", "Move")
                .when(conditions::speed_at_least(0.25)),
        )
        .add_transition(
            TransitionDefinition::switch("move_to_idle", "Move", "Idle")
                .when(conditions::speed_at_most(0.1)),
        )
        .add_transition(
            TransitionDefinition::push("pulse_push", TransitionSource::Any, "Pulse")
                .when(TransitionCondition::ActionRequested("pulse".into())),
        )
        .add_transition(
            TransitionDefinition::pop("pulse_done", "Pulse")
                .when(TransitionCondition::AnimationFinished),
        );
    let definition_id = library.register(definition).unwrap();

    commands.spawn((
        Name::new("Basic Demo Character"),
        CharacterStateMachine::new(definition_id),
        CharacterAnimationFacts::default().with_boolean(keys::GROUNDED, true),
        CharacterAnimationRequests::default(),
        DemoSprite,
        Sprite::from_color(Color::srgb(0.18, 0.47, 0.78), Vec2::new(180.0, 180.0)),
    ));

    commands.spawn((
        Name::new("Basic HUD"),
        DemoLabel,
        Text::new("Left/Right: move | Space: pulse\nPane: tune move speed live"),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.75)),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

/// Reads keyboard input and sets animation facts + action requests.
/// Left/Right arrows set speed (triggers Idle <-> Move transitions).
/// Space pushes the "pulse" action (temporary overlay state).
fn keyboard_drive(
    keyboard: Res<ButtonInput<KeyCode>>,
    pane: Res<BasicPane>,
    mut query: Query<
        (
            &mut CharacterAnimationFacts,
            &mut CharacterAnimationRequests,
        ),
        With<DemoSprite>,
    >,
) {
    for (mut facts, mut requests) in &mut query {
        let moving =
            keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::ArrowLeft);
        facts.set_speed(if moving { pane.move_speed } else { 0.0 });

        if keyboard.just_pressed(KeyCode::Space) {
            requests.push("pulse");
        }
    }
}

fn paint_sprite(mut sprites: Query<(&CharacterAnimationSelection, &mut Sprite), With<DemoSprite>>) {
    for (selection, mut sprite) in &mut sprites {
        match selection.binding.as_ref().map(|binding| binding.0.as_str()) {
            Some("move") => {
                sprite.color = Color::srgb(0.20, 0.70, 0.38);
                sprite.custom_size = Some(Vec2::new(220.0, 150.0));
            }
            Some("pulse") => {
                sprite.color = Color::srgb(0.92, 0.44, 0.24);
                sprite.custom_size = Some(Vec2::new(240.0, 210.0));
            }
            _ => {
                sprite.color = Color::srgb(0.18, 0.47, 0.78);
                sprite.custom_size = Some(Vec2::new(180.0, 180.0));
            }
        }
    }
}

fn update_label(
    runtime: Single<&CharacterStateMachineRuntime, With<DemoSprite>>,
    mut label: Single<&mut Text, With<DemoLabel>>,
    mut pane: ResMut<BasicPane>,
) {
    let runtime = *runtime;
    let current = runtime
        .current_state
        .as_ref()
        .map(|state| state.0.as_str())
        .unwrap_or("none");
    let binding = runtime
        .current_binding
        .as_ref()
        .map(|binding| binding.0.as_str())
        .unwrap_or("none");
    let transition = runtime
        .last_transition
        .as_ref()
        .and_then(|trace| trace.transition_id.as_ref())
        .map(|transition| transition.0.as_str())
        .unwrap_or("none");

    label.0 = format!(
        "Left/Right: move | Space: pulse\nstate: {current}  binding: {binding}\nstack: {}  state time: {:.2}\nlast transition: {transition}",
        runtime.state_stack.len(),
        runtime.state_elapsed_seconds,
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.current_binding = binding.into();
    pane.last_transition = transition.into();
}
