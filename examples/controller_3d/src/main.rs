//! Cross-crate example: saddle-character-controller + saddle-character-state-machine.
//!
//! This demonstrates the canonical 3D character pipeline:
//!   Keyboard → CharacterController (Avian3D physics) → bridge → StateMachine → AnimationBridge
//!
//! The bridge system (`sync_controller_facts`) converts controller output
//! into animation facts that drive state machine transitions.

#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use saddle_character_state_machine_example_support as support;

use avian3d::prelude::*;
use bevy::prelude::*;
use saddle_character_controller::{
    AccumulatedInput, CharacterController, CharacterControllerPlugin, CharacterControllerState,
    CharacterControllerSystems, CharacterMotionStats, MovementMode,
};
use saddle_character_state_machine::extensions::CharacterAnimationFactsExt;
use saddle_character_state_machine::*;
use saddle_pane::prelude::*;

#[derive(Component)]
pub struct DemoPlayer;

#[derive(Component)]
struct DemoHud;

#[derive(Resource, Pane)]
#[pane(title = "Controller 3D")]
struct ControllerPane {
    #[pane(tab = "Controller", slider, min = 4.0, max = 20.0, step = 0.5)]
    speed: f32,
    #[pane(tab = "Controller", slider, min = 0.8, max = 4.0, step = 0.1)]
    jump_height: f32,
    #[pane(tab = "Controller", slider, min = 10.0, max = 50.0, step = 1.0)]
    gravity: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    movement_mode: String,
    #[pane(tab = "Runtime", monitor)]
    horizontal_speed: String,
}

impl Default for ControllerPane {
    fn default() -> Self {
        Self {
            speed: 12.0,
            jump_height: 1.8,
            gravity: 29.0,
            current_state: "Idle".into(),
            movement_mode: "Grounded".into(),
            horizontal_speed: "0.00".into(),
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        support::pane_plugins(),
        // Avian3D physics (the controller runs in FixedUpdate).
        PhysicsPlugins::default(),
        // Character controller on FixedUpdate (standard for physics-driven controllers).
        CharacterControllerPlugin::always_on(FixedUpdate),
        // State machine on Update (animation doesn't need fixed timestep).
        CharacterStateMachinePlugin::always_on(Update),
    ));

    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::Controller3dLabE2EPlugin);

    app.init_resource::<ControllerPane>()
        .register_pane::<ControllerPane>()
        .init_resource::<InputBuffer>();

    // Bridge runs after controller finishes, before state machine evaluates.
    app.add_systems(
        Update,
        sync_controller_facts.before(CharacterStateMachineSystems::GatherFacts),
    );

    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            read_keyboard_input,
            sync_pane_to_controller,
            color_capsule_by_state,
            update_hud,
        ),
    );
    app.add_systems(
        FixedUpdate,
        apply_input_to_controller.before(CharacterControllerSystems::ReadInput),
    );

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
    // Camera.
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 6.0, 14.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));
    // Light.
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 24_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Ground — Avian3D static collider.
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.20, 0.18),
            perceptual_roughness: 1.0,
            ..default()
        })),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));

    // Register the state machine definition (same showcase as other examples).
    let definition_id = library
        .register(support::build_showcase_definition("controller_3d"))
        .unwrap();
    let bridge = support::build_animation_bridge(&mut animations, &mut graphs);

    // Spawn the character with BOTH the physics controller AND the state machine.
    let root = support::spawn_demo_character(
        &mut commands,
        &mut meshes,
        &mut materials,
        definition_id,
        bridge,
        Color::srgb(0.82, 0.47, 0.24),
        Vec3::new(0.0, 2.0, 0.0),
        "Controller 3D Character",
    );

    commands.entity(root).insert((
        DemoPlayer,
        // Physics controller (all required avian3d components auto-added).
        CharacterController {
            speed: 12.0,
            jump_height: 1.8,
            gravity: 29.0,
            ..default()
        },
    ));

    // HUD.
    commands.spawn((
        Name::new("Controller 3D HUD"),
        DemoHud,
        Text::new(
            "WASD: move | Space: jump | Shift: sprint | J: attack | R: reload | E: emote\nThis example uses saddle-character-controller (Avian3D physics) driving the state machine.",
        ),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(580.0),
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

// ---------------------------------------------------------------------------
// Input: keyboard → AccumulatedInput
// ---------------------------------------------------------------------------

/// Ephemeral resource to pass keyboard state from Update to FixedUpdate.
#[derive(Resource, Default)]
struct InputBuffer {
    move_axis: Vec2,
    jump_pressed: bool,
    jump_held: bool,
    sprint: bool,
}

fn read_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut buffer: ResMut<InputBuffer>,
    mut query: Query<&mut CharacterAnimationRequests, With<DemoPlayer>>,
) {
    let forward = keyboard.pressed(KeyCode::KeyW) as i8 - keyboard.pressed(KeyCode::KeyS) as i8;
    let right = keyboard.pressed(KeyCode::KeyD) as i8 - keyboard.pressed(KeyCode::KeyA) as i8;
    buffer.move_axis = Vec2::new(right as f32, forward as f32).clamp_length_max(1.0);
    // Latch jump_pressed on (never overwrite to false) — cleared after FixedUpdate consumes it.
    if keyboard.just_pressed(KeyCode::Space) {
        buffer.jump_pressed = true;
    }
    buffer.jump_held = keyboard.pressed(KeyCode::Space);
    buffer.sprint = keyboard.pressed(KeyCode::ShiftLeft);

    // Action requests are handled in Update (where the state machine runs).
    for mut requests in &mut query {
        if keyboard.just_pressed(KeyCode::KeyJ) {
            requests.push("attack");
        }
        if keyboard.just_pressed(KeyCode::KeyR) {
            requests.push("reload");
        }
        if keyboard.just_pressed(KeyCode::KeyE) {
            requests.push("emote");
        }
    }
}

fn apply_input_to_controller(
    mut buffer: ResMut<InputBuffer>,
    mut query: Query<&mut AccumulatedInput, With<DemoPlayer>>,
) {
    for mut input in &mut query {
        // Move axis: X = strafe, Y = forward/back. Controller uses this in its movement systems.
        input.move_axis = buffer.move_axis;
        if buffer.jump_pressed {
            input.press_jump();
        }
        input.jump_held = buffer.jump_held;
        input.sprint_active = buffer.sprint;
    }
    // Clear one-shot flags after consumption.
    buffer.jump_pressed = false;
}

// ---------------------------------------------------------------------------
// Bridge: controller output → animation facts
// ---------------------------------------------------------------------------

/// The core bridge: reads physics controller state and writes animation facts.
/// This is the system that connects the two crates.
fn sync_controller_facts(
    mut query: Query<
        (
            &CharacterController,
            &CharacterControllerState,
            &CharacterMotionStats,
            &LinearVelocity,
            &mut CharacterAnimationFacts,
        ),
        With<DemoPlayer>,
    >,
) {
    for (controller, state, stats, velocity, mut facts) in &mut query {
        // Speed: normalize horizontal speed to 0..~1 range using the controller's max speed.
        let max_speed = controller.speed * controller.sprint_speed_scale;
        let normalized_speed = (stats.horizontal_speed / max_speed).clamp(0.0, 1.2);
        facts.set_speed(normalized_speed);

        // Grounded: directly from movement mode.
        facts.set_grounded(matches!(state.movement_mode, MovementMode::Grounded));

        // Vertical velocity: from the physics velocity.
        facts.set_vertical_velocity(velocity.y);
    }
}

// ---------------------------------------------------------------------------
// Presentation
// ---------------------------------------------------------------------------

fn sync_pane_to_controller(
    pane: Res<ControllerPane>,
    mut query: Query<&mut CharacterController, With<DemoPlayer>>,
) {
    if !pane.is_changed() {
        return;
    }
    for mut controller in &mut query {
        controller.speed = pane.speed;
        controller.jump_height = pane.jump_height;
        controller.gravity = pane.gravity;
    }
}

fn color_capsule_by_state(
    machines: Query<(&CharacterStateMachineRuntime, &Children), With<DemoPlayer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    for (runtime, children) in &machines {
        let state_color = match runtime.current_state.as_ref().map(|s| s.0.as_str()) {
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
    query: Single<
        (
            &CharacterStateMachineRuntime,
            &CharacterAnimationSelection,
            &CharacterControllerState,
            &CharacterMotionStats,
        ),
        With<DemoPlayer>,
    >,
    mut text: Single<&mut Text, With<DemoHud>>,
    mut pane: ResMut<ControllerPane>,
) {
    let (runtime, selection, controller_state, stats) = *query;
    let current = runtime
        .current_state
        .as_ref()
        .map(|s| s.0.as_str())
        .unwrap_or("none");
    let stack = runtime
        .state_stack
        .iter()
        .map(|f| f.state.0.as_str())
        .collect::<Vec<_>>()
        .join(" -> ");
    let active = support::format_active_bindings(selection);
    let mode = format!("{:?}", controller_state.movement_mode);

    text.0 = format!(
        "WASD: move | Space: jump | Shift: sprint | J: attack | R: reload | E: emote\nstate: {current}  mode: {mode}\nstack: {stack}\nactive: {active}\nspeed: {:.2}  grounded_time: {:.2}",
        stats.horizontal_speed,
        stats.grounded_time,
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.movement_mode = mode;
    pane.horizontal_speed = format!("{:.2}", stats.horizontal_speed);
}
