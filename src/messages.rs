use bevy::prelude::*;

use crate::components::TransitionRejectionReason;
use crate::config::{
    AnimationEventId, CharacterAnimationBindingId, CharacterStateId,
    CharacterStateMachineDefinitionId, CharacterTransitionId,
};

#[derive(Clone, Debug, Message, Reflect)]
pub struct StateEntered {
    pub entity: Entity,
    pub definition_id: CharacterStateMachineDefinitionId,
    pub state: CharacterStateId,
    pub stack_depth: usize,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct StateExited {
    pub entity: Entity,
    pub definition_id: CharacterStateMachineDefinitionId,
    pub state: CharacterStateId,
    pub stack_depth: usize,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct StatePushed {
    pub entity: Entity,
    pub definition_id: CharacterStateMachineDefinitionId,
    pub state: CharacterStateId,
    pub pushed_over: CharacterStateId,
    pub stack_depth: usize,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct StatePopped {
    pub entity: Entity,
    pub definition_id: CharacterStateMachineDefinitionId,
    pub popped_state: CharacterStateId,
    pub resumed_state: CharacterStateId,
    pub stack_depth: usize,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct TransitionRejected {
    pub entity: Entity,
    pub definition_id: CharacterStateMachineDefinitionId,
    pub transition_id: CharacterTransitionId,
    pub reason: TransitionRejectionReason,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct AnimationBindingMissing {
    pub entity: Entity,
    pub definition_id: CharacterStateMachineDefinitionId,
    pub state: CharacterStateId,
    pub binding: CharacterAnimationBindingId,
}

#[derive(Clone, Debug, Message, Reflect)]
pub struct AnimationEventFired {
    pub entity: Entity,
    pub definition_id: CharacterStateMachineDefinitionId,
    pub state: CharacterStateId,
    pub event_id: AnimationEventId,
    pub normalized_time: f32,
}
