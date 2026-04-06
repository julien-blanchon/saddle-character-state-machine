use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_character_state_machine::{
    CharacterAnimationFacts, CharacterAnimationRequests, CharacterAnimationSelection,
    CharacterStateMachineRuntime, extensions::CharacterAnimationFactsExt,
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

fn character_entity(world: &mut World) -> Entity {
    let mut query = world.query_filtered::<Entity, With<crate::LabCharacter>>();
    query
        .single(world)
        .expect("lab character should exist for e2e")
}

fn push_request(action: &'static str) -> Action {
    Action::Custom(Box::new(move |world| {
        let entity = character_entity(world);
        let mut entity_ref = world.entity_mut(entity);
        let mut requests = entity_ref
            .get_mut::<CharacterAnimationRequests>()
            .expect("lab character should expose request queue");
        requests.push(action);
    }))
}

fn trigger_airborne() -> Action {
    Action::Custom(Box::new(move |world| {
        let entity = character_entity(world);
        {
            let mut entity_ref = world.entity_mut(entity);
            let mut facts = entity_ref
                .get_mut::<CharacterAnimationFacts>()
                .expect("lab character should expose animation facts");
            facts.set_grounded(false);
            facts.set_vertical_velocity(4.8);
        }
        world.resource_mut::<crate::AirState>().airborne_time = 0.24;
    }))
}

fn state_machine_smoke() -> Scenario {
    Scenario::builder("state_machine_smoke")
        .description("Boot the crate-local state-machine lab, verify the hero initializes into a stable state with active bindings, and capture the baseline scene.")
        .then(Action::WaitFrames(20))
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
        .then(Action::Screenshot("state_machine_smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("state_machine_smoke"))
        .build()
}

fn state_machine_airborne() -> Scenario {
    Scenario::builder("state_machine_airborne")
        .description("Force the hero into an authored jump arc, verify the machine reaches airborne state, then wait for the landing cycle to return to idle.")
        .then(Action::WaitFrames(10))
        .then(trigger_airborne())
        .then(Action::WaitFrames(20))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero reaches airborne during the jump arc",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Airborne"),
        ))
        .then(Action::Screenshot("airborne_peak".into()))
        .then(Action::WaitFrames(80))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "hero returns to idle after landing",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Idle"),
        ))
        .then(Action::Screenshot("airborne_recovered".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("state_machine_airborne"))
        .build()
}

fn state_machine_actions() -> Scenario {
    Scenario::builder("state_machine_actions")
        .description("Exercise stacked action handling by entering reload, verifying attack is rejected while reload is active, and then triggering an emote once the stack clears.")
        .then(Action::WaitFrames(12))
        .then(push_request("reload"))
        .then(Action::WaitFrames(4))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "reload push becomes the active state",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Reload"),
        ))
        .then(push_request("attack"))
        .then(Action::WaitFrames(4))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "attack stays rejected while reload is non-interruptible",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Reload"),
        ))
        .then(Action::Screenshot("reload_rejects_attack".into()))
        .then(Action::WaitFrames(52))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "reload eventually pops back to idle",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Idle"),
        ))
        .then(push_request("emote"))
        .then(Action::WaitFrames(4))
        .then(assertions::component_where::<CharacterStateMachineRuntime, crate::LabCharacter>(
            "emote becomes the active transient state",
            |runtime| runtime.current_state.as_ref().is_some_and(|state| state.0 == "Emote"),
        ))
        .then(Action::Screenshot("emote_push".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("state_machine_actions"))
        .build()
}
