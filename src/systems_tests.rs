use bevy::prelude::*;

use crate::bindings::CharacterAnimationSelection;
use crate::components::{
    CharacterAnimationFacts, CharacterAnimationRequests, CharacterStateMachine,
    CharacterStateMachineRuntime,
};
use crate::config::{
    CharacterStateMachineDefinition, CharacterStateMachineLibrary, StateDefinition,
    TransitionCondition, TransitionDefinition,
};
use crate::messages::{StateEntered, StatePopped, StatePushed};
use crate::{CharacterStateMachinePlugin, CharacterStateMachineSystems};

fn insert_definition(world: &mut World) {
    let definition = CharacterStateMachineDefinition::new("basic", "Idle")
        .with_fallback_state("Idle")
        .add_state(StateDefinition::new("Idle").with_binding("idle"))
        .add_state(
            StateDefinition::new("Attack")
                .transient()
                .with_binding("attack")
                .with_expected_duration(0.1),
        )
        .add_transition(
            TransitionDefinition::push("attack", crate::config::TransitionSource::Any, "Attack")
                .when(TransitionCondition::ActionRequested("attack".into())),
        )
        .add_transition(
            TransitionDefinition::pop("attack_done", "Attack")
                .when(TransitionCondition::AnimationFinished),
        );

    world
        .resource_mut::<CharacterStateMachineLibrary>()
        .register(definition)
        .unwrap();
}

#[test]
fn plugin_initializes_runtime_and_emits_initial_message() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CharacterStateMachinePlugin::always_on(Update));
    insert_definition(app.world_mut());
    let entity = app
        .world_mut()
        .spawn(CharacterStateMachine::new("basic"))
        .id();

    app.update();

    assert!(
        app.world()
            .entity(entity)
            .contains::<CharacterStateMachineRuntime>()
    );
    assert!(
        app.world()
            .entity(entity)
            .contains::<CharacterAnimationSelection>()
    );

    let messages = app.world().resource::<Messages<StateEntered>>();
    assert_eq!(messages.len(), 1);
}

#[test]
fn machine_pushes_and_pops_with_messages() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CharacterStateMachinePlugin::always_on(Update));
    insert_definition(app.world_mut());

    let entity = app
        .world_mut()
        .spawn((
            CharacterStateMachine::new("basic"),
            CharacterAnimationFacts::default(),
            CharacterAnimationRequests::default(),
        ))
        .id();

    app.update();

    app.world_mut()
        .entity_mut(entity)
        .get_mut::<CharacterAnimationRequests>()
        .unwrap()
        .push("attack");
    app.update();

    let runtime = app
        .world()
        .entity(entity)
        .get::<CharacterStateMachineRuntime>()
        .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Attack");
    assert_eq!(app.world().resource::<Messages<StatePushed>>().len(), 1);

    {
        let mut entity_ref = app.world_mut().entity_mut(entity);
        let mut facts = entity_ref.get_mut::<CharacterAnimationFacts>().unwrap();
        facts.clip_finished = true;
    }
    app.update();

    let runtime = app
        .world()
        .entity(entity)
        .get::<CharacterStateMachineRuntime>()
        .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Idle");
    assert_eq!(app.world().resource::<Messages<StatePopped>>().len(), 1);
}

#[test]
fn public_system_sets_can_be_configured() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CharacterStateMachinePlugin::always_on(Update));
    app.configure_sets(
        Update,
        CharacterStateMachineSystems::GatherFacts
            .before(CharacterStateMachineSystems::ResolveTransitions),
    );
    app.update();
}

#[test]
fn late_spawned_machine_initializes_on_next_update() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CharacterStateMachinePlugin::always_on(Update));
    insert_definition(app.world_mut());

    app.update();

    let entity = app
        .world_mut()
        .spawn((
            CharacterStateMachine::new("basic"),
            CharacterAnimationFacts::default(),
        ))
        .id();

    app.update();

    assert!(
        app.world()
            .entity(entity)
            .contains::<CharacterStateMachineRuntime>()
    );
    assert!(
        app.world()
            .entity(entity)
            .contains::<CharacterAnimationSelection>()
    );

    let messages = app.world().resource::<Messages<StateEntered>>();
    assert_eq!(messages.len(), 1);
}
