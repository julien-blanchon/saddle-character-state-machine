use saddle_character_state_machine_example_support as support;

use bevy::prelude::*;
use saddle_character_state_machine::*;

#[derive(Component)]
struct DemoCharacter;

#[derive(Component)]
struct DemoHud;

#[derive(Resource, Default)]
struct JumpState {
    upward_time: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CharacterStateMachinePlugin::always_on(Update))
        .insert_resource(JumpState::default())
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
        Text::new("W: run | Space: jump"),
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
    mut query: Query<(&mut Transform, &mut CharacterAnimationFacts), With<DemoCharacter>>,
) {
    for (mut transform, mut facts) in &mut query {
        let moving = keyboard.pressed(KeyCode::KeyW);
        facts.speed = if moving { 1.0 } else { 0.0 };
        facts.locomotion_mode = if moving {
            LocomotionMode::Run
        } else {
            LocomotionMode::Idle
        };
        if moving {
            transform.translation.x += time.delta_secs() * 1.8;
        }

        if keyboard.just_pressed(KeyCode::Space) && facts.grounded {
            facts.grounded = false;
            facts.vertical_velocity = 4.6;
            jump_state.upward_time = 0.24;
        }

        if !facts.grounded {
            if jump_state.upward_time > 0.0 {
                jump_state.upward_time -= time.delta_secs();
                facts.vertical_velocity = 4.6;
            } else {
                facts.vertical_velocity -= 16.0 * time.delta_secs();
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
    runtime: Single<&CharacterStateMachineRuntime, With<DemoCharacter>>,
    mut text: Single<&mut Text, With<DemoHud>>,
) {
    let runtime = *runtime;
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

    text.0 = format!(
        "W: run | Space: jump\nstate: {current}\nstack: {stack}\nnormalized: {:.2}",
        runtime.normalized_time
    );
}
