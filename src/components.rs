use std::collections::HashMap;

use bevy::prelude::*;

use crate::config::{
    CharacterActionId, CharacterAnimationBindingId, CharacterFactId, CharacterFactTag,
    CharacterStateId, CharacterStateMachineDefinitionId, CharacterTransitionId,
    TransitionCondition, TransitionOperation,
};

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CharacterStateMachine {
    pub definition_id: CharacterStateMachineDefinitionId,
    pub enabled: bool,
    pub time_scale: f32,
}

impl CharacterStateMachine {
    pub fn new(definition_id: impl Into<CharacterStateMachineDefinitionId>) -> Self {
        Self {
            definition_id: definition_id.into(),
            enabled: true,
            time_scale: 1.0,
        }
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CharacterAnimationFacts {
    pub numbers: HashMap<CharacterFactId, f32>,
    pub booleans: HashMap<CharacterFactId, bool>,
    pub vectors: HashMap<CharacterFactId, Vec2>,
    pub tags: Vec<CharacterFactTag>,
    pub animation_normalized_time: f32,
    pub clip_finished: bool,
    pub exit_window_open: bool,
}

impl CharacterAnimationFacts {
    pub fn with_number(mut self, fact: impl Into<CharacterFactId>, value: f32) -> Self {
        self.set_number(fact, value);
        self
    }

    pub fn set_number(&mut self, fact: impl Into<CharacterFactId>, value: f32) {
        self.numbers.insert(fact.into(), value);
    }

    pub fn remove_number(&mut self, fact: &str) -> Option<f32> {
        self.numbers.remove(fact)
    }

    pub fn number(&self, fact: &str) -> Option<f32> {
        self.numbers.get(fact).copied()
    }

    pub fn number_or(&self, fact: &str, default: f32) -> f32 {
        self.number(fact).unwrap_or(default)
    }

    pub fn with_boolean(mut self, fact: impl Into<CharacterFactId>, value: bool) -> Self {
        self.set_boolean(fact, value);
        self
    }

    pub fn set_boolean(&mut self, fact: impl Into<CharacterFactId>, value: bool) {
        self.booleans.insert(fact.into(), value);
    }

    pub fn remove_boolean(&mut self, fact: &str) -> Option<bool> {
        self.booleans.remove(fact)
    }

    pub fn boolean(&self, fact: &str) -> Option<bool> {
        self.booleans.get(fact).copied()
    }

    pub fn boolean_or(&self, fact: &str, default: bool) -> bool {
        self.boolean(fact).unwrap_or(default)
    }

    pub fn with_vec2(mut self, fact: impl Into<CharacterFactId>, value: Vec2) -> Self {
        self.set_vec2(fact, value);
        self
    }

    pub fn set_vec2(&mut self, fact: impl Into<CharacterFactId>, value: Vec2) {
        self.vectors.insert(fact.into(), value);
    }

    pub fn remove_vec2(&mut self, fact: &str) -> Option<Vec2> {
        self.vectors.remove(fact)
    }

    pub fn vec2(&self, fact: &str) -> Option<Vec2> {
        self.vectors.get(fact).copied()
    }

    pub fn vec2_or(&self, fact: &str, default: Vec2) -> Vec2 {
        self.vec2(fact).unwrap_or(default)
    }

    pub fn with_tag(mut self, tag: impl Into<CharacterFactTag>) -> Self {
        self.insert_tag(tag);
        self
    }

    pub fn insert_tag(&mut self, tag: impl Into<CharacterFactTag>) {
        let tag = tag.into();
        if self.tags.iter().all(|existing| existing != &tag) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(index) = self.tags.iter().position(|existing| existing.0 == tag) {
            self.tags.remove(index);
            return true;
        }
        false
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|existing| existing.0 == tag)
    }

    pub fn clear_tags_with_prefix(&mut self, prefix: &str) {
        self.tags.retain(|tag| !tag.0.starts_with(prefix));
    }

    pub fn clear_gameplay_facts(&mut self) {
        self.numbers.clear();
        self.booleans.clear();
        self.vectors.clear();
        self.tags.clear();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
pub struct CharacterActionRequest {
    pub action: CharacterActionId,
}

impl CharacterActionRequest {
    pub fn new(action: impl Into<CharacterActionId>) -> Self {
        Self {
            action: action.into(),
        }
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CharacterAnimationRequests {
    pub queue: Vec<CharacterActionRequest>,
}

impl CharacterAnimationRequests {
    pub fn push(&mut self, action: impl Into<CharacterActionId>) {
        self.queue.push(CharacterActionRequest::new(action));
    }

    pub fn contains(&self, action: &CharacterActionId) -> bool {
        self.queue.iter().any(|request| &request.action == action)
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct ActiveStateFrame {
    pub state: CharacterStateId,
    pub elapsed_seconds: f32,
}

impl ActiveStateFrame {
    pub fn new(state: impl Into<CharacterStateId>) -> Self {
        Self {
            state: state.into(),
            elapsed_seconds: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
pub enum TransitionOutcome {
    Applied,
    Rejected,
    Held,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum TransitionRejectionReason {
    MissingDefinition(CharacterStateMachineDefinitionId),
    MissingState(CharacterStateId),
    MissingTargetState(CharacterStateId),
    StateLocked(CharacterStateId),
    MinimumDurationNotMet {
        required_seconds: f32,
        actual_seconds: f32,
    },
    ExitWindowClosed {
        start: f32,
        end: f32,
        current: f32,
    },
    GuardFailed(TransitionCondition),
    GuardAnyGroupFailed,
    PushConflict(CharacterStateId),
    EmptyStack,
    SelfTransitionDisallowed(CharacterStateId),
    MissingBinding(CharacterAnimationBindingId),
    NoTransitionMatched,
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct TransitionDecisionTrace {
    pub transition_id: Option<CharacterTransitionId>,
    pub source_state: Option<CharacterStateId>,
    pub target_state: Option<CharacterStateId>,
    pub operation: Option<TransitionOperation>,
    pub outcome: Option<TransitionOutcome>,
    pub reason: Option<TransitionRejectionReason>,
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CharacterStateMachineRuntime {
    pub current_state: Option<CharacterStateId>,
    pub previous_state: Option<CharacterStateId>,
    pub state_stack: Vec<ActiveStateFrame>,
    pub current_binding: Option<CharacterAnimationBindingId>,
    pub pending_request: Option<CharacterActionId>,
    pub queued_requests: Vec<CharacterActionId>,
    pub state_elapsed_seconds: f32,
    pub normalized_time: f32,
    pub machine_time_seconds: f32,
    pub generation: u64,
    pub last_transition: Option<TransitionDecisionTrace>,
    pub(crate) previous_normalized_time: f32,
    pub(crate) fired_event_indices: Vec<usize>,
}

impl CharacterStateMachineRuntime {
    pub fn top_frame(&self) -> Option<&ActiveStateFrame> {
        self.state_stack.last()
    }

    pub fn top_frame_mut(&mut self) -> Option<&mut ActiveStateFrame> {
        self.state_stack.last_mut()
    }
}
