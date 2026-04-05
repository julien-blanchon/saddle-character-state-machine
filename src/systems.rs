use std::{collections::HashSet, time::Duration};

use bevy::prelude::*;

use crate::bindings::{
    BevyAnimationBridge, CharacterAnimationActiveBinding, CharacterAnimationLayers,
    CharacterAnimationSelection,
};
use crate::components::{
    CharacterAnimationFacts, CharacterAnimationRequests, CharacterStateMachine,
    CharacterStateMachineRuntime, TransitionRejectionReason,
};
use crate::config::{CharacterStateMachineLibrary, StateDefinition};
use crate::machine::{self, MachineEvent};
use crate::messages::{
    AnimationBindingMissing, AnimationEventFired, StateEntered, StateExited, StatePopped,
    StatePushed, TransitionRejected,
};

pub(crate) fn activate_machines(
    mut commands: Commands,
    library: Res<CharacterStateMachineLibrary>,
    machines: Query<(Entity, &CharacterStateMachine), Without<CharacterStateMachineRuntime>>,
    mut entered_writer: MessageWriter<StateEntered>,
) {
    for (entity, machine_component) in &machines {
        initialize_machine_components(
            &mut commands,
            &library,
            &mut entered_writer,
            entity,
            machine_component,
            "activation",
        );
    }
}

pub(crate) fn initialize_new_machines(
    mut commands: Commands,
    library: Res<CharacterStateMachineLibrary>,
    machines: Query<(Entity, &CharacterStateMachine), Without<CharacterStateMachineRuntime>>,
    mut entered_writer: MessageWriter<StateEntered>,
) {
    for (entity, machine_component) in &machines {
        initialize_machine_components(
            &mut commands,
            &library,
            &mut entered_writer,
            entity,
            machine_component,
            "update initialization",
        );
    }
}

pub(crate) fn deactivate_machines(
    mut commands: Commands,
    machines: Query<Entity, With<CharacterStateMachine>>,
) {
    for entity in &machines {
        commands
            .entity(entity)
            .remove::<CharacterStateMachineRuntime>()
            .remove::<CharacterAnimationSelection>();
    }
}

pub(crate) fn sync_bevy_animation_facts(
    mut machines: Query<
        (
            Entity,
            &CharacterAnimationSelection,
            &BevyAnimationBridge,
            &mut CharacterAnimationFacts,
        ),
        With<CharacterStateMachineRuntime>,
    >,
    players: Query<&AnimationPlayer>,
) {
    for (entity, selection, bridge, mut facts) in &mut machines {
        let Some(binding_id) = selection.binding.as_ref() else {
            continue;
        };
        let Some(binding) = bridge.binding(binding_id) else {
            continue;
        };
        let player_entity = bridge.player_entity.unwrap_or(entity);
        let Ok(player) = players.get(player_entity) else {
            continue;
        };

        let animation_index = AnimationNodeIndex::new(binding.node_index as usize);
        let Some(active_animation) = player.animation(animation_index) else {
            continue;
        };

        facts.clip_finished = active_animation.is_finished();
        if let Some(duration_seconds) = binding.duration_seconds.filter(|duration| *duration > 0.0)
        {
            facts.animation_normalized_time =
                (active_animation.seek_time() / duration_seconds).clamp(0.0, 1.0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn advance_machines(
    time: Res<Time>,
    library: Res<CharacterStateMachineLibrary>,
    mut machines: Query<(
        Entity,
        &CharacterStateMachine,
        &CharacterAnimationFacts,
        Option<&CharacterAnimationRequests>,
        &mut CharacterStateMachineRuntime,
        &mut CharacterAnimationSelection,
    )>,
    mut entered_writer: MessageWriter<StateEntered>,
    mut exited_writer: MessageWriter<StateExited>,
    mut pushed_writer: MessageWriter<StatePushed>,
    mut popped_writer: MessageWriter<StatePopped>,
    mut rejected_writer: MessageWriter<TransitionRejected>,
) {
    for (entity, machine_component, facts, requests, mut runtime, mut selection) in &mut machines {
        if !machine_component.enabled {
            runtime.last_transition = Some(crate::components::TransitionDecisionTrace {
                outcome: Some(crate::components::TransitionOutcome::Held),
                reason: Some(TransitionRejectionReason::NoTransitionMatched),
                ..default()
            });
            continue;
        }

        let Some(definition) = library.get(&machine_component.definition_id) else {
            runtime.last_transition = Some(crate::components::TransitionDecisionTrace {
                outcome: Some(crate::components::TransitionOutcome::Rejected),
                reason: Some(TransitionRejectionReason::MissingDefinition(
                    machine_component.definition_id.clone(),
                )),
                ..default()
            });
            continue;
        };

        let delta_seconds = time.delta_secs() * machine_component.time_scale;
        match machine::advance_machine(
            definition,
            &mut runtime,
            facts,
            requests,
            delta_seconds,
            &mut selection,
        ) {
            Ok(result) => {
                for event in result.events {
                    match event {
                        MachineEvent::Entered(state) => {
                            entered_writer.write(StateEntered {
                                entity,
                                definition_id: machine_component.definition_id.clone(),
                                state,
                                stack_depth: runtime.state_stack.len(),
                            });
                        }
                        MachineEvent::Exited(state) => {
                            exited_writer.write(StateExited {
                                entity,
                                definition_id: machine_component.definition_id.clone(),
                                state,
                                stack_depth: runtime.state_stack.len(),
                            });
                        }
                        MachineEvent::Pushed { state, over } => {
                            pushed_writer.write(StatePushed {
                                entity,
                                definition_id: machine_component.definition_id.clone(),
                                state,
                                pushed_over: over,
                                stack_depth: runtime.state_stack.len(),
                            });
                        }
                        MachineEvent::Popped { popped, resumed } => {
                            popped_writer.write(StatePopped {
                                entity,
                                definition_id: machine_component.definition_id.clone(),
                                popped_state: popped,
                                resumed_state: resumed,
                                stack_depth: runtime.state_stack.len(),
                            });
                        }
                        MachineEvent::Rejected {
                            transition_id,
                            reason,
                        } => {
                            rejected_writer.write(TransitionRejected {
                                entity,
                                definition_id: machine_component.definition_id.clone(),
                                transition_id,
                                reason,
                            });
                        }
                    }
                }
            }
            Err(reason) => {
                runtime.last_transition = Some(crate::components::TransitionDecisionTrace {
                    outcome: Some(crate::components::TransitionOutcome::Rejected),
                    reason: Some(reason),
                    ..default()
                });
            }
        }
    }
}

pub(crate) fn expand_animation_selection(
    library: Res<CharacterStateMachineLibrary>,
    mut machines: Query<(
        &CharacterStateMachine,
        &CharacterAnimationFacts,
        Option<&CharacterAnimationLayers>,
        &mut CharacterStateMachineRuntime,
        &mut CharacterAnimationSelection,
    )>,
) {
    for (machine_component, facts, layers, mut runtime, mut selection) in &mut machines {
        let Some(definition) = library.get(&machine_component.definition_id) else {
            continue;
        };

        let active_bindings = selection
            .state
            .as_ref()
            .and_then(|state_id| definition.state(state_id))
            .map_or_else(
                || legacy_base_selection(&selection),
                |state| base_active_bindings(state, &selection, facts),
            );

        let mut expanded = active_bindings;
        let base_sync = selection
            .sync_normalized_time
            .or(Some(runtime.normalized_time.clamp(0.0, 1.0)));
        if let Some(layers) = layers {
            for layer in &layers.layers {
                if !layer.enabled || !layer.weight.is_finite() || layer.weight <= 0.0 {
                    continue;
                }
                expanded.push(CharacterAnimationActiveBinding::layer(
                    layer.name.clone(),
                    layer.binding.clone(),
                    layer.weight.max(0.0),
                    layer.sync_to_base_time.then_some(base_sync).flatten(),
                ));
            }
        }

        let dominant_binding = expanded
            .iter()
            .filter(|entry| {
                matches!(
                    entry.slot,
                    crate::bindings::CharacterAnimationBindingSlot::Base
                )
            })
            .max_by(|left, right| left.weight.total_cmp(&right.weight))
            .map(|entry| entry.binding.clone())
            .or_else(|| selection.binding.clone());

        if selection.active_bindings != expanded {
            selection.active_bindings = expanded;
        }
        if selection.binding != dominant_binding {
            selection.binding = dominant_binding.clone();
        }
        if runtime.current_binding != dominant_binding {
            runtime.current_binding = dominant_binding;
        }
    }
}

pub(crate) fn apply_animation_selection(
    mut commands: Commands,
    machines: Query<
        (
            Entity,
            &CharacterStateMachine,
            &CharacterAnimationSelection,
            &BevyAnimationBridge,
        ),
        Changed<CharacterAnimationSelection>,
    >,
    mut players: Query<(
        &mut AnimationPlayer,
        Option<&mut AnimationTransitions>,
        Option<&AnimationGraphHandle>,
    )>,
    mut missing_binding_writer: MessageWriter<AnimationBindingMissing>,
) {
    for (entity, machine_component, selection, bridge) in &machines {
        let desired = if selection.active_bindings.is_empty() {
            legacy_base_selection(selection)
        } else {
            selection.active_bindings.clone()
        };
        if desired.is_empty() {
            continue;
        }

        let player_entity = bridge.player_entity.unwrap_or(entity);
        let Ok((mut player, maybe_transitions, graph_handle)) = players.get_mut(player_entity)
        else {
            continue;
        };

        let mut resolved = Vec::with_capacity(desired.len());
        for entry in desired {
            let Some(binding) = bridge.binding(&entry.binding) else {
                if let Some(state) = selection.state.clone() {
                    missing_binding_writer.write(AnimationBindingMissing {
                        entity,
                        definition_id: machine_component.definition_id.clone(),
                        state,
                        binding: entry.binding.clone(),
                    });
                }
                warn!(
                    "character_state_machine: missing bevy animation binding '{}' on entity {entity:?}",
                    entry.binding
                );
                continue;
            };
            resolved.push((entry, binding));
        }
        if resolved.is_empty() {
            continue;
        }

        if graph_handle.is_none() {
            commands
                .entity(player_entity)
                .insert(AnimationGraphHandle(bridge.graph_handle.clone()));
        }

        let weighted_mode = resolved.len() > 1
            || resolved
                .iter()
                .any(|(entry, _)| (entry.weight - 1.0).abs() > f32::EPSILON);

        if weighted_mode {
            if maybe_transitions.is_some() {
                commands
                    .entity(player_entity)
                    .remove::<AnimationTransitions>();
            }
            apply_weighted_bindings(&mut player, &resolved);
            continue;
        }

        let (entry, binding) = resolved.remove(0);
        let animation_index = AnimationNodeIndex::new(binding.node_index as usize);
        let active_indices = player
            .playing_animations()
            .map(|(index, _)| *index)
            .collect::<Vec<_>>();
        for index in active_indices {
            if index != animation_index {
                player.stop(index);
            }
        }

        let transition_duration =
            Duration::from_secs_f32(selection.blend.duration_seconds.max(0.0));
        let active_animation = if let Some(mut transitions) = maybe_transitions {
            transitions.play(&mut player, animation_index, transition_duration)
        } else {
            let mut transitions = AnimationTransitions::new();
            let active_animation =
                transitions.play(&mut player, animation_index, transition_duration);
            commands.entity(player_entity).insert(transitions);
            active_animation
        };
        active_animation
            .set_repeat(binding.repeat.to_bevy())
            .set_weight(entry.weight.max(0.0));

        if let Some(duration_seconds) = binding.duration_seconds.filter(|duration| *duration > 0.0)
            && let Some(sync_normalized_time) = entry.sync_normalized_time
        {
            active_animation.seek_to(sync_normalized_time.clamp(0.0, 1.0) * duration_seconds);
        }
    }
}

pub(crate) fn fire_animation_events(
    library: Res<CharacterStateMachineLibrary>,
    mut machines: Query<(
        Entity,
        &CharacterStateMachine,
        &mut CharacterStateMachineRuntime,
    )>,
    mut event_writer: MessageWriter<AnimationEventFired>,
) {
    for (entity, machine_component, mut runtime) in &mut machines {
        let Some(definition) = library.get(&machine_component.definition_id) else {
            continue;
        };
        let Some(state_id) = runtime.current_state.clone() else {
            continue;
        };
        let Some(state) = definition.state(&state_id) else {
            continue;
        };
        if state.events.is_empty() {
            continue;
        }

        let previous = runtime.previous_normalized_time;
        let current = runtime.normalized_time;

        for (index, event) in state.events.iter().enumerate() {
            if runtime.fired_event_indices.contains(&index) {
                continue;
            }
            if previous < event.normalized_time && current >= event.normalized_time {
                runtime.fired_event_indices.push(index);
                event_writer.write(AnimationEventFired {
                    entity,
                    definition_id: machine_component.definition_id.clone(),
                    state: state_id.clone(),
                    event_id: event.id.clone(),
                    normalized_time: event.normalized_time,
                });
            }
        }
    }
}

pub(crate) fn clear_transient_requests(mut requests: Query<&mut CharacterAnimationRequests>) {
    for mut queue in &mut requests {
        queue.clear();
    }
}

fn base_active_bindings(
    state: &StateDefinition,
    selection: &CharacterAnimationSelection,
    facts: &CharacterAnimationFacts,
) -> Vec<CharacterAnimationActiveBinding> {
    if let Some(blend_tree) = &state.blend_tree_1d {
        return blend_tree
            .evaluate(facts)
            .into_iter()
            .filter(|(_, weight)| weight.is_finite() && *weight > 0.0)
            .map(|(binding, weight)| {
                CharacterAnimationActiveBinding::base(
                    binding,
                    weight,
                    selection.sync_normalized_time,
                )
            })
            .collect();
    }

    legacy_base_selection(selection)
}

fn legacy_base_selection(
    selection: &CharacterAnimationSelection,
) -> Vec<CharacterAnimationActiveBinding> {
    selection
        .binding
        .clone()
        .map(|binding| {
            vec![CharacterAnimationActiveBinding::base(
                binding,
                1.0,
                selection.sync_normalized_time,
            )]
        })
        .unwrap_or_default()
}

fn apply_weighted_bindings(
    player: &mut AnimationPlayer,
    resolved: &[(
        CharacterAnimationActiveBinding,
        &crate::bindings::BevyAnimationBinding,
    )],
) {
    let desired_indices = resolved
        .iter()
        .map(|(_, binding)| AnimationNodeIndex::new(binding.node_index as usize))
        .collect::<HashSet<_>>();
    let active_indices = player
        .playing_animations()
        .map(|(index, _)| *index)
        .collect::<Vec<_>>();
    for index in active_indices {
        if !desired_indices.contains(&index) {
            player.stop(index);
        }
    }

    for (entry, binding) in resolved {
        let animation_index = AnimationNodeIndex::new(binding.node_index as usize);
        let active_animation = if player.is_playing_animation(animation_index) {
            player
                .animation_mut(animation_index)
                .expect("active animation should be retrievable")
        } else {
            player.start(animation_index)
        };
        active_animation
            .set_repeat(binding.repeat.to_bevy())
            .set_weight(entry.weight.max(0.0));

        if let Some(duration_seconds) = binding.duration_seconds.filter(|duration| *duration > 0.0)
            && let Some(sync_normalized_time) = entry.sync_normalized_time
        {
            active_animation.seek_to(sync_normalized_time.clamp(0.0, 1.0) * duration_seconds);
        }
    }
}

fn initialize_machine_components(
    commands: &mut Commands,
    library: &CharacterStateMachineLibrary,
    entered_writer: &mut MessageWriter<StateEntered>,
    entity: Entity,
    machine_component: &CharacterStateMachine,
    context: &str,
) {
    let Some(definition) = library.get(&machine_component.definition_id) else {
        warn!(
            "character_state_machine: missing definition '{}' during {context}",
            machine_component.definition_id
        );
        return;
    };

    let mut runtime = CharacterStateMachineRuntime::default();
    let mut selection = CharacterAnimationSelection::default();
    match machine::initialize_machine(definition, &mut runtime, &mut selection) {
        Ok(result) => {
            for event in result.events {
                if let MachineEvent::Entered(state) = event {
                    entered_writer.write(StateEntered {
                        entity,
                        definition_id: machine_component.definition_id.clone(),
                        state,
                        stack_depth: runtime.state_stack.len(),
                    });
                }
            }
            commands.entity(entity).insert((runtime, selection));
        }
        Err(reason) => {
            warn!(
                "character_state_machine: failed to initialize '{}' on entity {entity:?} during {context}: {reason:?}",
                machine_component.definition_id
            );
        }
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
