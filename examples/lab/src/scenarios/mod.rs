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
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "state_machine_smoke" => Some(state_machine_smoke()),
        "state_machine_airborne" => Some(state_machine_airborne()),
        "state_machine_actions" => Some(state_machine_actions()),
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
