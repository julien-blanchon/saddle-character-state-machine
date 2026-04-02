use bevy::prelude::*;

use crate::config::{
    CharacterActionId, CharacterAnimationBindingId, CharacterStateId,
    CharacterStateMachineDefinitionId, CharacterTransitionId, TransitionCondition,
    TransitionOperation,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub enum LocomotionMode {
    #[default]
    Idle,
    Walk,
    Run,
    Sprint,
    Custom(String),
}

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
    pub speed: f32,
    pub movement_direction: Vec2,
    pub locomotion_intensity: f32,
    pub locomotion_mode: LocomotionMode,
    pub grounded: bool,
    pub wall_contact: bool,
    pub vertical_velocity: f32,
    pub facing_direction: Vec2,
    pub aim_direction: Vec2,
    pub animation_normalized_time: f32,
    pub clip_finished: bool,
    pub exit_window_open: bool,
    pub animation_locked: bool,
    pub inhibit_flags: Vec<String>,
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
}

impl CharacterStateMachineRuntime {
    pub fn top_frame(&self) -> Option<&ActiveStateFrame> {
        self.state_stack.last()
    }

    pub fn top_frame_mut(&mut self) -> Option<&mut ActiveStateFrame> {
        self.state_stack.last_mut()
    }
}
