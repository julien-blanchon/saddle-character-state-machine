use saddle_character_state_machine_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_character_state_machine::*;

#[derive(Component)]
struct ShowcaseCharacter;

#[derive(Component)]
struct ShowcaseHud;

#[derive(Resource, Default)]
struct AirState {
    airborne_time: f32,
    aim_layer_enabled: bool,
}

#[derive(Resource, Pane)]
#[pane(title = "Stacked Actions")]
struct ShowcasePane {
    #[pane(tab = "Movement", slider, min = 0.0, max = 1.4, step = 0.05)]
    move_speed: f32,
    #[pane(tab = "Movement", slider, min = 0.5, max = 4.0, step = 0.1)]
    translation_speed: f32,
    #[pane(tab = "Movement", slider, min = 1.0, max = 8.0, step = 0.1)]
    jump_velocity: f32,
    #[pane(tab = "Movement", slider, min = 0.0, max = 0.6, step = 0.02)]
    jump_hold_seconds: f32,
    #[pane(tab = "Movement", slider, min = 4.0, max = 24.0, step = 0.5)]
    gravity: f32,
    #[pane(tab = "Layers")]
    aim_layer_enabled: bool,
    #[pane(tab = "Layers", slider, min = 0.0, max = 1.0, step = 0.05)]
    aim_layer_weight: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    active_bindings: String,
}

impl Default for ShowcasePane {
    fn default() -> Self {
        Self {
            move_speed: 1.0,
            translation_speed: 1.8,
            jump_velocity: 4.8,
            jump_hold_seconds: 0.24,
            gravity: 15.0,
            aim_layer_enabled: false,
            aim_layer_weight: 0.45,
            current_state: "Idle".into(),
            active_bindings: "idle:1.00".into(),
        }
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, support::pane_plugins()))
        .add_plugins(CharacterStateMachinePlugin::always_on(Update))
        .insert_resource(AirState::default())
        .init_resource::<ShowcasePane>()
        .register_pane::<ShowcasePane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (control_showcase, update_showcase_hud))
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
        .register(support::build_showcase_definition("stacked_actions"))
        .unwrap();
    let bridge = support::build_animation_bridge(&mut animations, &mut graphs);
    let entity = support::spawn_demo_character(
        &mut commands,
        &mut meshes,
        &mut materials,
        definition_id,
        bridge,
        Color::srgb(0.91, 0.54, 0.23),
        Vec3::ZERO,
        "Stacked Actions Character",
    );
    commands.entity(entity).insert((
        ShowcaseCharacter,
        CharacterAnimationLayers {
            layers: vec![
                CharacterAnimationLayer::new("UpperBodyAim", "aim_overlay")
                    .with_weight(0.45)
                    .sync_to_base_time(),
            ],
        },
    ));

    commands.spawn((
        ShowcaseHud,
        Text::new("W run | Space jump | J attack | R reload | E emote | F aim layer"),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(420.0),
            padding: UiRect::all(px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.04, 0.06, 0.78)),
        TextFont {
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn control_showcase(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut air_state: ResMut<AirState>,
    mut pane: ResMut<ShowcasePane>,
    mut query: Query<
        (
            &mut Transform,
            &mut CharacterAnimationFacts,
            &mut CharacterAnimationRequests,
            &mut CharacterAnimationLayers,
        ),
        With<ShowcaseCharacter>,
    >,
) {
    for (mut transform, mut facts, mut requests, mut layers) in &mut query {
        let moving = keyboard.pressed(KeyCode::KeyW);
        facts.speed = if moving { pane.move_speed } else { 0.0 };
        facts.locomotion_mode = if moving {
            LocomotionMode::Run
        } else {
            LocomotionMode::Idle
        };
        if moving {
            transform.translation.x =
                (transform.translation.x + pane.translation_speed * time.delta_secs())
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
        if keyboard.just_pressed(KeyCode::KeyF) {
            pane.aim_layer_enabled = !pane.aim_layer_enabled;
        }
        if keyboard.just_pressed(KeyCode::Space) && facts.grounded {
            facts.grounded = false;
            facts.vertical_velocity = pane.jump_velocity;
            air_state.airborne_time = pane.jump_hold_seconds;
        }

        air_state.aim_layer_enabled = pane.aim_layer_enabled;
        if let Some(layer) = layers.layers.first_mut() {
            layer.enabled = air_state.aim_layer_enabled;
            layer.weight = pane.aim_layer_weight;
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

fn update_showcase_hud(
    runtime: Single<
        (
            &CharacterStateMachineRuntime,
            &CharacterAnimationSelection,
            &CharacterAnimationLayers,
        ),
        With<ShowcaseCharacter>,
    >,
    mut hud: Single<&mut Text, With<ShowcaseHud>>,
    mut pane: ResMut<ShowcasePane>,
) {
    let (runtime, selection, layers) = *runtime;
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
    let queued = runtime
        .queued_requests
        .iter()
        .map(|request| request.0.as_str())
        .collect::<Vec<_>>()
        .join(", ");
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
    let reason = runtime
        .last_transition
        .as_ref()
        .and_then(|trace| trace.reason.as_ref())
        .map(|reason| format!("{reason:?}"))
        .unwrap_or_else(|| "none".to_string());
    let active = selection
        .active_bindings
        .iter()
        .map(|binding| format!("{}:{:.2}", binding.binding.0, binding.weight))
        .collect::<Vec<_>>()
        .join(", ");
    let layer_enabled = layers.layers.first().is_some_and(|layer| layer.enabled);

    hud.0 = format!(
        "W run | Space jump | J attack | R reload | E emote | F aim layer\ncurrent: {current}\nprevious: {previous}\nstack: {stack}\npending: {}\nqueued: {queued}\nlayer enabled: {layer_enabled}\nactive bindings: {active}\nstate time: {:.2}\nnormalized: {:.2}\nlast transition: {last}\nreason: {reason}",
        runtime
            .pending_request
            .as_ref()
            .map(|request| request.0.as_str())
            .unwrap_or("none"),
        runtime.state_elapsed_seconds,
        runtime.normalized_time,
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.active_bindings = active;
}
