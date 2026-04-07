use saddle_character_state_machine_example_support as support;

use bevy::prelude::*;
use saddle_character_state_machine::extensions::CharacterAnimationFactsExt;
use saddle_character_state_machine::*;
use saddle_pane::prelude::*;

#[derive(Component)]
struct DemoCharacter;

#[derive(Component)]
struct DemoHud;

#[derive(Resource, Default)]
struct JumpState {
    upward_time: f32,
}

#[derive(Resource, Default)]
struct LastAnimationEvent {
    text: String,
    cooldown: f32,
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
    #[pane(tab = "Layers", slider, min = 0.0, max = 1.0, step = 0.05)]
    aim_layer_weight: f32,
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
            aim_layer_weight: 0.0,
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
        .insert_resource(LastAnimationEvent::default())
        .init_resource::<LocomotionPane>()
        .register_pane::<LocomotionPane>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                control_character,
                toggle_aim_layer,
                observe_animation_events,
                color_capsule_by_state,
                update_hud,
            ),
        )
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

    let root = support::spawn_demo_character(
        &mut commands,
        &mut meshes,
        &mut materials,
        definition_id,
        bridge,
        Color::srgb(0.82, 0.47, 0.24),
        Vec3::ZERO,
        "Locomotion Demo Character",
    );

    // Add the marker, aim layer, and animation requests to the character.
    commands.entity(root).insert((
        DemoCharacter,
        // Aim layer: concurrent overlay on top of base animation (F to toggle).
        CharacterAnimationLayers {
            layers: vec![CharacterAnimationLayer {
                name: "aim_overlay".into(),
                enabled: false,
                binding: CharacterAnimationBindingId("aim_overlay".into()),
                weight: 0.0,
                mode: CharacterAnimationLayerMode::Override,
                sync_to_base_time: true,
            }],
        },
    ));

    commands.spawn((
        Name::new("Locomotion HUD"),
        DemoHud,
        Text::new(
            "W: walk | Shift+W: run | Space: jump | J: attack | R: reload | E: emote | F: aim layer\nPane: tune movement, layers, and watch runtime state",
        ),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(520.0),
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
    mut query: Query<
        (
            &mut Transform,
            &mut CharacterAnimationFacts,
            &mut CharacterAnimationRequests,
        ),
        With<DemoCharacter>,
    >,
) {
    for (mut transform, mut facts, mut requests) in &mut query {
        let moving = keyboard.pressed(KeyCode::KeyW);
        let sprinting = keyboard.pressed(KeyCode::ShiftLeft);
        let speed = if moving {
            if sprinting {
                pane.run_speed
            } else {
                pane.walk_speed
            }
        } else {
            0.0
        };
        facts.set_speed(speed);
        if moving {
            transform.translation.x += time.delta_secs() * pane.translation_speed;
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

        if keyboard.just_pressed(KeyCode::Space) && facts.grounded() {
            facts.set_grounded(false);
            facts.set_vertical_velocity(pane.jump_velocity);
            jump_state.upward_time = pane.jump_hold_seconds;
        }

        if !facts.grounded() {
            if jump_state.upward_time > 0.0 {
                jump_state.upward_time -= time.delta_secs();
                facts.set_vertical_velocity(pane.jump_velocity);
            } else {
                let next =
                    facts.vertical_velocity() - pane.gravity * time.delta_secs();
                facts.set_vertical_velocity(next);
            }
            transform.translation.y += facts.vertical_velocity() * time.delta_secs();
            if transform.translation.y <= 0.0 {
                transform.translation.y = 0.0;
                facts.set_grounded(true);
                facts.set_vertical_velocity(-1.0);
            }
        } else {
            facts.set_vertical_velocity(0.0);
        }
    }
}

/// F key toggles the aim overlay layer. Pane slider controls its weight.
fn toggle_aim_layer(
    keyboard: Res<ButtonInput<KeyCode>>,
    pane: Res<LocomotionPane>,
    mut query: Query<&mut CharacterAnimationLayers, With<DemoCharacter>>,
) {
    for mut layers in &mut query {
        if let Some(aim) = layers.layers.first_mut() {
            if keyboard.just_pressed(KeyCode::KeyF) {
                aim.enabled = !aim.enabled;
            }
            aim.weight = pane.aim_layer_weight;
        }
    }
}

fn observe_animation_events(
    time: Res<Time>,
    mut last_event: ResMut<LastAnimationEvent>,
    mut events: MessageReader<AnimationEventFired>,
) {
    last_event.cooldown = (last_event.cooldown - time.delta_secs()).max(0.0);
    for event in events.read() {
        last_event.text = format!("{} @ {:.2}", event.event_id.0, event.normalized_time);
        last_event.cooldown = 0.5;
    }
    if last_event.cooldown <= 0.0 {
        last_event.text.clear();
    }
}

fn color_capsule_by_state(
    machines: Query<(&CharacterStateMachineRuntime, &Children), With<DemoCharacter>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    for (runtime, children) in &machines {
        let state_color = match runtime.current_state.as_ref().map(|state| state.0.as_str()) {
            Some("Idle") => Color::srgb(0.82, 0.47, 0.24),
            Some("Locomotion") => Color::srgb(0.32, 0.78, 0.46),
            Some("JumpStart") => Color::srgb(0.97, 0.76, 0.25),
            Some("Airborne") => Color::srgb(0.98, 0.56, 0.28),
            Some("Land") => Color::srgb(0.73, 0.86, 0.95),
            Some("Attack") => Color::srgb(0.90, 0.22, 0.22),
            Some("Reload") => Color::srgb(0.55, 0.33, 0.80),
            Some("Emote") => Color::srgb(0.95, 0.60, 0.85),
            _ => Color::srgb(0.82, 0.47, 0.24),
        };
        for child in children.iter() {
            if let Ok(mat_handle) = meshes.get(child)
                && let Some(material) = materials.get_mut(&mat_handle.0)
            {
                material.base_color = state_color;
            }
        }
    }
}

fn update_hud(
    runtime: Single<
        (
            &CharacterStateMachineRuntime,
            &CharacterAnimationSelection,
            &CharacterAnimationLayers,
        ),
        With<DemoCharacter>,
    >,
    mut text: Single<&mut Text, With<DemoHud>>,
    mut pane: ResMut<LocomotionPane>,
    last_event: Res<LastAnimationEvent>,
) {
    let (runtime, selection, layers) = *runtime;
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

    let active = support::format_active_bindings(selection);

    let aim = layers.layers.first();
    let aim_status = match aim {
        Some(layer) if layer.enabled => format!("ON ({:.0}%)", layer.weight * 100.0),
        _ => "OFF".into(),
    };

    let event_line = if last_event.text.is_empty() {
        String::new()
    } else {
        format!("\nevent: {}", last_event.text)
    };

    text.0 = format!(
        "W: walk | Shift+W: run | Space: jump | J: attack | R: reload | E: emote | F: aim layer\nstate: {current}  aim layer: {aim_status}\nstack: {stack}\nactive: {active}\nnormalized: {:.2}{event_line}",
        runtime.normalized_time
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.active_bindings = active;
}
