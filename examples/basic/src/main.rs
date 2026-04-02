use bevy::prelude::*;
use saddle_character_state_machine::*;

#[derive(Component)]
struct DemoSprite;

#[derive(Component)]
struct DemoLabel;

#[derive(Resource, Default)]
struct DemoClock {
    elapsed: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CharacterStateMachinePlugin::always_on(Update))
        .insert_resource(DemoClock::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (drive_demo, paint_sprite, update_label))
        .run();
}

fn setup(mut commands: Commands, mut library: ResMut<CharacterStateMachineLibrary>) {
    commands.spawn(Camera2d);

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
                .when(TransitionCondition::SpeedAtLeast(0.25)),
        )
        .add_transition(
            TransitionDefinition::switch("move_to_idle", "Move", "Idle")
                .when(TransitionCondition::SpeedAtMost(0.1)),
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
        CharacterAnimationFacts {
            grounded: true,
            ..default()
        },
        CharacterAnimationRequests::default(),
        DemoSprite,
        Sprite::from_color(Color::srgb(0.18, 0.47, 0.78), Vec2::new(180.0, 180.0)),
    ));

    commands.spawn((
        DemoLabel,
        Text::new("saddle-character-state-machine basic"),
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

fn drive_demo(
    time: Res<Time>,
    mut clock: ResMut<DemoClock>,
    mut query: Query<
        (
            &mut CharacterAnimationFacts,
            &mut CharacterAnimationRequests,
        ),
        With<DemoSprite>,
    >,
) {
    clock.elapsed += time.delta_secs();
    let phase = clock.elapsed % 4.0;

    for (mut facts, mut requests) in &mut query {
        facts.speed = if phase < 1.5 { 0.0 } else { 1.0 };
        facts.locomotion_mode = if facts.speed > 0.0 {
            LocomotionMode::Run
        } else {
            LocomotionMode::Idle
        };
        if (3.0..3.1).contains(&phase) {
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
        "saddle-character-state-machine basic\nstate: {current}\nbinding: {binding}\nstate time: {:.2}\nlast transition: {transition}",
        runtime.state_elapsed_seconds
    );
}
