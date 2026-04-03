use saddle_character_state_machine_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_character_state_machine::*;

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
    support::setup_basic_3d_scene(&mut commands, &mut meshes, &mut materials);
    let definition_id = library
        .register(support::build_showcase_definition("locomotion_3d"))
        .unwrap();
    let bridge = support::build_animation_bridge(&mut animations, &mut graphs);
    let entity = support::spawn_demo_character(
        &mut commands,
        &mut meshes,
        &mut materials,
        definition_id,
        bridge,
        Color::srgb(0.82, 0.47, 0.24),
        Vec3::ZERO,
        "Locomotion Demo Character",
    );
    commands.entity(entity).insert(DemoCharacter);

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
