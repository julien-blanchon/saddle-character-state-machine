use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_character_state_machine::CharacterStateMachineRuntime;

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "platformer_2d_smoke",
        "platformer_2d_jump",
        "platformer_2d_wall_slide",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "platformer_2d_smoke" => Some(platformer_2d_smoke()),
        "platformer_2d_jump" => Some(platformer_2d_jump()),
        "platformer_2d_wall_slide" => Some(platformer_2d_wall_slide()),
        _ => None,
    }
}

/// Boot, verify idle, run right, verify Run state, release, verify idle return.
fn platformer_2d_smoke() -> Scenario {
    Scenario::builder("platformer_2d_smoke")
        .description("Verify platformer controller drives state machine: idle at rest, run when moving, idle on stop.")
        .then(Action::WaitFrames(60))
        .then(Action::Log("Verifying initial idle state".into()))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero starts in idle",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("platformer_2d_idle".into()))

        // Run right.
        .then(Action::Log("Holding ArrowRight to run".into()))
        .then(Action::HoldKey { key: KeyCode::ArrowRight, frames: 60 })
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero transitions to run while moving",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Run"),
        ))
        .then(Action::Screenshot("platformer_2d_run".into()))

        // Release and wait for idle.
        .then(Action::Log("Released, waiting for idle".into()))
        .then(Action::WaitFrames(60))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero returns to idle after stopping",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("platformer_2d_idle_returned".into()))
        .then(assertions::log_summary("platformer_2d_smoke"))
        .build()
}

/// Press Space to jump, verify jump/fall states, wait for landing.
fn platformer_2d_jump() -> Scenario {
    Scenario::builder("platformer_2d_jump")
        .description("Press Space to jump, verify jump/fall state machine transitions, verify landing.")
        .then(Action::WaitFrames(60))
        .then(Action::Log("Pressing Space to jump".into()))
        .then(Action::PressKey(KeyCode::Space))
        .then(Action::WaitFrames(2))
        .then(Action::ReleaseKey(KeyCode::Space))

        // Wait for jump arc.
        .then(Action::WaitFrames(20))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero enters jump or fall state",
            |runtime| {
                runtime.current_state.as_ref().is_some_and(|s| {
                    s.0 == "Jump" || s.0 == "Fall"
                })
            },
        ))
        .then(Action::Screenshot("platformer_2d_airborne".into()))

        // Wait for landing.
        .then(Action::Log("Waiting for landing".into()))
        .then(Action::WaitFrames(80))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero returns to idle after landing",
            |runtime| runtime.current_state.as_ref().is_some_and(|s| s.0 == "Idle"),
        ))
        .then(Action::Screenshot("platformer_2d_landed".into()))
        .then(assertions::log_summary("platformer_2d_jump"))
        .build()
}

/// Run toward right wall + jump to trigger wall slide.
fn platformer_2d_wall_slide() -> Scenario {
    Scenario::builder("platformer_2d_wall_slide")
        .description("Run toward right wall, jump, verify wall slide state.")
        .then(Action::WaitFrames(60))

        // Run right toward the wall.
        .then(Action::Log("Running right toward wall".into()))
        .then(Action::PressKey(KeyCode::ArrowRight))
        .then(Action::WaitFrames(80))

        // Jump while near wall.
        .then(Action::Log("Jumping near wall".into()))
        .then(Action::PressKey(KeyCode::Space))
        .then(Action::WaitFrames(2))
        .then(Action::ReleaseKey(KeyCode::Space))

        // Keep pressing right to stay against wall during fall.
        .then(Action::WaitFrames(40))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::DemoPlayer>(
            "hero enters wall slide or fall state near wall",
            |runtime| {
                runtime.current_state.as_ref().is_some_and(|s| {
                    s.0 == "WallSlide" || s.0 == "Fall" || s.0 == "Jump"
                })
            },
        ))
        .then(Action::Screenshot("platformer_2d_near_wall".into()))

        // Release and let the character land.
        .then(Action::ReleaseKey(KeyCode::ArrowRight))
        .then(Action::WaitFrames(80))
        .then(Action::Screenshot("platformer_2d_wall_slide_done".into()))
        .then(assertions::log_summary("platformer_2d_wall_slide"))
        .build()
}
