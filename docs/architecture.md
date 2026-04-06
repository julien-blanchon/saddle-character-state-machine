# Architecture

`saddle-character-state-machine` is split into a small data-driven core plus ECS glue:

```text
CharacterAnimationFacts / CharacterAnimationRequests
                    |
                    v
        GatherFacts / playback sync
                    |
                    v
     Pure transition resolver in machine.rs
                    |
                    v
 CharacterStateMachineRuntime + Selection
                    |
                    v
 Weighted binding expansion + optional layers
                    |
                    v
 Optional BevyAnimationBridge adapter
```

## Core Model

The runtime revolves around five concepts:

1. `CharacterAnimationFacts`
   Input facts stored as named numeric, boolean, `Vec2`, and tag values plus optional playback timing fields.
2. `StateDefinition`
   A logical animation state with an id, optional parent state, binding id, duration hint, interrupt policy, and resume policy.
3. `TransitionDefinition`
   A directed edge with source, operation (`Set`, `Push`, `Pop`), guard, priority, exit window, blend override, and push conflict policy.
4. `CharacterStateMachineRuntime`
   Per-entity runtime state: current state, previous state, pushdown stack, current binding, pending request, state timer, normalized time, and last transition trace.
5. `CharacterAnimationSelection`
   The consumer-facing output: selected state, dominant logical binding id, weighted `active_bindings`, blend metadata, and optional sync hint.

## Resolution Flow

The public system sets run in this order:

```text
GatherFacts -> ResolveTransitions -> ApplyAnimation -> Cleanup
```

### GatherFacts

- Any entity with `CharacterStateMachine` but no runtime components is initialized here as a safety net for late spawns and same-schedule setup ordering.
- Optional Bevy playback sync reads `AnimationPlayer` and updates `CharacterAnimationFacts.clip_finished` / `animation_normalized_time`.
- External gameplay systems are expected to write most facts before this phase.

### ResolveTransitions

- The pure resolver advances the current top-of-stack timer.
- It computes a playback snapshot from explicit facts plus fallback duration hints.
- It ranks transitions by:
  1. higher priority
  2. more specific source (`Current` / exact parent chain before `Any`)
  3. declaration order
- The first valid transition wins.
- If nothing wins, the runtime still records the most relevant blocked reason for debugging.

### ApplyAnimation

- `CharacterAnimationSelection` is updated whenever the active state or logical binding changes.
- Transition-level blend overrides are copied onto `CharacterAnimationSelection`, and the runtime emits normalized-time sync hints for source-time sync and pop/resume cases.
- A follow-up expansion pass turns the logical state into weighted active bindings:
  - a plain state becomes one base binding
  - a `BlendTree1D` state becomes 1-2 weighted base bindings
  - optional `CharacterAnimationLayers` append concurrent overlay bindings
- If a `BevyAnimationBridge` exists, the runtime drives:
  - `AnimationTransitions::play(...)` for single-binding playback
  - direct weighted animation playback for blend-tree/layer cases
- If no bridge exists, the logical selection still updates for custom 2D / UI / sprite adapters.

### Cleanup

- `CharacterAnimationRequests` are cleared.
- The crate treats action requests as one-frame intents by default.

## Stack Semantics

The pushdown stack exists to handle temporary interrupts cleanly.

```text
Base movement state
    push Attack
        push HitReact
        pop -> Attack resumes
    pop -> Locomotion resumes
```

Rules:

- `Set` replaces the current top state.
- `Push` adds a temporary state on top of the current state.
- `Pop` removes the top state and resumes the previous frame.
- `PushConflictPolicy` controls what happens when a push occurs while another pushed state is already active:
  - `Stack`
  - `ReplaceTop`
  - `Reject`
- `ResumePolicy` controls whether a resumed state keeps its elapsed time or resets.

## Lightweight Hierarchy

Hierarchy is intentionally simple:

- A `StateDefinition` may name one parent state.
- `TransitionSource::State(parent_id)` lets parent transitions act as shared fallback logic.
- The runtime walks `current -> parent -> grandparent -> ...` before checking `Any`.

This keeps grounded/airborne shared rules possible without implementing a full orthogonal statechart runtime.

## Ergonomic Layers

The core runtime is intentionally generic. Ergonomics live one layer above it:

- `extensions` defines conventional fact keys plus helpers such as `conditions::speed_at_least(...)` and `parameters::movement_direction_x()`.
- `locomotion` is an optional recipe layer that maps `LocomotionMode` onto generic tags and keeps gait-specific semantics out of `CharacterAnimationFacts`.

## Animation Integration Strategy

There are two integration levels:

### Logical only

Consumers read `CharacterAnimationSelection` and map it to:

- sprite rows
- atlas tags
- UI poses
- custom material states
- game-specific animation systems

### Bevy `AnimationPlayer`

`BevyAnimationBridge` provides:

- `graph_handle`
- logical binding id -> graph node index mapping
- repeat mode
- optional duration hint for normalized time sync

The runtime deliberately keeps state selection separate from raw playback. This keeps the core reusable even when the consumer is not using Bevy skeletal animation.

When you need skeletal masking or additive behavior, author that in the `AnimationGraph` itself. The state-machine crate provides the weighted playback plan and concurrent binding activation; the graph determines how those clips combine on the rig.

## Failure Modes

The runtime tries to fail explicitly, not silently:

- missing definition
- missing state
- missing target state
- missing logical binding fallback
- missing Bevy bridge binding
- non-interruptible state blocking an external push/set
- minimum duration not met
- exit window closed
- empty stack on pop
- self-transition disallowed

Every blocked transition updates `CharacterStateMachineRuntime.last_transition`, and bridge-level missing bindings also emit `AnimationBindingMissing`.

## Extension Points

- Prefer adding helpers in `extensions` or an opt-in recipe module before expanding the core transition vocabulary.
- Keep custom gameplay rules outside the crate by deriving additional generic facts before `GatherFacts`.
- Swap the Bevy animation bridge for a sprite, shader, or material adapter by consuming `CharacterAnimationSelection`.
- Replace the linear Bevy transition adapter later without changing the logical state machine API.
