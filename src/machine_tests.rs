use crate::bindings::CharacterAnimationSelection;
use crate::components::{
    CharacterAnimationFacts, CharacterAnimationRequests, CharacterStateMachineRuntime,
    LocomotionMode, TransitionRejectionReason,
};
use crate::config::{
    BlendDefinition, BlendEasing, CharacterStateMachineDefinition, PushConflictPolicy,
    ResumePolicy, StateDefinition, TransitionCondition, TransitionDefinition, TransitionSource,
};
use crate::machine::{advance_machine, initialize_machine};

fn make_definition() -> CharacterStateMachineDefinition {
    CharacterStateMachineDefinition::new("humanoid", "Idle")
        .with_default_blend(BlendDefinition::new(0.12))
        .with_fallback_state("Idle")
        .add_state(
            StateDefinition::new("Grounded")
                .with_binding("idle")
                .with_resume_policy(ResumePolicy::PreserveTime),
        )
        .add_state(
            StateDefinition::new("Idle")
                .with_parent("Grounded")
                .with_binding("idle"),
        )
        .add_state(
            StateDefinition::new("Locomotion")
                .with_parent("Grounded")
                .with_binding("run")
                .with_expected_duration(0.8),
        )
        .add_state(
            StateDefinition::new("Attack")
                .transient()
                .with_binding("attack")
                .with_expected_duration(0.35),
        )
        .add_state(
            StateDefinition::new("Reload")
                .transient()
                .with_binding("reload")
                .with_expected_duration(0.6)
                .non_interruptible(),
        )
        .add_state(
            StateDefinition::new("JumpStart")
                .transient()
                .with_binding("jump")
                .with_expected_duration(0.2),
        )
        .add_state(
            StateDefinition::new("Airborne")
                .with_binding("fall")
                .with_expected_duration(0.7),
        )
        .add_state(
            StateDefinition::new("Land")
                .transient()
                .with_binding("land")
                .with_expected_duration(0.15),
        )
        .add_transition(
            TransitionDefinition::switch("idle_to_locomotion", "Idle", "Locomotion")
                .when(TransitionCondition::SpeedAtLeast(0.2)),
        )
        .add_transition(
            TransitionDefinition::switch("locomotion_to_idle", "Locomotion", "Idle")
                .when(TransitionCondition::SpeedAtMost(0.05)),
        )
        .add_transition(
            TransitionDefinition::push("attack_push", TransitionSource::Any, "Attack")
                .when(TransitionCondition::ActionRequested("attack".into())),
        )
        .add_transition(
            TransitionDefinition::push("reload_push", TransitionSource::Any, "Reload")
                .when(TransitionCondition::ActionRequested("reload".into()))
                .with_push_conflict_policy(PushConflictPolicy::Reject),
        )
        .add_transition(
            TransitionDefinition::pop("attack_complete", "Attack")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::pop("reload_complete", "Reload")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::switch("leave_ground", "Grounded", "JumpStart")
                .when(TransitionCondition::Grounded(false))
                .when(TransitionCondition::VerticalVelocityAtLeast(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("jump_to_air", "JumpStart", "Airborne")
                .when(TransitionCondition::AnimationFinished),
        )
        .add_transition(
            TransitionDefinition::switch("air_to_land", "Airborne", "Land")
                .when(TransitionCondition::Grounded(true))
                .when(TransitionCondition::VerticalVelocityAtMost(0.0)),
        )
        .add_transition(
            TransitionDefinition::switch("land_to_idle", "Land", "Idle")
                .when(TransitionCondition::AnimationFinished),
        )
}

#[test]
fn pushed_attack_pops_back_to_previous_state() {
    let definition = make_definition();
    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    initialize_machine(&definition, &mut runtime, &mut selection).unwrap();

    let mut facts = CharacterAnimationFacts {
        speed: 1.0,
        locomotion_mode: LocomotionMode::Run,
        grounded: true,
        ..Default::default()
    };
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Locomotion");

    let mut requests = CharacterAnimationRequests::default();
    requests.push("attack");
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        Some(&requests),
        0.016,
        &mut selection,
    )
    .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Attack");
    assert_eq!(runtime.state_stack.len(), 2);

    facts.clip_finished = true;
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();

    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Locomotion");
    assert_eq!(runtime.state_stack.len(), 1);
}

#[test]
fn non_interruptible_reload_rejects_attack_push() {
    let definition = make_definition();
    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    initialize_machine(&definition, &mut runtime, &mut selection).unwrap();

    let facts = CharacterAnimationFacts {
        grounded: true,
        ..Default::default()
    };
    let mut requests = CharacterAnimationRequests::default();
    requests.push("reload");
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        Some(&requests),
        0.016,
        &mut selection,
    )
    .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Reload");

    let mut attack_requests = CharacterAnimationRequests::default();
    attack_requests.push("attack");
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        Some(&attack_requests),
        0.016,
        &mut selection,
    )
    .unwrap();

    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Reload");
    let reason = runtime
        .last_transition
        .as_ref()
        .and_then(|trace| trace.reason.as_ref())
        .cloned();
    assert!(
        matches!(reason, Some(TransitionRejectionReason::StateLocked(_))),
        "unexpected reason: {reason:?}"
    );
}

#[test]
fn airborne_landing_transition_requires_grounded_descent() {
    let definition = make_definition();
    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    initialize_machine(&definition, &mut runtime, &mut selection).unwrap();
    runtime.current_state = Some("Airborne".into());
    runtime.state_stack = vec![crate::components::ActiveStateFrame::new("Airborne")];

    let mut facts = CharacterAnimationFacts {
        grounded: false,
        vertical_velocity: -4.0,
        ..Default::default()
    };
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Airborne");

    facts.grounded = true;
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Land");
}

#[test]
fn fallback_binding_is_used_when_state_has_no_binding() {
    let definition = CharacterStateMachineDefinition::new("fallback", "Idle")
        .with_fallback_state("Idle")
        .add_state(StateDefinition::new("Idle").with_binding("idle"))
        .add_state(StateDefinition::new("Gesture").transient())
        .add_transition(
            TransitionDefinition::push("gesture_push", TransitionSource::Any, "Gesture")
                .when(TransitionCondition::ActionRequested("wave".into())),
        );
    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    initialize_machine(&definition, &mut runtime, &mut selection).unwrap();

    let mut requests = CharacterAnimationRequests::default();
    requests.push("wave");
    advance_machine(
        &definition,
        &mut runtime,
        &CharacterAnimationFacts::default(),
        Some(&requests),
        0.016,
        &mut selection,
    )
    .unwrap();

    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Gesture");
    assert_eq!(selection.binding.as_ref().unwrap().0, "idle");
}

#[test]
fn transition_blend_override_and_sync_time_are_applied_to_selection() {
    let definition = CharacterStateMachineDefinition::new("blend", "Idle")
        .add_state(
            StateDefinition::new("Idle")
                .with_binding("idle")
                .with_expected_duration(1.0),
        )
        .add_state(
            StateDefinition::new("Locomotion")
                .with_binding("run")
                .with_expected_duration(1.0),
        )
        .add_transition(
            TransitionDefinition::switch("idle_to_locomotion", "Idle", "Locomotion")
                .when(TransitionCondition::SpeedAtLeast(0.2))
                .with_blend(
                    BlendDefinition::new(0.35)
                        .with_easing(BlendEasing::SineInOut)
                        .sync_to_source_time(),
                ),
        );

    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    initialize_machine(&definition, &mut runtime, &mut selection).unwrap();

    let facts = CharacterAnimationFacts {
        speed: 1.0,
        animation_normalized_time: 0.42,
        ..Default::default()
    };

    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();

    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Locomotion");
    assert_eq!(runtime.current_binding.as_ref().unwrap().0, "run");
    assert_eq!(selection.blend.duration_seconds, 0.35);
    assert_eq!(selection.blend.easing, BlendEasing::SineInOut);
    assert_eq!(selection.sync_normalized_time, Some(0.42));
}

#[test]
fn pop_resumes_previous_state_time_for_playback_sync() {
    let definition = CharacterStateMachineDefinition::new("resume", "Idle")
        .add_state(
            StateDefinition::new("Idle")
                .with_binding("idle")
                .with_expected_duration(1.0)
                .with_resume_policy(ResumePolicy::PreserveTime),
        )
        .add_state(
            StateDefinition::new("Attack")
                .transient()
                .with_binding("attack")
                .with_expected_duration(0.2),
        )
        .add_transition(
            TransitionDefinition::push("attack_push", TransitionSource::Any, "Attack")
                .when(TransitionCondition::ActionRequested("attack".into())),
        )
        .add_transition(
            TransitionDefinition::pop("attack_done", "Attack")
                .when(TransitionCondition::AnimationFinished),
        );

    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    initialize_machine(&definition, &mut runtime, &mut selection).unwrap();

    runtime.state_elapsed_seconds = 0.5;
    runtime.state_stack[0].elapsed_seconds = 0.5;
    runtime.normalized_time = 0.5;

    let mut requests = CharacterAnimationRequests::default();
    requests.push("attack");
    advance_machine(
        &definition,
        &mut runtime,
        &CharacterAnimationFacts::default(),
        Some(&requests),
        0.016,
        &mut selection,
    )
    .unwrap();

    let facts = CharacterAnimationFacts {
        clip_finished: true,
        ..Default::default()
    };
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();

    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Idle");
    let resumed_time = selection.sync_normalized_time.unwrap();
    assert!(
        (resumed_time - 0.516).abs() < 0.000_1,
        "unexpected resumed time: {resumed_time}"
    );
}

#[test]
fn state_ids_returns_all_defined_states() {
    let definition = make_definition();
    let ids: Vec<&str> = definition
        .state_ids()
        .iter()
        .map(|id| id.0.as_str())
        .collect();
    assert!(ids.contains(&"Idle"));
    assert!(ids.contains(&"Locomotion"));
    assert!(ids.contains(&"Attack"));
    assert!(ids.contains(&"Reload"));
    assert!(ids.contains(&"JumpStart"));
    assert!(ids.contains(&"Airborne"));
    assert!(ids.contains(&"Land"));
    assert!(ids.contains(&"Grounded"));
    assert_eq!(ids.len(), 8);
}

#[test]
fn transition_ids_returns_all_defined_transitions() {
    let definition = make_definition();
    let ids: Vec<&str> = definition
        .transition_ids()
        .iter()
        .map(|id| id.0.as_str())
        .collect();
    assert!(ids.contains(&"idle_to_locomotion"));
    assert!(ids.contains(&"attack_push"));
    assert!(ids.contains(&"attack_complete"));
    assert_eq!(ids.len(), 10);
}

#[test]
fn dot_graph_contains_states_and_transitions() {
    let definition = make_definition();
    let dot = definition.dot_graph();
    assert!(dot.contains("digraph \"humanoid\""));
    assert!(dot.contains("\"Idle\""));
    assert!(dot.contains("\"Attack\""));
    assert!(dot.contains("idle_to_locomotion"));
    assert!(dot.contains("__start"));
}

#[test]
fn validation_error_display_is_human_readable() {
    use crate::config::CharacterStateMachineValidationError;
    let error = CharacterStateMachineValidationError::NoStates;
    assert_eq!(error.to_string(), "definition has no states");

    let error = CharacterStateMachineValidationError::DuplicateState("Idle".into());
    assert_eq!(error.to_string(), "duplicate state 'Idle'");

    let error = CharacterStateMachineValidationError::MissingInitialState("Run".into());
    assert_eq!(error.to_string(), "missing initial state 'Run'");
}

#[test]
fn custom_flag_condition_evaluates_correctly() {
    let definition = CharacterStateMachineDefinition::new("custom_flags", "Idle")
        .add_state(StateDefinition::new("Idle").with_binding("idle"))
        .add_state(StateDefinition::new("Special").with_binding("special"))
        .add_transition(
            TransitionDefinition::switch("idle_to_special", "Idle", "Special")
                .when(TransitionCondition::CustomFlag("powered_up".into())),
        )
        .add_transition(
            TransitionDefinition::switch("special_to_idle", "Special", "Idle")
                .when(TransitionCondition::CustomFlagMissing("powered_up".into())),
        );

    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    initialize_machine(&definition, &mut runtime, &mut selection).unwrap();

    // Without the custom flag, no transition
    let facts = CharacterAnimationFacts::default();
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Idle");

    // With the custom flag, transition fires
    let facts = CharacterAnimationFacts {
        custom_flags: vec!["powered_up".into()],
        ..Default::default()
    };
    advance_machine(
        &definition,
        &mut runtime,
        &facts,
        None,
        0.016,
        &mut selection,
    )
    .unwrap();
    assert_eq!(runtime.current_state.as_ref().unwrap().0, "Special");
}
