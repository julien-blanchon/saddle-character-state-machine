use saddle_character_state_machine_example_support as support;

use bevy::prelude::*;
use saddle_character_state_machine::*;
use saddle_pane::prelude::*;

#[derive(Component)]
struct PlatformerSprite;

#[derive(Component)]
struct PlatformerHud;

#[derive(Resource, Pane)]
#[pane(title = "Sprite 2D")]
struct SpritePane {
    #[pane(tab = "Movement", slider, min = 60.0, max = 320.0, step = 5.0)]
    move_speed: f32,
    #[pane(tab = "Movement", slider, min = 180.0, max = 600.0, step = 10.0)]
    jump_velocity: f32,
    #[pane(tab = "Movement", slider, min = 300.0, max = 1600.0, step = 25.0)]
    gravity: f32,
    #[pane(tab = "Movement", slider, min = 40.0, max = 220.0, step = 5.0)]
    wall_slide_speed: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    current_binding: String,
}

impl Default for SpritePane {
    fn default() -> Self {
        Self {
            move_speed: 180.0,
            jump_velocity: 420.0,
            gravity: 900.0,
            wall_slide_speed: 120.0,
            current_state: "Idle".into(),
            current_binding: "idle".into(),
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            support::pane_plugins(),
        ))
        .add_plugins(CharacterStateMachinePlugin::always_on(Update))
        .init_resource::<SpritePane>()
        .register_pane::<SpritePane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (drive_platformer, apply_sprite_style, update_hud))
        .run();
}

fn setup(mut commands: Commands, mut library: ResMut<CharacterStateMachineLibrary>) {
    commands.spawn(Camera2d);

    let definition = CharacterStateMachineDefinition::new("sprite_platformer", "Idle")
        .with_fallback_state("Idle")
        .add_state(StateDefinition::new("Grounded").with_binding("idle"))
        .add_state(
            StateDefinition::new("Idle")
                .with_parent("Grounded")
                .with_binding("idle"),
        )
        .add_state(
            StateDefinition::new("Run")
                .with_parent("Grounded")
                .with_binding("run")
                .with_expected_duration(0.45),
        )
        .add_state(
            StateDefinition::new("Jump")
                .transient()
                .with_binding("jump")
                .with_expected_duration(0.18),
        )
        .add_state(
            StateDefinition::new("Fall")
                .with_binding("fall")
                .with_expected_duration(0.38),
        )
        .add_state(
            StateDefinition::new("WallSlide")
                .with_binding("wall_slide")
                .with_expected_duration(0.3),
        )
        .add_state(
            StateDefinition::new("Land")
                .transient()
                .with_binding("land")
                .with_expected_duration(0.12),
        )
        .add_transition(
            TransitionDefinition::switch("idle_to_run", "Idle", "Run")
                .when(TransitionCondition::SpeedAtLeast(0.2)),
        )
        .add_transition(
            TransitionDefinition::switch("run_to_idle", "Run", "Idle")
                .when(TransitionCondition::SpeedAtMost(0.05)),
        )
        .add_transition(
            TransitionDefinition::switch("leave_ground", "Grounded", "Jump")
                .when(TransitionCondition::Grounded(false))
                .when(TransitionCondition::VerticalVelocityAtLeast(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("jump_to_fall", "Jump", "Fall")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::switch("fall_to_wall_slide", "Fall", "WallSlide")
                .when(TransitionCondition::WallContact(true)),
        )
        .add_transition(
            TransitionDefinition::switch("wall_slide_to_fall", "WallSlide", "Fall")
                .when(TransitionCondition::WallContact(false)),
        )
        .add_transition(
            TransitionDefinition::switch("fall_to_land", "Fall", "Land")
                .when(TransitionCondition::Grounded(true))
                .when(TransitionCondition::VerticalVelocityAtMost(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("land_to_idle", "Land", "Idle")
                .when(TransitionCondition::AnimationFinished),
        );
    let definition_id = library.register(definition).unwrap();

    commands.spawn((
        Name::new("2D Platformer Sprite"),
        PlatformerSprite,
        CharacterStateMachine::new(definition_id),
        CharacterAnimationFacts {
            grounded: true,
            ..default()
        },
        Sprite::from_color(Color::srgb(0.28, 0.54, 0.90), Vec2::new(70.0, 120.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        Sprite::from_color(Color::srgb(0.14, 0.16, 0.14), Vec2::new(700.0, 26.0)),
        Transform::from_xyz(0.0, -130.0, 0.0),
    ));

    commands.spawn((
        PlatformerHud,
        Text::new("Arrow keys: move, Space: jump"),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.74)),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn drive_platformer(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    pane: Res<SpritePane>,
    mut query: Query<(&mut Transform, &mut CharacterAnimationFacts), With<PlatformerSprite>>,
) {
    for (mut transform, mut facts) in &mut query {
        let axis = (keyboard.pressed(KeyCode::ArrowRight) as i8
            - keyboard.pressed(KeyCode::ArrowLeft) as i8) as f32;
        facts.speed = axis.abs();
        transform.translation.x = (transform.translation.x
            + axis * pane.move_speed * time.delta_secs())
        .clamp(-280.0, 280.0);

        if keyboard.just_pressed(KeyCode::Space) && facts.grounded {
            facts.grounded = false;
            facts.vertical_velocity = pane.jump_velocity;
        }

        if !facts.grounded {
            facts.vertical_velocity -= pane.gravity * time.delta_secs();
            transform.translation.y += facts.vertical_velocity * time.delta_secs();
            facts.wall_contact =
                transform.translation.x.abs() >= 275.0 && facts.vertical_velocity < 0.0;
            if facts.wall_contact {
                facts.vertical_velocity = facts.vertical_velocity.max(-pane.wall_slide_speed);
            }
            if transform.translation.y <= -70.0 {
                transform.translation.y = -70.0;
                facts.grounded = true;
                facts.wall_contact = false;
                facts.vertical_velocity = -1.0;
            }
        } else {
            facts.vertical_velocity = 0.0;
            facts.wall_contact = false;
        }
    }
}

fn apply_sprite_style(
    mut query: Query<(&CharacterAnimationSelection, &mut Sprite), With<PlatformerSprite>>,
) {
    for (selection, mut sprite) in &mut query {
        match selection.binding.as_ref().map(|binding| binding.0.as_str()) {
            Some("run") => {
                sprite.color = Color::srgb(0.32, 0.78, 0.46);
                sprite.custom_size = Some(Vec2::new(90.0, 112.0));
            }
            Some("jump") => {
                sprite.color = Color::srgb(0.97, 0.76, 0.25);
                sprite.custom_size = Some(Vec2::new(64.0, 132.0));
            }
            Some("fall") => {
                sprite.color = Color::srgb(0.98, 0.56, 0.28);
                sprite.custom_size = Some(Vec2::new(62.0, 142.0));
            }
            Some("wall_slide") => {
                sprite.color = Color::srgb(0.82, 0.34, 0.30);
                sprite.custom_size = Some(Vec2::new(72.0, 126.0));
            }
            Some("land") => {
                sprite.color = Color::srgb(0.73, 0.86, 0.95);
                sprite.custom_size = Some(Vec2::new(108.0, 92.0));
            }
            _ => {
                sprite.color = Color::srgb(0.28, 0.54, 0.90);
                sprite.custom_size = Some(Vec2::new(70.0, 120.0));
            }
        }
    }
}

fn update_hud(
    runtime: Single<
        (&CharacterStateMachineRuntime, &CharacterAnimationSelection),
        With<PlatformerSprite>,
    >,
    mut hud: Single<&mut Text, With<PlatformerHud>>,
    mut pane: ResMut<SpritePane>,
) {
    let (runtime, selection) = *runtime;
    let current = runtime
        .current_state
        .as_ref()
        .map(|state| state.0.as_str())
        .unwrap_or("none");
    hud.0 = format!(
        "Arrow keys: move, Space: jump\nstate: {current}\nstack depth: {}\nnormalized: {:.2}",
        runtime.state_stack.len(),
        runtime.normalized_time,
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.current_binding = selection
        .binding
        .as_ref()
        .map(|binding| binding.0.clone())
        .unwrap_or_else(|| "none".into());
}
