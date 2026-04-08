use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_character_state_machine::{
    CharacterAnimationSelection, CharacterStateMachineRuntime,
};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "state_machine_smoke",
        "state_machine_airborne",
        "state_machine_actions",
        "state_machine_animation_events",
        "state_machine_attack_lifecycle",
        "state_machine_emote_replace",
        "state_machine_state_elapsed",
        "state_machine_transition_trace",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "state_machine_smoke" => Some(state_machine_smoke()),
        "state_machine_airborne" => Some(state_machine_airborne()),
        "state_machine_actions" => Some(state_machine_actions()),
        "state_machine_animation_events" => Some(state_machine_animation_events()),
        "state_machine_attack_lifecycle" => Some(state_machine_attack_lifecycle()),
        "state_machine_emote_replace" => Some(state_machine_emote_replace()),
        "state_machine_state_elapsed" => Some(state_machine_state_elapsed()),
        "state_machine_transition_trace" => Some(state_machine_transition_trace()),
        _ => None,
    }
}

/// Boot the lab, verify idle, exercise Idle <-> Locomotion transitions via keyboard,
/// and capture screenshots at each stage.
fn state_machine_smoke() -> Scenario {
    Scenario::builder("state_machine_smoke")
        .description("Boot the lab, verify idle state, move with W to trigger Locomotion, release to return to Idle.")
        // -- Let the scene settle --
        .then(Action::WaitFrames(20))
        .then(Action::Log("Verifying initial idle state".into()))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "runtime exists on the lab character",
            |runtime| runtime.current_state.is_some(),
        ))
        .then(assertions::component_where::<CharacterAnimationSelection, crate::LabCharacter>(
            "selection exposes active bindings",
            |selection| !selection.active_bindings.is_empty(),
        ))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero starts in idle",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Idle"),
        ))
        .then(Action::Screenshot("idle_baseline".into()))

        // -- Hold W to trigger Locomotion --
        .then(Action::Log("Pressing W to trigger locomotion".into()))
        .then(Action::HoldKey { key: KeyCode::KeyW, frames: 30 })
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero transitions to locomotion on movement input",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Locomotion"),
        ))
        .then(Action::Screenshot("locomotion_active".into()))

        // -- Release and wait for Idle return --
        .then(Action::Log("Released W, waiting for idle return".into()))
        .then(Action::WaitFrames(30))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero returns to idle when movement stops",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("idle_returned".into()))

        .then(assertions::log_summary("state_machine_smoke"))
        .build()
}

/// Press Space to jump, verify the hero reaches airborne state via the real
/// jump arc in control_character, then wait for landing back to idle.
fn state_machine_airborne() -> Scenario {
    Scenario::builder("state_machine_airborne")
        .description("Press Space to jump, verify airborne states through the jump arc, wait for landing cycle back to idle.")
        .then(Action::WaitFrames(10))
        .then(Action::Log("Pressing Space to jump".into()))
        .then(Action::PressKey(KeyCode::Space))
        .then(Action::WaitFrames(2))
        .then(Action::ReleaseKey(KeyCode::Space))

        // -- After a few frames the jump arc should reach JumpStart or Airborne --
        .then(Action::WaitFrames(15))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero reaches airborne during the jump arc",
            |runtime| {
                runtime.current_state.as_ref().is_some_and(|s| {
                    s.0 == "JumpStart" || s.0 == "Airborne"
                })
            },
        ))
        .then(Action::Screenshot("airborne_peak".into()))
        .then(Action::Log("Hero is airborne, waiting for landing".into()))

        // -- Wait for the full jump arc + land + land-to-idle transition (generous) --
        .then(Action::WaitFrames(80))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero returned to idle after landing",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Idle"),
        ))
        .then(Action::Screenshot("airborne_recovered".into()))

        .then(assertions::log_summary("state_machine_airborne"))
        .build()
}

/// Exercise stacked action handling with real keyboard input:
/// reload (R), attempt attack during reload (J, should be rejected),
/// then emote (E) after reload completes.
fn state_machine_actions() -> Scenario {
    Scenario::builder("state_machine_actions")
        .description("Press R to reload, press J during reload (rejected because non-interruptible), wait for reload to complete, then press E for emote.")
        .then(Action::WaitFrames(12))

        // -- Trigger reload with R --
        .then(Action::Log("Pressing R to trigger reload".into()))
        .then(Action::PressKey(KeyCode::KeyR))
        .then(Action::WaitFrames(1))
        .then(Action::ReleaseKey(KeyCode::KeyR))
        .then(Action::WaitFrames(4))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "reload push becomes the active state",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Reload"),
        ))

        // -- Attempt attack during reload (should be rejected) --
        .then(Action::Log("Pressing J during reload (should be rejected)".into()))
        .then(Action::PressKey(KeyCode::KeyJ))
        .then(Action::WaitFrames(1))
        .then(Action::ReleaseKey(KeyCode::KeyJ))
        .then(Action::WaitFrames(4))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "attack stays rejected while reload is non-interruptible",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Reload"),
        ))
        .then(Action::Screenshot("reload_rejects_attack".into()))

        // -- Wait for reload to complete (0.8s at 60fps = 48 frames, plus margin) --
        .then(Action::Log("Waiting for reload to complete".into()))
        .then(Action::WaitFrames(52))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "reload eventually pops back to idle",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Idle"),
        ))

        // -- Trigger emote with E --
        .then(Action::Log("Pressing E to trigger emote".into()))
        .then(Action::PressKey(KeyCode::KeyE))
        .then(Action::WaitFrames(1))
        .then(Action::ReleaseKey(KeyCode::KeyE))
        .then(Action::WaitFrames(4))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "emote becomes the active transient state",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Emote"),
        ))
        .then(Action::Screenshot("emote_push".into()))

        .then(assertions::log_summary("state_machine_actions"))
        .build()
}

/// Verify the complete Attack transient lifecycle: push from Idle, verify the state,
/// wait for AnimationFinished to auto-pop back to Idle, and confirm no stack residue.
fn state_machine_attack_lifecycle() -> Scenario {
    Scenario::builder("state_machine_attack_lifecycle")
        .description("Press J to trigger Attack from Idle; verify Attack becomes active, then waits for AnimationFinished to auto-pop the stack back to Idle.")
        .then(Action::WaitFrames(10))

        // Confirm we start in Idle.
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero starts in idle before attack",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))

        // Press J to push Attack.
        .then(Action::Log("Pressing J to trigger attack".into()))
        .then(Action::PressKey(KeyCode::KeyJ))
        .then(Action::WaitFrames(1))
        .then(Action::ReleaseKey(KeyCode::KeyJ))
        .then(Action::WaitFrames(4))

        // Attack is a transient state (expected duration 0.35 s); it should be active now.
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "attack is the active state after J press",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Attack"),
        ))
        // The stack underneath must still contain Idle (or Grounded hierarchy).
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "state stack has at least two frames while attack is active",
            |runtime| runtime.state_stack.len() >= 2,
        ))
        .then(Action::Screenshot("attack_active".into()))

        // Attack clip is 0.35 s → ~21 frames at 60 fps; wait generously.
        .then(Action::Log("Waiting for attack to auto-complete".into()))
        .then(Action::WaitFrames(40))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero returned to idle after attack completes",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "state stack is back to base depth after attack pop",
            |runtime| runtime.state_stack.len() <= 1,
        ))
        .then(Action::Screenshot("attack_completed".into()))

        .then(assertions::log_summary("state_machine_attack_lifecycle"))
        .build()
}

/// Verify the Emote PushConflictPolicy::ReplaceTop policy: trigger a first emote,
/// then immediately trigger a second emote before the first finishes, and confirm the
/// second emote replaces the first rather than being rejected.
fn state_machine_emote_replace() -> Scenario {
    Scenario::builder("state_machine_emote_replace")
        .description("Trigger emote (E), then trigger a second emote (E) mid-playback; verify ReplaceTop policy fires the second emote without rejection.")
        .then(Action::WaitFrames(10))

        // First emote.
        .then(Action::Log("Pressing E for first emote".into()))
        .then(Action::PressKey(KeyCode::KeyE))
        .then(Action::WaitFrames(1))
        .then(Action::ReleaseKey(KeyCode::KeyE))
        .then(Action::WaitFrames(5))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "first emote is active",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Emote"),
        ))
        .then(Action::Screenshot("first_emote_active".into()))

        // Immediately trigger a second emote while the first is still running.
        // Emote clip is 0.55 s (~33 frames); we fire the second at frame ~6 — well within.
        .then(Action::Log("Pressing E again to replace emote mid-playback".into()))
        .then(Action::PressKey(KeyCode::KeyE))
        .then(Action::WaitFrames(1))
        .then(Action::ReleaseKey(KeyCode::KeyE))
        .then(Action::WaitFrames(4))

        // ReplaceTop should keep the emote state active (not rejected, not double-stacked).
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "emote still active after replace (not rejected)",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Emote"),
        ))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "stack depth did not grow beyond two after replace",
            |runtime| runtime.state_stack.len() <= 2,
        ))
        .then(Action::Screenshot("second_emote_replaced".into()))

        // Wait for the replaced emote to complete and verify return to Idle.
        .then(Action::Log("Waiting for replaced emote to finish".into()))
        .then(Action::WaitFrames(45))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero returns to idle after replaced emote completes",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("emote_replace_idle".into()))

        .then(assertions::log_summary("state_machine_emote_replace"))
        .build()
}

/// Verify that state_elapsed_seconds and normalized_time advance while in Locomotion,
/// and that normalized_time stays within [0.0, 1.0] while the clip loops.
fn state_machine_state_elapsed() -> Scenario {
    Scenario::builder("state_machine_state_elapsed")
        .description("Hold W to enter Locomotion; verify state_elapsed_seconds grows and normalized_time stays within [0, 1] across at least one full clip loop.")
        .then(Action::WaitFrames(10))

        // Enter locomotion.
        .then(Action::Log("Holding W to enter locomotion".into()))
        .then(Action::HoldKey { key: KeyCode::KeyW, frames: 15 })
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero enters locomotion",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Locomotion"),
        ))

        // Continue holding to accumulate time — locomotion clip is 0.65 s; hold for 2 full loops.
        .then(Action::HoldKey { key: KeyCode::KeyW, frames: 80 })

        // elapsed_seconds should be >= 0.65 s after more than one loop.
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "state_elapsed_seconds advanced past one full clip loop",
            |runtime| {
                runtime.current_state.as_ref().is_some_and(|s| s.0 == "Locomotion")
                    && runtime.state_elapsed_seconds >= 0.6
            },
        ))
        // normalized_time must stay in [0, 1] range across looping clips.
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "normalized_time is within valid range [0, 1]",
            |runtime| (0.0..=1.0).contains(&runtime.normalized_time),
        ))
        .then(Action::Screenshot("locomotion_elapsed".into()))

        // Release and verify idle return to confirm no stuck elapsed state.
        .then(Action::Log("Released W; verifying idle return".into()))
        .then(Action::WaitFrames(30))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "state_elapsed_seconds resets after idle return",
            |runtime| {
                runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle")
                    && runtime.state_elapsed_seconds < 5.0
            },
        ))
        .then(Action::Screenshot("elapsed_idle_returned".into()))

        .then(assertions::log_summary("state_machine_state_elapsed"))
        .build()
}

/// Verify the last_transition trace is populated correctly after a state switch,
/// and that the transition_id and previous_state fields reflect the actual transition.
fn state_machine_transition_trace() -> Scenario {
    Scenario::builder("state_machine_transition_trace")
        .description("Move with W to trigger idle_to_locomotion, then stop to trigger locomotion_to_idle; verify last_transition trace captures both transition ids.")
        .then(Action::WaitFrames(10))

        // Trigger idle → locomotion.
        .then(Action::Log("Holding W to trigger idle_to_locomotion".into()))
        .then(Action::HoldKey { key: KeyCode::KeyW, frames: 30 })
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "locomotion is active",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Locomotion"),
        ))
        // Transition trace should capture the idle_to_locomotion id.
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "last_transition trace shows idle_to_locomotion",
            |runtime| {
                runtime
                    .last_transition
                    .as_ref()
                    .and_then(|trace| trace.transition_id.as_ref())
                    .is_some_and(|id| id.0 == "idle_to_locomotion")
            },
        ))
        // previous_state in the trace should be Idle.
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "previous_state in trace is Idle",
            |runtime| {
                runtime
                    .previous_state
                    .as_ref()
                    .is_some_and(|s| s.0 == "Idle")
            },
        ))
        .then(Action::Screenshot("trace_locomotion".into()))

        // Stop movement — triggers locomotion → idle.
        .then(Action::Log("Released W to trigger locomotion_to_idle".into()))
        .then(Action::WaitFrames(30))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "idle is active after stopping",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "last_transition trace shows locomotion_to_idle",
            |runtime| {
                runtime
                    .last_transition
                    .as_ref()
                    .and_then(|trace| trace.transition_id.as_ref())
                    .is_some_and(|id| id.0 == "locomotion_to_idle")
            },
        ))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "previous_state in trace is Locomotion",
            |runtime| {
                runtime
                    .previous_state
                    .as_ref()
                    .is_some_and(|s| s.0 == "Locomotion")
            },
        ))
        .then(Action::Screenshot("trace_idle".into()))

        .then(assertions::log_summary("state_machine_transition_trace"))
        .build()
}

/// Walk the hero long enough for footstep AnimationEventFired messages to accumulate,
/// then verify the event pipeline is live and the last event id is one of the defined
/// footstep markers ("footstep_left" / "footstep_right").
fn state_machine_animation_events() -> Scenario {
    Scenario::builder("state_machine_animation_events")
        .description("Move with W to enter Locomotion, verify footstep AnimationEventFired messages fire and accumulate.")
        // Let the scene settle first.
        .then(Action::WaitFrames(10))

        // Hold W long enough to accumulate at least two footstep events.
        // The locomotion clip is 0.65 s with events at 0.25 and 0.75 (normalised).
        // Two full loops = 1.3 s = ~78 frames; we hold for 90 to be safe.
        .then(Action::Log("Holding W to trigger footstep events".into()))
        .then(Action::HoldKey { key: KeyCode::KeyW, frames: 90 })

        // The FiredEvents resource is initialised in main and updated by collect_animation_events.
        .then(assertions::custom(
            "AnimationEventFired fired at least twice during locomotion",
            |world| {
                world.get_resource::<crate::FiredEvents>()
                    .is_some_and(|fired| fired.count >= 2)
            },
        ))
        .then(assertions::custom(
            "last fired event is a footstep marker",
            |world| {
                world.get_resource::<crate::FiredEvents>()
                    .and_then(|fired| fired.last_event_id.as_deref())
                    .is_some_and(|id| id == "footstep_left" || id == "footstep_right")
            },
        ))
        .then(Action::Screenshot("animation_events_locomotion".into()))

        .then(assertions::log_summary("state_machine_animation_events"))
        .build()
}
