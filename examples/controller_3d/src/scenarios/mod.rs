use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_character_state_machine::CharacterStateMachineRuntime;

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "controller_3d_smoke",
        "controller_3d_jump",
        "controller_3d_actions",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "controller_3d_smoke" => Some(controller_3d_smoke()),
        "controller_3d_jump" => Some(controller_3d_jump()),
        "controller_3d_actions" => Some(controller_3d_actions()),
        _ => None,
    }
}

/// Boot, verify idle, walk with W, verify locomotion, release, verify idle return.
fn controller_3d_smoke() -> Scenario {
    Scenario::builder("controller_3d_smoke")
        .description("Verify CharacterController drives state machine: idle at rest, locomotion when moving, idle on stop.")
        .then(Action::WaitFrames(60))
        .then(Action::Log("Verifying idle state at rest".into()))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero starts in idle",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("controller_3d_idle".into()))

        // Walk forward with W.
        .then(Action::Log("Holding W to walk forward".into()))
        .then(Action::HoldKey { key: KeyCode::KeyW, frames: 60 })
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero transitions to locomotion while moving",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Locomotion"),
        ))
        .then(Action::Screenshot("controller_3d_locomotion".into()))

        // Release and wait for idle.
        .then(Action::Log("Released W, waiting for idle".into()))
        .then(Action::WaitFrames(60))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero returns to idle after stopping",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("controller_3d_idle_returned".into()))
        .then(assertions::log_summary("controller_3d_smoke"))
        .build()
}

/// Press Space to jump, verify airborne states, wait for landing.
fn controller_3d_jump() -> Scenario {
    Scenario::builder("controller_3d_jump")
        .description("Press Space to jump with physics controller, verify airborne state machine transition, verify landing.")
        .then(Action::WaitFrames(60))
        .then(Action::Log("Pressing Space to jump".into()))
        .then(Action::PressKey(KeyCode::Space))
        .then(Action::WaitFrames(2))
        .then(Action::ReleaseKey(KeyCode::Space))

        // Wait for jump to take effect through physics + state machine.
        .then(Action::WaitFrames(20))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero reaches airborne during jump arc",
            |runtime| {
                runtime.current_state.as_ref().is_some_and(|s| {
                    s.0 == "JumpStart" || s.0 == "Airborne"
                })
            },
        ))
        .then(Action::Screenshot("controller_3d_airborne".into()))

        // Wait for landing and return to idle.
        .then(Action::Log("Waiting for landing".into()))
        .then(Action::WaitFrames(120))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero returns to idle after landing",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("controller_3d_landed".into()))
        .then(assertions::log_summary("controller_3d_jump"))
        .build()
}

/// Press J for attack push, verify it pushes onto stack, verify it pops back.
fn controller_3d_actions() -> Scenario {
    Scenario::builder("controller_3d_actions")
        .description("Trigger attack action via J key, verify push/pop behavior on state machine stack.")
        .then(Action::WaitFrames(60))
        .then(Action::Log("Pressing J for attack".into()))
        .then(Action::PressKey(KeyCode::KeyJ))
        .then(Action::WaitFrames(1))
        .then(Action::ReleaseKey(KeyCode::KeyJ))
        .then(Action::WaitFrames(4))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "attack becomes the active state (pushed)",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Attack"),
        ))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "stack depth is 2 (idle underneath attack)",
            |runtime| runtime.state_stack.len() == 2,
        ))
        .then(Action::Screenshot("controller_3d_attack".into()))

        // Wait for attack to complete and pop.
        .then(Action::WaitFrames(40))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "attack pops back to idle",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("controller_3d_attack_popped".into()))
        .then(assertions::log_summary("controller_3d_actions"))
        .build()
}
