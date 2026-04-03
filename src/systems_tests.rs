use bevy::prelude::*;

use crate::bindings::{
    CharacterAnimationLayer, CharacterAnimationLayers, CharacterAnimationSelection,
};
use crate::components::{
    CharacterAnimationFacts, CharacterAnimationRequests, CharacterStateMachine,
    CharacterStateMachineRuntime,
};
use crate::config::{
    BlendTree1D, BlendTreeParameter, CharacterStateMachineDefinition, CharacterStateMachineLibrary,
    StateDefinition, TransitionCondition, TransitionDefinition,
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

#[test]
fn blend_tree_expands_to_weighted_base_bindings() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CharacterStateMachinePlugin::always_on(Update));

    let definition = CharacterStateMachineDefinition::new("blend_tree", "Locomotion")
        .with_fallback_state("Locomotion")
        .add_state(
            StateDefinition::new("Locomotion").with_blend_tree_1d(
                BlendTree1D::new(BlendTreeParameter::Speed)
                    .with_point(0.0, "walk")
                    .with_point(1.0, "run"),
            ),
        );
    app.world_mut()
        .resource_mut::<CharacterStateMachineLibrary>()
        .register(definition)
        .unwrap();

    let entity = app
        .world_mut()
        .spawn((
            CharacterStateMachine::new("blend_tree"),
            CharacterAnimationFacts {
                speed: 0.5,
                ..default()
            },
        ))
        .id();

    app.update();

    let selection = app
        .world()
        .entity(entity)
        .get::<CharacterAnimationSelection>()
        .unwrap();
    assert_eq!(selection.active_bindings.len(), 2);
    assert!(matches!(
        selection.binding.as_ref().map(|binding| binding.0.as_str()),
        Some("walk" | "run")
    ));
    assert_eq!(selection.active_bindings[0].binding.0, "walk");
    assert!((selection.active_bindings[0].weight - 0.5).abs() < 0.001);
    assert_eq!(selection.active_bindings[1].binding.0, "run");
    assert!((selection.active_bindings[1].weight - 0.5).abs() < 0.001);
}

#[test]
fn layers_are_appended_to_selection() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CharacterStateMachinePlugin::always_on(Update));
    insert_definition(app.world_mut());

    let entity = app
        .world_mut()
        .spawn((
            CharacterStateMachine::new("basic"),
            CharacterAnimationFacts::default(),
            CharacterAnimationLayers {
                layers: vec![
                    CharacterAnimationLayer::new("UpperBody", "aim")
                        .with_weight(0.65)
                        .sync_to_base_time(),
                ],
            },
        ))
        .id();

    app.update();

    let selection = app
        .world()
        .entity(entity)
        .get::<CharacterAnimationSelection>()
        .unwrap();
    assert_eq!(selection.active_bindings.len(), 2);
    assert_eq!(selection.active_bindings[0].binding.0, "idle");
    assert_eq!(selection.active_bindings[1].binding.0, "aim");
    assert!((selection.active_bindings[1].weight - 0.65).abs() < 0.001);
    assert!(selection.active_bindings[1].sync_normalized_time.is_some());
}
