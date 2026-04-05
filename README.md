# Saddle Character State Machine

Reusable character animation state machine for Bevy.

The crate maps generic motion facts and action requests into logical animation states, pushdown interruptions, transition metadata, and optional Bevy `AnimationPlayer` playback. It is intentionally project-agnostic: no `Screen`, no `GameSet`, no gameplay assumptions about combat, AI, or a specific skeleton.

Use `CharacterStateMachinePlugin::always_on(Update)` for standalone examples and small tools. Use `CharacterStateMachinePlugin::new(...)` when you need explicit activate/deactivate schedules such as `OnEnter` / `OnExit`. Machines spawned after activation are initialized automatically on the next `GatherFacts` pass, so late-spawned characters do not need manual runtime setup.

## Why This Crate?

Bevy's built-in `AnimationGraph` handles low-level clip blending and skeletal masking, but it has no concept of game-level states, transitions, or pushdown stacks. On the other end, `bevy_animation_graph` provides a full visual editor with custom LERP nodes but brings significant complexity and editor coupling.

This crate fills the gap — comparable in scope to Unity's Animator Controller or a simplified Unreal Animation Blueprint — by providing:

- **Data-driven state graphs** defined in code (no external editor dependency)
- **Pushdown stack semantics** for temporary interrupts (attack, hit-react, emote) with clean resume
- **Guard-based transitions** with priority ranking, minimum durations, exit windows, and interrupt policies
- **1D blend trees** for locomotion blending within a single logical state
- **Concurrent animation layers** for upper-body overlays, additive recoil, etc.
- **Dual output surface**: logical bindings for 2D/sprite consumers, plus optional `BevyAnimationBridge` for 3D skeletal playback
- **DOT graph export** for state machine visualization and debugging

The design stays intentionally below a full statechart runtime. If your game needs hierarchical parallel regions or a visual node editor, consider `bevy_animation_graph`. If you need a practical, code-first state machine that handles locomotion, actions, and layers without fighting an editor workflow, this crate is the right fit.

## Quick Start

```toml
[dependencies]
saddle-character-state-machine = { git = "https://github.com/julien-blanchon/saddle-character-state-machine" }
bevy = "0.18"
```

```rust
use bevy::prelude::*;
use saddle_character_state_machine::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Gameplay,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<DemoState>()
        .add_plugins(CharacterStateMachinePlugin::new(
            OnEnter(DemoState::Gameplay),
            OnExit(DemoState::Gameplay),
            Update,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut library: ResMut<CharacterStateMachineLibrary>,
) {
    let definition_id = library
        .register(
            CharacterStateMachineDefinition::new("hero", "Idle")
                .with_fallback_state("Idle")
                .add_state(StateDefinition::new("Idle").with_binding("idle"))
                .add_state(StateDefinition::new("Run").with_binding("run"))
                .add_transition(
                    TransitionDefinition::switch("idle_to_run", "Idle", "Run")
                        .when(TransitionCondition::SpeedAtLeast(0.2)),
                )
                .add_transition(
                    TransitionDefinition::switch("run_to_idle", "Run", "Idle")
                        .when(TransitionCondition::SpeedAtMost(0.1)),
                ),
        )
        .unwrap();

    commands.spawn((
        Name::new("Hero"),
        CharacterStateMachine::new(definition_id),
        CharacterAnimationFacts {
            grounded: true,
            ..default()
        },
    ));
}
```

## Public API

| Type | Purpose |
| --- | --- |
| `CharacterStateMachinePlugin` | Runtime plugin with injectable activate, deactivate, and update schedules |
| `CharacterStateMachineSystems` | Public ordering hooks: `GatherFacts`, `ResolveTransitions`, `ApplyAnimation`, `Cleanup` |
| `CharacterStateMachineLibrary` | Shared definition registry for reusable machine graphs |
| `CharacterStateMachineDefinition` | Data-driven state graph: initial state, fallback state, states, transitions, and default blend |
| `CharacterStateMachine` | Per-entity machine component pointing at a registered definition |
| `CharacterAnimationFacts` | Generic inputs such as speed, grounded, vertical velocity, locomotion mode, wall contact, and inhibit flags |
| `CharacterAnimationRequests` | One-frame action queue used by transitions with `ActionRequested(...)` guards |
| `CharacterStateMachineRuntime` | BRP/debug-friendly runtime surface: current state, previous state, stack, current binding, pending request, state time, normalized time, and last transition trace |
| `CharacterAnimationSelection` | Logical output surface: selected state, dominant binding id, weighted active bindings, blend metadata, and sync hint |
| `BevyAnimationBridge` | Optional Bevy `AnimationPlayer` integration for `AnimationGraph` node playback |
| `BlendTree1D` / `BlendTreeParameter` | Per-state 1D binding blends driven by generic animation facts such as speed or locomotion intensity |
| `CharacterAnimationLayers` | Optional concurrent overlay bindings for upper-body aim, additive recoil, or other extra animation channels |
| Messages | `StateEntered`, `StateExited`, `StatePushed`, `StatePopped`, `TransitionRejected`, `AnimationBindingMissing` |

## Supported Patterns

- Locomotion selection from movement facts such as `speed`, `grounded`, and `vertical_velocity`
- 1D locomotion blend trees that smoothly mix multiple bindings inside a single logical state
- Temporary pushdown states like attack, reload, hit-react, or emote
- Concurrent animation layers driven by extra binding inputs instead of hardcoding everything into the base state graph
- Lightweight hierarchy through parent-state fallback chains
- Transition guards with explicit priorities, minimum durations, exit windows, and interrupt rules
- Logical binding output for 2D or sprite-driven consumers
- Optional Bevy `AnimationTransitions` playback for 3D / transform animations
- BRP-visible runtime diagnostics through reflected components

## Schedule Injection

The plugin does not assume a game state machine. Consumers decide when it activates:

```rust
app.add_plugins(CharacterStateMachinePlugin::new(
    OnEnter(MyState::Playing),
    OnExit(MyState::Playing),
    Update,
));
```

For always-on tools, sandboxes, or examples:

```rust
app.add_plugins(CharacterStateMachinePlugin::always_on(Update));
```

## Configuration Notes

- The runtime is data-driven but intentionally small. `StateDefinition` and `TransitionDefinition` cover common production needs without becoming a full editor/runtime graph system.
- `TransitionSource::State(parent_id)` plus `StateDefinition::with_parent(...)` gives lightweight hierarchy without a heavyweight statechart runtime.
- `TransitionDefinition::push(...)` plus `PushConflictPolicy` covers stacking, replace-top, and reject behavior for temporary states.
- `CharacterAnimationSelection` is always updated, even if you do not use `BevyAnimationBridge`. `binding` remains the dominant logical binding for backwards-compatible consumers, while `active_bindings` exposes the full weighted playback plan for blend trees and layers.
- Transition-specific blend overrides now flow through to `CharacterAnimationSelection`, and pop transitions restore preserved state time as a playback sync hint when the resumed state has duration data.
- `CharacterAnimationLayers` is an optional component so games can drive upper-body or additive overlays without duplicating the base machine definition. The authored `AnimationGraph` still decides masking and additive-vs-override behavior.

## Current Limitations / Non-goals

- No editor or authored graph UI
- No motion matching, IK, or root-motion extraction
- No built-in sprite animation runtime; 2D integrations adapt `CharacterAnimationSelection`
- Bevy playback crossfades are currently linear because `AnimationTransitions` only exposes duration-based fades. `BlendEasing` is preserved in config/output for future or custom adapters.
- Layer masking stays consumer-authored in the Bevy `AnimationGraph`; the crate drives concurrent bindings but does not author skeletal mask graphs for you.
- Hierarchy is a parent fallback chain, not a full orthogonal / parallel statechart runtime

## Examples

| Example | Description | Run |
| --- | --- | --- |
| `basic` | Minimal logical machine with state output and a single action push | `cargo run -p saddle-character-state-machine-example-basic` |
| `locomotion_3d` | Programmatic 3D clips driven through `BevyAnimationBridge` | `cargo run -p saddle-character-state-machine-example-locomotion-3d` |
| `sprite_2d` | 2D-friendly adapter using logical binding output instead of skeletal playback | `cargo run -p saddle-character-state-machine-example-sprite-2d` |
| `stacked_actions` | Rich showcase with locomotion, jump, reload rejection, attack push, emote replace-top, and HUD diagnostics | `cargo run -p saddle-character-state-machine-example-stacked-actions` |
| `graph_preview` | 2D visual state graph with live state highlighting, pane-driven fact inputs, and DOT export | `cargo run -p saddle-character-state-machine-example-graph-preview` |

Every standalone example now ships with a live `saddle-pane` panel so blend thresholds, locomotion values, jump timing, and layer weights can be tuned without recompiling.

## Workspace Lab

The workspace also includes a crate-local integration lab app at
`shared/character/saddle-character-state-machine/examples/lab`:

```bash
cargo run -p saddle-character-state-machine-lab
```

## More Docs

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
