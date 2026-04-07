//! Cross-crate example: saddle-character-platformer-controller + saddle-character-state-machine
//! + saddle-animation-spritesheet.
//!
//! This demonstrates the complete 2D character pipeline:
//!   Keyboard → PlatformerController (Avian2D physics) → bridge → StateMachine → Spritesheet
//!
//! Two bridge systems connect the crates:
//! 1. `sync_platformer_facts`: controller state → animation facts
//! 2. `sync_spritesheet`:      state machine selection → spritesheet clip target

#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use saddle_character_state_machine_example_support as support;

use avian2d::prelude::*;
use bevy::prelude::*;
use saddle_animation_spritesheet::{
    AnimationClip as SpriteClip, AnimationController, AnimationLibrary, AnimationState,
    AnimationTarget, SpritesheetAnimationBundle, SpritesheetPlugin,
};
use saddle_character_platformer_controller::{
    MovementConfig, PlatformerControllerBundle, PlatformerControllerConfig,
    PlatformerControllerPlugin, PlatformerControllerState, PlatformerJumpConfig,
    PlatformerMovementIntent,
};
use saddle_character_state_machine::extensions::{CharacterAnimationFactsExt, conditions};
use saddle_character_state_machine::*;
use saddle_pane::prelude::*;

#[derive(Component)]
pub struct DemoPlayer;

#[derive(Component)]
struct DemoHud;

#[derive(Resource, Pane)]
#[pane(title = "Platformer 2D")]
struct PlatformerPane {
    #[pane(tab = "Controller", slider, min = 80.0, max = 400.0, step = 10.0)]
    max_speed: f32,
    #[pane(tab = "Controller", slider, min = 40.0, max = 200.0, step = 5.0)]
    jump_height: f32,
    #[pane(tab = "Runtime", monitor)]
    current_state: String,
    #[pane(tab = "Runtime", monitor)]
    phase: String,
    #[pane(tab = "Runtime", monitor)]
    speed: String,
}

impl Default for PlatformerPane {
    fn default() -> Self {
        Self {
            max_speed: 240.0,
            jump_height: 88.0,
            current_state: "Idle".into(),
            phase: "Grounded".into(),
            speed: "0.00".into(),
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        support::pane_plugins(),
        PhysicsPlugins::default(),
        // Platformer controller on FixedUpdate (physics-driven).
        PlatformerControllerPlugin::always_on(FixedUpdate),
        // State machine on Update (animation selection).
        CharacterStateMachinePlugin::always_on(Update),
        // Spritesheet animation on Update (frame advance).
        SpritesheetPlugin::always_on(Update),
    ));

    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::Platformer2dLabE2EPlugin);

    app.init_resource::<PlatformerPane>()
        .register_pane::<PlatformerPane>();

    // Bridge 1: controller → facts (runs before state machine evaluates).
    app.add_systems(
        Update,
        sync_platformer_facts.before(CharacterStateMachineSystems::GatherFacts),
    );
    // Bridge 2: state machine → spritesheet (runs after state machine, before spritesheet).
    app.add_systems(
        Update,
        sync_spritesheet
            .after(CharacterStateMachineSystems::ApplyAnimation)
            .before(saddle_animation_spritesheet::SpritesheetSystems::ResolveRequests),
    );

    app.add_systems(Startup, setup);
    app.add_systems(Update, (drive_keyboard_intent, update_hud));

    app.run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut libraries: ResMut<Assets<AnimationLibrary>>,
    mut sm_library: ResMut<CharacterStateMachineLibrary>,
) {
    commands.spawn((Name::new("Camera"), Camera2d));

    // --- Build the procedural spritesheet (colored frames per state) ---
    let (image_handle, layout_handle) = build_procedural_spritesheet(&mut images, &mut layouts);
    let library_handle = libraries.add(build_sprite_library());

    // --- State machine definition ---
    let definition = CharacterStateMachineDefinition::new("platformer_2d", "Idle")
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
                .when(conditions::speed_at_least(0.2)),
        )
        .add_transition(
            TransitionDefinition::switch("run_to_idle", "Run", "Idle")
                .when(conditions::speed_at_most(0.05)),
        )
        .add_transition(
            TransitionDefinition::switch("leave_ground", "Grounded", "Jump")
                .when(conditions::grounded(false))
                .when(conditions::vertical_velocity_at_least(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("jump_to_fall", "Jump", "Fall")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::switch("fall_to_wall_slide", "Fall", "WallSlide")
                .when(conditions::wall_contact(true)),
        )
        .add_transition(
            TransitionDefinition::switch("wall_slide_to_fall", "WallSlide", "Fall")
                .when(conditions::wall_contact(false)),
        )
        .add_transition(
            TransitionDefinition::switch("fall_to_land", "Fall", "Land")
                .when(conditions::grounded(true))
                .when(conditions::vertical_velocity_at_most(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("land_to_idle", "Land", "Idle")
                .when(TransitionCondition::AnimationFinished),
        );
    let definition_id = sm_library.register(definition).unwrap();

    // --- Ground, walls, and a floating platform ---
    let wall_color = Color::srgb(0.14, 0.16, 0.14);
    // Floor.
    commands.spawn((
        Name::new("Floor"),
        Sprite::from_color(wall_color, Vec2::new(700.0, 26.0)),
        Transform::from_xyz(0.0, -180.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(700.0, 26.0),
    ));
    // Left wall.
    commands.spawn((
        Name::new("Left Wall"),
        Sprite::from_color(wall_color, Vec2::new(26.0, 400.0)),
        Transform::from_xyz(-350.0, 0.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(26.0, 400.0),
    ));
    // Right wall.
    commands.spawn((
        Name::new("Right Wall"),
        Sprite::from_color(wall_color, Vec2::new(26.0, 400.0)),
        Transform::from_xyz(350.0, 0.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(26.0, 400.0),
    ));
    // Floating platform.
    commands.spawn((
        Name::new("Platform"),
        Sprite::from_color(Color::srgb(0.22, 0.24, 0.22), Vec2::new(180.0, 16.0)),
        Transform::from_xyz(120.0, -60.0, 0.0),
        RigidBody::Static,
        Collider::rectangle(180.0, 16.0),
    ));

    // --- Spawn the player ---
    let config = PlatformerControllerConfig {
        movement: MovementConfig {
            max_speed: 240.0,
            ..default()
        },
        jump: PlatformerJumpConfig {
            height: 88.0,
            time_to_apex: 0.4,
            coyote_time: 0.11,
            jump_buffer_time: 0.12,
            max_air_jumps: 1,
            ..default()
        },
        ..default()
    };

    commands.spawn((
        Name::new("Platformer Player"),
        DemoPlayer,
        Sprite::from_atlas_image(
            image_handle,
            TextureAtlas {
                layout: layout_handle,
                index: 0,
            },
        ),
        // Physics controller.
        PlatformerControllerBundle::with_config(Collider::rectangle(28.0, 40.0), config)
            .with_transform(Transform::from_xyz(-100.0, -100.0, 10.0)),
        // State machine.
        CharacterStateMachine::new(definition_id),
        CharacterAnimationFacts::default(),
        // Spritesheet animation.
        SpritesheetAnimationBundle::new(library_handle, AnimationTarget::state("idle")),
    ));

    // --- HUD ---
    commands.spawn((
        Name::new("Platformer HUD"),
        DemoHud,
        Text::new(
            "Left/Right: move | Space: jump (air jump supported) | Wall contact = wall slide\nPlatformerController -> StateMachine -> Spritesheet",
        ),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(520.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.74)),
        TextFont {
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

// ---------------------------------------------------------------------------
// Input: keyboard → PlatformerMovementIntent
// ---------------------------------------------------------------------------

fn drive_keyboard_intent(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut PlatformerMovementIntent, With<DemoPlayer>>,
) {
    for mut intent in &mut query {
        let left = keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA);
        let right = keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD);
        intent.move_axis = right as i8 as f32 - left as i8 as f32;
        intent.jump_held = keyboard.pressed(KeyCode::Space);
        if keyboard.just_pressed(KeyCode::Space) {
            intent.jump_pressed = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Bridge 1: controller state → animation facts
// ---------------------------------------------------------------------------

fn sync_platformer_facts(
    mut query: Query<
        (&PlatformerControllerState, &mut CharacterAnimationFacts),
        With<DemoPlayer>,
    >,
    pane: Res<PlatformerPane>,
) {
    for (state, mut facts) in &mut query {
        let max_speed = pane.max_speed;
        facts.set_speed((state.velocity.x.abs() / max_speed).clamp(0.0, 1.2));
        facts.set_grounded(state.is_grounded);
        facts.set_vertical_velocity(state.velocity.y);
        facts.set_wall_contact(state.wall.is_some());
    }
}

// ---------------------------------------------------------------------------
// Bridge 2: state machine selection → spritesheet clip target
// ---------------------------------------------------------------------------

fn sync_spritesheet(
    mut query: Query<
        (&CharacterAnimationSelection, &mut AnimationController),
        (With<DemoPlayer>, Changed<CharacterAnimationSelection>),
    >,
) {
    for (selection, mut controller) in &mut query {
        if let Some(binding) = &selection.binding {
            controller.set_target(AnimationTarget::state(binding.0.as_str()));
        }
    }
}

// ---------------------------------------------------------------------------
// Presentation
// ---------------------------------------------------------------------------

fn update_hud(
    query: Single<
        (
            &CharacterStateMachineRuntime,
            &PlatformerControllerState,
        ),
        With<DemoPlayer>,
    >,
    mut text: Single<&mut Text, With<DemoHud>>,
    mut pane: ResMut<PlatformerPane>,
) {
    let (runtime, controller_state) = *query;
    let current = runtime
        .current_state
        .as_ref()
        .map(|s| s.0.as_str())
        .unwrap_or("none");
    let phase = format!("{:?}", controller_state.phase);
    let stack = runtime
        .state_stack
        .iter()
        .map(|f| f.state.0.as_str())
        .collect::<Vec<_>>()
        .join(" -> ");

    text.0 = format!(
        "Left/Right: move | Space: jump | Wall contact = wall slide\nstate: {current}  phase: {phase}\nstack: {stack}\nspeed: {:.0}  grounded: {}",
        controller_state.velocity.x.abs(),
        controller_state.is_grounded,
    );

    let pane = pane.bypass_change_detection();
    pane.current_state = current.into();
    pane.phase = phase;
    pane.speed = format!("{:.0}", controller_state.velocity.x.abs());
}

// ---------------------------------------------------------------------------
// Procedural spritesheet: colored frames per animation state
// ---------------------------------------------------------------------------

/// Builds a 6x4 procedural texture atlas where each row is a state with 4 frames.
/// Each state gets a distinct color, and frames slightly vary brightness for animation.
fn build_procedural_spritesheet(
    images: &mut Assets<Image>,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> (Handle<Image>, Handle<TextureAtlasLayout>) {
    let tile_w = 32u32;
    let tile_h = 40u32;
    let cols = 4u32;
    let rows = 6u32; // idle, run, jump, fall, wall_slide, land
    let img_w = tile_w * cols;
    let img_h = tile_h * rows;

    // State colors (one per row): idle=blue, run=green, jump=yellow, fall=orange, wall_slide=red, land=cyan
    let state_colors: [(f32, f32, f32); 6] = [
        (0.28, 0.54, 0.90), // idle
        (0.32, 0.78, 0.46), // run
        (0.97, 0.76, 0.25), // jump
        (0.98, 0.56, 0.28), // fall
        (0.82, 0.34, 0.30), // wall_slide
        (0.73, 0.86, 0.95), // land
    ];

    let mut data = vec![0u8; (img_w * img_h * 4) as usize];
    for row in 0..rows {
        let (r, g, b) = state_colors[row as usize];
        for col in 0..cols {
            // Slight brightness variation per frame for visual animation.
            let brightness = 0.85 + 0.15 * ((col as f32) / (cols as f32 - 1.0));
            let pr = (r * brightness * 255.0).min(255.0) as u8;
            let pg = (g * brightness * 255.0).min(255.0) as u8;
            let pb = (b * brightness * 255.0).min(255.0) as u8;

            for y in 0..tile_h {
                for x in 0..tile_w {
                    // Add a 2px border for clarity.
                    let is_border = x < 2 || x >= tile_w - 2 || y < 2 || y >= tile_h - 2;
                    let px = (col * tile_w + x) as usize;
                    let py = (row * tile_h + y) as usize;
                    let idx = (py * img_w as usize + px) * 4;
                    if is_border {
                        data[idx] = 20;
                        data[idx + 1] = 22;
                        data[idx + 2] = 24;
                    } else {
                        data[idx] = pr;
                        data[idx + 1] = pg;
                        data[idx + 2] = pb;
                    }
                    data[idx + 3] = 255;
                }
            }
        }
    }

    let image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: img_w,
            height: img_h,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::all(),
    );

    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(tile_w, tile_h),
        cols,
        rows,
        None,
        None,
    );

    (images.add(image), layouts.add(layout))
}

/// Builds the animation library mapping state machine bindings to spritesheet rows.
fn build_sprite_library() -> AnimationLibrary {
    // Each state maps to 4 frames in its row of the 6x4 atlas.
    AnimationLibrary::new("platformer_2d")
        .with_default_target(AnimationTarget::state("idle"))
        .add_clip(SpriteClip::from_indices("idle_clip", [0, 1, 2, 3]))
        .add_clip(SpriteClip::from_indices("run_clip", [4, 5, 6, 7]))
        .add_clip(SpriteClip::from_indices("jump_clip", [8, 9, 10, 11]))
        .add_clip(SpriteClip::from_indices("fall_clip", [12, 13, 14, 15]))
        .add_clip(SpriteClip::from_indices("wall_slide_clip", [16, 17, 18, 19]))
        .add_clip(SpriteClip::from_indices("land_clip", [20, 21, 22, 23]))
        // States: names match the state machine binding IDs.
        .add_state(AnimationState::new("idle", "idle_clip"))
        .add_state(AnimationState::new("run", "run_clip"))
        .add_state(AnimationState::new("jump", "jump_clip"))
        .add_state(AnimationState::new("fall", "fall_clip"))
        .add_state(AnimationState::new("wall_slide", "wall_slide_clip"))
        .add_state(AnimationState::new("land", "land_clip"))
}
