# Configuration

This crate is configured through `CharacterStateMachineDefinition`, `StateDefinition`, `TransitionDefinition`, and optional `BevyAnimationBridge`.

## `CharacterStateMachine`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `definition_id` | `CharacterStateMachineDefinitionId` | required | Selects the registered machine definition | Must exist in `CharacterStateMachineLibrary` |
| `enabled` | `bool` | `true` | Skips transition evaluation when `false` | Useful for pause / cutscene takeover |
| `time_scale` | `f32` | `1.0` | Scales state timers | `0.0` effectively pauses state-time-driven transitions |

## `CharacterAnimationFacts`

| Field | Type | Default | Effect | Common tuning advice |
| --- | --- | --- | --- | --- |
| `speed` | `f32` | `0.0` | Used by speed-based guards | Normalize to your movement model before writing facts |
| `movement_direction` | `Vec2` | `Vec2::ZERO` | Consumer-owned directional context | Current built-in guards do not inspect this directly |
| `locomotion_intensity` | `f32` | `0.0` | Optional gait intensity | Keep in `[0, 1]` if you expose it to custom systems |
| `locomotion_mode` | `LocomotionMode` | `Idle` | Used by `TransitionCondition::LocomotionMode(...)` | Prefer this over hardcoding speed thresholds when the gameplay layer already resolved gait |
| `grounded` | `bool` | `false` | Used by airborne / landing transitions | Set every frame |
| `wall_contact` | `bool` | `false` | Used by wall-slide style states | Set every frame |
| `vertical_velocity` | `f32` | `0.0` | Used for jump vs fall separation | Signed Y velocity is enough |
| `facing_direction` | `Vec2` | `Vec2::ZERO` | Consumer-owned facing context | Useful for custom adapters or future directional bindings |
| `aim_direction` | `Vec2` | `Vec2::ZERO` | Consumer-owned aim context | Not used by the built-in guards yet |
| `animation_normalized_time` | `f32` | `0.0` | Explicit normalized playback position | Write this when another animation system already knows exact clip time |
| `clip_finished` | `bool` | `false` | Drives `AnimationFinished` guards | The Bevy bridge can populate this automatically |
| `exit_window_open` | `bool` | `false` | Drives `ExitWindowOpen` guards | Useful when the consumer owns more complex clip events |
| `animation_locked` | `bool` | `false` | Used by `AnimationLocked(...)` guards | Keep separate from gameplay stun/freeze flags when possible |
| `inhibit_flags` | `Vec<String>` | `[]` | Generic gating flags | Use stable strings like `"stunned"` or `"frozen"` |

## `StateDefinition`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `id` | `CharacterStateId` | required | Stable logical state name | Prefer descriptive names like `Idle`, `Land`, `HitReact` |
| `parent` | `Option<CharacterStateId>` | `None` | Enables parent fallback transitions | Keep the tree shallow in v1 |
| `kind` | `StateKind` | `Persistent` | Controls derived completion behavior | `Transient` + `expected_duration_seconds` can drive auto-pop transitions |
| `binding` | `Option<CharacterAnimationBindingId>` | `None` | Logical animation binding output | If missing, the definition fallback binding is used when available |
| `blend_tree_1d` | `Option<BlendTree1D>` | `None` | Expands the state into weighted base bindings instead of a single clip | Use this for idle/walk/run style locomotion inside one logical state |
| `expected_duration_seconds` | `Option<f32>` | `None` | Duration hint for normalized time / completion fallback | Keep this aligned with real clip length when using exit windows or `AnimationFinished` |
| `minimum_duration_seconds` | `f32` | `0.0` | Global floor before any outgoing transition may leave this state | Use for landing or jump-start readability |
| `interruptible` | `bool` | `true` | Blocks external push/set transitions when `false` | Exact-state exit transitions can still run |
| `resume_policy` | `ResumePolicy` | `PreserveTime` | Behavior when a popped state resumes | `ResetTime` is useful for states that should restart when uncovered |

## `BlendTree1D`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `parameter` | `BlendTreeParameter` | required | Chooses which generic fact drives the 1D blend | `Speed` and `LocomotionIntensity` are the common locomotion choices |
| `points` | `Vec<BlendTree1DPoint>` | `[]` | Ordered sample points used to compute 1-2 active bindings | Thresholds are sorted during evaluation |

## `BlendTreeParameter`

Supported built-in inputs:

- `Speed`
- `LocomotionIntensity`
- `VerticalVelocity`
- `MovementDirectionX`
- `MovementDirectionY`
- `FacingDirectionX`
- `AimDirectionX`

## `BlendTree1DPoint`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `threshold` | `f32` | required | Sample point along the chosen parameter axis | Use ascending locomotion thresholds such as `0.0`, `0.5`, `1.0` |
| `binding` | `CharacterAnimationBindingId` | required | Binding activated at this point | Adjacent points are blended linearly |

## `TransitionDefinition`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `id` | `CharacterTransitionId` | required | Stable transition name for debugging | Shows up in `last_transition` and `TransitionRejected` |
| `source` | `TransitionSource` | required | Chooses where the edge applies | `Current`, `Any`, or `State(id)` |
| `target` | `Option<CharacterStateId>` | required for `Set` / `Push` | Destination state | `Pop` does not need a target |
| `operation` | `TransitionOperation` | required | `Set`, `Push`, or `Pop` | `Push` is the main temporary-interrupt tool |
| `guard` | `TransitionGuard` | empty | Condition groups | Empty guard means “always allowed” |
| `priority` | `i32` | `0` | Higher value wins before lower value | Use positive numbers for emergency or high-importance interrupts |
| `minimum_source_duration_seconds` | `Option<f32>` | `None` | Extra floor on top of the state minimum duration | Good for clip-specific exit readability |
| `exit_window` | `Option<NormalizedTimeWindow>` | `None` | Restricts the transition to a normalized-time window | Requires accurate timing facts or duration hints |
| `blend` | `Option<BlendDefinition>` | `None` | Overrides the definition default blend | The logical blend is preserved even if the current adapter is simpler |
| `allow_self_transition` | `bool` | `false` | Allows `target == current` | Use sparingly |
| `force_interrupt` | `bool` | `false` | Bypasses non-interruptible-state blocking | Reserve for truly global interrupts |
| `push_conflict_policy` | `PushConflictPolicy` | `Stack` | Behavior when another pushed state is already active | `Reject` is useful for non-layerable actions; `ReplaceTop` is useful for emotes or hit-react upgrades |

## `BlendDefinition`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `duration_seconds` | `f32` | `0.15` | Crossfade duration hint | `0.0` means hard cut |
| `easing` | `BlendEasing` | `Linear` | Desired curve shape | Bevy’s current built-in adapter still crossfades linearly |
| `reset_on_entry` | `bool` | `true` | Restart the destination animation/state on entry | Set `false` when resuming aligned loops matters more than strict replay |
| `sync_to_source_time` | `bool` | `false` | Preserve the source normalized time when possible | The Bevy adapter uses this as a `seek_to` hint when it has a duration hint for the active binding |

## `BevyAnimationBridge`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `player_entity` | `Option<Entity>` | `None` | Animation target entity | `None` means the machine entity itself owns the `AnimationPlayer` |
| `graph_handle` | `Handle<AnimationGraph>` | required | Graph inserted on the player entity when needed | The runtime does not author graphs for you |
| `bindings` | `Vec<BevyAnimationBinding>` | `[]` | Logical binding id -> graph node mapping | Each logical state binding should have at most one Bevy binding entry |

## `CharacterAnimationLayers`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `layers` | `Vec<CharacterAnimationLayer>` | `[]` | Optional concurrent overlay bindings appended after the base state selection | Useful for upper-body aim, recoil, breathing, or prop-hold layers |

## `CharacterAnimationLayer`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `name` | `String` | required | Debug-friendly layer label | Appears in `CharacterAnimationSelection.active_bindings` |
| `enabled` | `bool` | `true` | Enables or disables the layer | Disabled layers are ignored without removing the component entry |
| `binding` | `CharacterAnimationBindingId` | required | Binding played for the layer | Must exist in the `BevyAnimationBridge` if you use Bevy playback |
| `weight` | `f32` | `1.0` | Layer playback weight | Keep override layers in a sane range; additive behavior is still authored in the graph |
| `mode` | `CharacterAnimationLayerMode` | `Override` | Consumer-facing intent for the layer | The authored `AnimationGraph` still decides actual additive masking behavior |
| `sync_to_base_time` | `bool` | `false` | Seeks the layer using the base state's normalized time when possible | Good for overlays that should stay phase-aligned with locomotion |

## `BevyAnimationBinding`

| Field | Type | Default | Effect | Notes |
| --- | --- | --- | --- | --- |
| `binding_id` | `CharacterAnimationBindingId` | required | Must match the logical selection id | Keep names aligned with `StateDefinition::binding` |
| `node_index` | `u32` | required | `AnimationGraph` node to play | Use the node index returned when building the graph |
| `duration_seconds` | `Option<f32>` | `None` | Optional timing hint for normalized-time sync | Set this when you want `sync_to_source_time` or exact normalized debug output |
| `repeat` | `PlaybackRepeat` | `Forever` | Repeat mode assigned to the active animation | Use `Never` for one-shots like `Land` or `Attack` |

## Tuning Advice

- Use `StateDefinition::minimum_duration_seconds` for readability-driven rules that should apply to all exits from a state.
- Use `TransitionDefinition::minimum_source_duration_seconds` when only one transition should wait longer.
- Prefer `TransitionSource::State(parent_id)` over duplicate edges on many grounded child states.
- Keep logical binding ids stable and descriptive. They are the seam between machine logic and playback adapters.
- `ResumePolicy::PreserveTime` is most visible when the resumed state has `expected_duration_seconds`; that gives the runtime enough information to derive a normalized-time resume hint for playback adapters.
- If you rely on `AnimationFinished` or exit windows, provide either:
  - accurate `CharacterAnimationFacts` timing from another system, or
  - realistic `expected_duration_seconds` / `BevyAnimationBinding.duration_seconds` hints.
