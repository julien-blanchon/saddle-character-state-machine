use bevy::prelude::*;

use crate::bindings::CharacterAnimationSelection;
use crate::components::{
    ActiveStateFrame, CharacterAnimationFacts, CharacterAnimationRequests,
    CharacterStateMachineRuntime, TransitionDecisionTrace, TransitionOutcome,
    TransitionRejectionReason,
};
use crate::config::{
    BlendDefinition, CharacterStateId, CharacterStateMachineDefinition, CharacterTransitionId,
    PushConflictPolicy, ResumePolicy, StateDefinition, StateKind, TransitionCondition,
    TransitionDefinition, TransitionOperation, TransitionSource,
};

#[derive(Clone, Debug)]
pub(crate) enum MachineEvent {
    Entered(CharacterStateId),
    Exited(CharacterStateId),
    Pushed {
        state: CharacterStateId,
        over: CharacterStateId,
    },
    Popped {
        popped: CharacterStateId,
        resumed: CharacterStateId,
    },
    Rejected {
        transition_id: CharacterTransitionId,
        reason: TransitionRejectionReason,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MachineStepResult {
    pub events: Vec<MachineEvent>,
}

#[derive(Clone, Debug)]
struct PlaybackSnapshot {
    normalized_time: f32,
    clip_finished: bool,
}

#[derive(Clone, Debug)]
struct ResolvedTransition<'a> {
    transition: &'a TransitionDefinition,
    target_state: Option<CharacterStateId>,
    replace_pushed_top: bool,
}

pub(crate) fn initialize_machine(
    definition: &CharacterStateMachineDefinition,
    runtime: &mut CharacterStateMachineRuntime,
    selection: &mut CharacterAnimationSelection,
) -> Result<MachineStepResult, TransitionRejectionReason> {
    let initial_state = definition
        .state(&definition.initial_state)
        .ok_or_else(|| TransitionRejectionReason::MissingState(definition.initial_state.clone()))?;

    runtime.current_state = Some(initial_state.id.clone());
    runtime.previous_state = None;
    runtime.state_stack.clear();
    runtime
        .state_stack
        .push(ActiveStateFrame::new(initial_state.id.clone()));
    runtime.state_elapsed_seconds = 0.0;
    runtime.normalized_time = 0.0;
    runtime.previous_normalized_time = 0.0;
    runtime.machine_time_seconds = 0.0;
    runtime.generation = 1;
    runtime.fired_event_indices.clear();
    runtime.last_transition = Some(TransitionDecisionTrace {
        source_state: None,
        target_state: Some(initial_state.id.clone()),
        outcome: Some(TransitionOutcome::Applied),
        ..default()
    });

    apply_selection(definition, runtime, selection);

    Ok(MachineStepResult {
        events: vec![MachineEvent::Entered(initial_state.id.clone())],
    })
}

pub(crate) fn advance_machine(
    definition: &CharacterStateMachineDefinition,
    runtime: &mut CharacterStateMachineRuntime,
    facts: &CharacterAnimationFacts,
    requests: Option<&CharacterAnimationRequests>,
    delta_seconds: f32,
    selection: &mut CharacterAnimationSelection,
) -> Result<MachineStepResult, TransitionRejectionReason> {
    if let Some(frame) = runtime.top_frame_mut() {
        frame.elapsed_seconds += delta_seconds.max(0.0);
        runtime.state_elapsed_seconds = frame.elapsed_seconds;
    }
    runtime.machine_time_seconds += delta_seconds.max(0.0);
    runtime.pending_request =
        requests.and_then(|queue| queue.queue.first().map(|r| r.action.clone()));
    runtime.queued_requests = requests
        .map(|queue| {
            queue
                .queue
                .iter()
                .map(|request| request.action.clone())
                .collect()
        })
        .unwrap_or_default();

    let current_state_id = runtime
        .current_state
        .clone()
        .ok_or_else(|| TransitionRejectionReason::MissingState(definition.initial_state.clone()))?;
    let current_state = definition
        .state(&current_state_id)
        .ok_or_else(|| TransitionRejectionReason::MissingState(current_state_id.clone()))?;
    let playback = derive_playback(current_state, runtime, facts);
    runtime.previous_normalized_time = runtime.normalized_time;
    runtime.normalized_time = playback.normalized_time;

    let (resolved_transition, first_rejection) = resolve_transition(
        definition,
        current_state,
        runtime,
        facts,
        requests,
        &playback,
    );
    let Some(transition) = resolved_transition else {
        let mut result = MachineStepResult::default();
        let (transition_id, reason, outcome) = match first_rejection {
            Some((transition_id, reason)) => {
                result.events.push(MachineEvent::Rejected {
                    transition_id: transition_id.clone(),
                    reason: reason.clone(),
                });
                (Some(transition_id), reason, TransitionOutcome::Rejected)
            }
            None => (
                None,
                TransitionRejectionReason::NoTransitionMatched,
                TransitionOutcome::Held,
            ),
        };
        runtime.last_transition = Some(TransitionDecisionTrace {
            source_state: Some(current_state_id),
            transition_id,
            outcome: Some(outcome),
            reason: Some(reason),
            ..default()
        });
        return Ok(result);
    };

    let result = apply_transition(definition, runtime, selection, transition);
    Ok(result)
}

fn derive_playback(
    state: &StateDefinition,
    runtime: &CharacterStateMachineRuntime,
    facts: &CharacterAnimationFacts,
) -> PlaybackSnapshot {
    let mut normalized_time = facts.animation_normalized_time.clamp(0.0, 1.0);
    let mut clip_finished = facts.clip_finished;

    if let Some(duration_seconds) = state
        .expected_duration_seconds
        .filter(|duration| *duration > 0.0)
    {
        let ratio = runtime.state_elapsed_seconds / duration_seconds;
        let derived_normalized = match state.kind {
            StateKind::Persistent => ratio.fract(),
            StateKind::Transient => ratio.min(1.0),
        };

        if facts.animation_normalized_time == 0.0 && !facts.clip_finished {
            normalized_time = derived_normalized;
        }

        if matches!(state.kind, StateKind::Transient)
            && runtime.state_elapsed_seconds >= duration_seconds
        {
            clip_finished = true;
            normalized_time = 1.0;
        }
    }

    PlaybackSnapshot {
        normalized_time,
        clip_finished,
    }
}

fn resolve_transition<'a>(
    definition: &'a CharacterStateMachineDefinition,
    current_state: &'a StateDefinition,
    runtime: &CharacterStateMachineRuntime,
    facts: &CharacterAnimationFacts,
    requests: Option<&CharacterAnimationRequests>,
    playback: &PlaybackSnapshot,
) -> (
    Option<ResolvedTransition<'a>>,
    Option<(CharacterTransitionId, TransitionRejectionReason)>,
) {
    let mut ranked = definition
        .transitions
        .iter()
        .enumerate()
        .filter_map(|(index, transition)| {
            source_rank(definition, &current_state.id, transition)
                .map(|rank| (rank, transition.priority, index, transition))
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(left.0.cmp(&right.0))
            .then(left.2.cmp(&right.2))
    });

    let mut first_rejection = None;

    for (_, _, _, transition) in ranked {
        match evaluate_transition(
            definition,
            current_state,
            runtime,
            facts,
            requests,
            playback,
            transition,
        ) {
            Ok(resolved) => return (Some(resolved), first_rejection),
            Err(reason) => {
                if first_rejection.is_none()
                    || matches!(reason, TransitionRejectionReason::StateLocked(_))
                    || (matches!(
                        first_rejection,
                        Some((
                            _,
                            TransitionRejectionReason::GuardFailed(
                                TransitionCondition::ActionRequested(_)
                            )
                        ))
                    ) && !matches!(
                        reason,
                        TransitionRejectionReason::GuardFailed(
                            TransitionCondition::ActionRequested(_)
                        )
                    ))
                {
                    first_rejection = Some((transition.id.clone(), reason.clone()));
                }
            }
        }
    }

    (None, first_rejection)
}

fn evaluate_transition<'a>(
    definition: &'a CharacterStateMachineDefinition,
    current_state: &'a StateDefinition,
    runtime: &CharacterStateMachineRuntime,
    facts: &CharacterAnimationFacts,
    requests: Option<&CharacterAnimationRequests>,
    playback: &PlaybackSnapshot,
    transition: &'a TransitionDefinition,
) -> Result<ResolvedTransition<'a>, TransitionRejectionReason> {
    let source_is_current_exact = match &transition.source {
        TransitionSource::Current => true,
        TransitionSource::State(state) => state == &current_state.id,
        TransitionSource::Any => false,
    };

    if !current_state.interruptible
        && matches!(
            transition.operation,
            TransitionOperation::Set | TransitionOperation::Push
        )
        && !transition.force_interrupt
        && !source_is_current_exact
    {
        return Err(TransitionRejectionReason::StateLocked(
            current_state.id.clone(),
        ));
    }

    let required_duration_seconds = current_state
        .minimum_duration_seconds
        .max(transition.minimum_source_duration_seconds.unwrap_or(0.0));
    if runtime.state_elapsed_seconds < required_duration_seconds {
        return Err(TransitionRejectionReason::MinimumDurationNotMet {
            required_seconds: required_duration_seconds,
            actual_seconds: runtime.state_elapsed_seconds,
        });
    }

    if let Some(window) = transition.exit_window
        && !window.contains(playback.normalized_time)
    {
        return Err(TransitionRejectionReason::ExitWindowClosed {
            start: window.start,
            end: window.end,
            current: playback.normalized_time,
        });
    }

    evaluate_guard(transition, facts, requests, runtime, playback)?;

    let target_state = transition
        .target
        .as_ref()
        .map(|target| {
            definition
                .state(target)
                .map(|state| state.id.clone())
                .ok_or_else(|| TransitionRejectionReason::MissingTargetState(target.clone()))
        })
        .transpose()?;

    if transition.operation == TransitionOperation::Pop && runtime.state_stack.len() <= 1 {
        return Err(TransitionRejectionReason::EmptyStack);
    }

    if let Some(target_state) = &target_state
        && target_state == &current_state.id
        && !transition.allow_self_transition
    {
        return Err(TransitionRejectionReason::SelfTransitionDisallowed(
            target_state.clone(),
        ));
    }

    let replace_pushed_top = transition.operation == TransitionOperation::Push
        && runtime.state_stack.len() > 1
        && transition.push_conflict_policy == PushConflictPolicy::ReplaceTop;
    if transition.operation == TransitionOperation::Push
        && runtime.state_stack.len() > 1
        && transition.push_conflict_policy == PushConflictPolicy::Reject
    {
        return Err(TransitionRejectionReason::PushConflict(
            current_state.id.clone(),
        ));
    }

    Ok(ResolvedTransition {
        transition,
        target_state,
        replace_pushed_top,
    })
}

fn evaluate_guard(
    transition: &TransitionDefinition,
    facts: &CharacterAnimationFacts,
    requests: Option<&CharacterAnimationRequests>,
    runtime: &CharacterStateMachineRuntime,
    playback: &PlaybackSnapshot,
) -> Result<(), TransitionRejectionReason> {
    for condition in &transition.guard.all {
        if !matches_condition(condition, facts, requests, runtime, playback) {
            return Err(TransitionRejectionReason::GuardFailed(condition.clone()));
        }
    }

    if !transition.guard.any.is_empty()
        && !transition
            .guard
            .any
            .iter()
            .any(|condition| matches_condition(condition, facts, requests, runtime, playback))
    {
        return Err(TransitionRejectionReason::GuardAnyGroupFailed);
    }

    for condition in &transition.guard.none {
        if matches_condition(condition, facts, requests, runtime, playback) {
            return Err(TransitionRejectionReason::GuardFailed(condition.clone()));
        }
    }

    Ok(())
}

fn matches_condition(
    condition: &TransitionCondition,
    facts: &CharacterAnimationFacts,
    requests: Option<&CharacterAnimationRequests>,
    runtime: &CharacterStateMachineRuntime,
    playback: &PlaybackSnapshot,
) -> bool {
    match condition {
        TransitionCondition::Always => true,
        TransitionCondition::Bool(fact, value) => facts.boolean_or(&fact.0, false) == *value,
        TransitionCondition::NumberAtLeast(fact, value) => facts.number_or(&fact.0, 0.0) >= *value,
        TransitionCondition::NumberAtMost(fact, value) => facts.number_or(&fact.0, 0.0) <= *value,
        TransitionCondition::Vec2LengthAtLeast(fact, value) => {
            facts.vec2_or(&fact.0, Vec2::ZERO).length() >= *value
        }
        TransitionCondition::Vec2LengthAtMost(fact, value) => {
            facts.vec2_or(&fact.0, Vec2::ZERO).length() <= *value
        }
        TransitionCondition::StateTimeAtLeast(value) => runtime.state_elapsed_seconds >= *value,
        TransitionCondition::StateTimeAtMost(value) => runtime.state_elapsed_seconds <= *value,
        TransitionCondition::NormalizedTimeAtLeast(value) => playback.normalized_time >= *value,
        TransitionCondition::NormalizedTimeAtMost(value) => playback.normalized_time <= *value,
        TransitionCondition::ExitWindowOpen => facts.exit_window_open,
        TransitionCondition::AnimationFinished => playback.clip_finished,
        TransitionCondition::ActionRequested(action) => {
            requests.is_some_and(|queue| queue.contains(action))
        }
        TransitionCondition::TagPresent(tag) => facts.has_tag(&tag.0),
        TransitionCondition::TagMissing(tag) => !facts.has_tag(&tag.0),
    }
}

fn apply_transition(
    definition: &CharacterStateMachineDefinition,
    runtime: &mut CharacterStateMachineRuntime,
    selection: &mut CharacterAnimationSelection,
    resolved: ResolvedTransition<'_>,
) -> MachineStepResult {
    let mut events = Vec::new();
    let previous_state = runtime.current_state.clone();
    let source_normalized_time = runtime.normalized_time;

    match resolved.transition.operation {
        TransitionOperation::Set => {
            if let Some(target_state) = resolved.target_state.clone() {
                if previous_state.as_ref() == Some(&target_state)
                    && !resolved
                        .transition
                        .blend
                        .unwrap_or(definition.default_blend)
                        .reset_on_entry
                {
                    runtime.last_transition = Some(TransitionDecisionTrace {
                        transition_id: Some(resolved.transition.id.clone()),
                        source_state: previous_state.clone(),
                        target_state: Some(target_state.clone()),
                        operation: Some(TransitionOperation::Set),
                        outcome: Some(TransitionOutcome::Held),
                        ..default()
                    });
                    return MachineStepResult::default();
                }

                if let Some(frame) = runtime.top_frame_mut() {
                    events.push(MachineEvent::Exited(frame.state.clone()));
                    frame.state = target_state.clone();
                    frame.elapsed_seconds = 0.0;
                }
                runtime.current_state = Some(target_state.clone());
                runtime.previous_state = previous_state.clone();
                runtime.state_elapsed_seconds = 0.0;
                runtime.normalized_time = 0.0;
                events.push(MachineEvent::Entered(target_state));
            }
        }
        TransitionOperation::Push => {
            if let Some(target_state) = resolved.target_state.clone() {
                if resolved.replace_pushed_top
                    && let Some(frame) = runtime.state_stack.pop()
                {
                    events.push(MachineEvent::Exited(frame.state));
                }

                let pushed_over = runtime
                    .current_state
                    .clone()
                    .expect("push transitions require an active state");
                runtime
                    .state_stack
                    .push(ActiveStateFrame::new(target_state.clone()));
                runtime.current_state = Some(target_state.clone());
                runtime.previous_state = Some(pushed_over.clone());
                runtime.state_elapsed_seconds = 0.0;
                runtime.normalized_time = 0.0;
                events.push(MachineEvent::Pushed {
                    state: target_state.clone(),
                    over: pushed_over,
                });
                events.push(MachineEvent::Entered(target_state));
            }
        }
        TransitionOperation::Pop => {
            let popped = runtime
                .state_stack
                .pop()
                .expect("pop transitions require a stacked state");
            let resumed_frame = runtime
                .state_stack
                .last_mut()
                .expect("pop transitions always leave a base state");

            if let Some(state) = definition.state(&resumed_frame.state)
                && state.resume_policy == ResumePolicy::ResetTime
            {
                resumed_frame.elapsed_seconds = 0.0;
            }

            runtime.previous_state = Some(popped.state.clone());
            runtime.current_state = Some(resumed_frame.state.clone());
            runtime.state_elapsed_seconds = resumed_frame.elapsed_seconds;
            runtime.normalized_time = 0.0;
            events.push(MachineEvent::Exited(popped.state.clone()));
            events.push(MachineEvent::Popped {
                popped: popped.state,
                resumed: resumed_frame.state.clone(),
            });
        }
    }

    runtime.generation += 1;
    runtime.fired_event_indices.clear();
    runtime.previous_normalized_time = 0.0;
    apply_selection(definition, runtime, selection);
    let blend = resolved
        .transition
        .blend
        .unwrap_or(definition.default_blend);
    selection.blend = blend;
    selection.sync_normalized_time = transition_sync_normalized_time(
        definition,
        runtime,
        source_normalized_time,
        blend,
        resolved.transition.operation,
    );
    runtime.last_transition = Some(TransitionDecisionTrace {
        transition_id: Some(resolved.transition.id.clone()),
        source_state: previous_state,
        target_state: runtime.current_state.clone(),
        operation: Some(resolved.transition.operation),
        outcome: Some(TransitionOutcome::Applied),
        ..default()
    });

    MachineStepResult { events }
}

fn apply_selection(
    definition: &CharacterStateMachineDefinition,
    runtime: &mut CharacterStateMachineRuntime,
    selection: &mut CharacterAnimationSelection,
) {
    let current_state = runtime.current_state.clone();
    let binding = current_state
        .as_ref()
        .and_then(|state| definition.resolve_binding(state).0);

    selection.state = current_state.clone();
    selection.binding = binding.clone();
    selection.blend = definition.default_blend;
    selection.generation = runtime.generation;
    selection.sync_normalized_time = None;
    runtime.current_binding = binding;
}

fn transition_sync_normalized_time(
    definition: &CharacterStateMachineDefinition,
    runtime: &CharacterStateMachineRuntime,
    source_normalized_time: f32,
    blend: BlendDefinition,
    operation: TransitionOperation,
) -> Option<f32> {
    match operation {
        TransitionOperation::Pop => current_state_normalized_time(definition, runtime),
        _ if blend.sync_to_source_time => Some(source_normalized_time.clamp(0.0, 1.0)),
        _ if !blend.reset_on_entry => current_state_normalized_time(definition, runtime),
        _ => None,
    }
}

fn current_state_normalized_time(
    definition: &CharacterStateMachineDefinition,
    runtime: &CharacterStateMachineRuntime,
) -> Option<f32> {
    let state_id = runtime.current_state.as_ref()?;
    let state = definition.state(state_id)?;
    let duration_seconds = state
        .expected_duration_seconds
        .filter(|duration| *duration > 0.0)?;
    let ratio = runtime.state_elapsed_seconds / duration_seconds;

    Some(match state.kind {
        StateKind::Persistent => ratio.fract(),
        StateKind::Transient => ratio.min(1.0),
    })
}

fn source_rank(
    definition: &CharacterStateMachineDefinition,
    current_state: &CharacterStateId,
    transition: &TransitionDefinition,
) -> Option<usize> {
    match &transition.source {
        TransitionSource::Current => Some(0),
        TransitionSource::Any => Some(usize::MAX / 4),
        TransitionSource::State(source) => definition
            .parent_chain(current_state)
            .iter()
            .position(|state| &state.id == source),
    }
}

#[cfg(test)]
#[path = "machine_tests.rs"]
mod tests;
