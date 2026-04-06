mod bindings;
mod components;
mod config;
pub mod extensions;
pub mod locomotion;
mod machine;
mod messages;
mod systems;

pub use bindings::{
    BevyAnimationBinding, BevyAnimationBridge, CharacterAnimationActiveBinding,
    CharacterAnimationBindingSlot, CharacterAnimationLayer, CharacterAnimationLayerMode,
    CharacterAnimationLayers, CharacterAnimationSelection, PlaybackRepeat,
};
pub use components::{
    ActiveStateFrame, CharacterActionRequest, CharacterAnimationFacts, CharacterAnimationRequests,
    CharacterStateMachine, CharacterStateMachineRuntime, TransitionDecisionTrace,
    TransitionOutcome, TransitionRejectionReason,
};
pub use config::{
    AnimationEventDefinition, AnimationEventId, BlendDefinition, BlendEasing, BlendTree1D,
    BlendTree1DPoint, BlendTreeParameter, CharacterActionId, CharacterAnimationBindingId,
    CharacterFactId, CharacterFactTag, CharacterStateId, CharacterStateMachineDefinition,
    CharacterStateMachineDefinitionId, CharacterStateMachineLibrary,
    CharacterStateMachineValidationError, CharacterTransitionId, NormalizedTimeWindow,
    PushConflictPolicy, ResumePolicy, StateDefinition, StateKind, TransitionCondition,
    TransitionDefinition, TransitionGuard, TransitionOperation, TransitionSource,
};
pub use messages::{
    AnimationBindingMissing, AnimationEventFired, StateEntered, StateExited, StatePopped,
    StatePushed, TransitionRejected,
};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterStateMachineSystems {
    GatherFacts,
    ResolveTransitions,
    ApplyAnimation,
    Cleanup,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct CharacterStateMachinePlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl CharacterStateMachinePlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Plugin for CharacterStateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterStateMachineLibrary>()
            .add_message::<StateEntered>()
            .add_message::<StateExited>()
            .add_message::<StatePushed>()
            .add_message::<StatePopped>()
            .add_message::<TransitionRejected>()
            .add_message::<AnimationBindingMissing>()
            .add_message::<AnimationEventFired>()
            .register_type::<AnimationEventDefinition>()
            .register_type::<AnimationEventId>()
            .register_type::<ActiveStateFrame>()
            .register_type::<BevyAnimationBinding>()
            .register_type::<BevyAnimationBridge>()
            .register_type::<BlendDefinition>()
            .register_type::<BlendEasing>()
            .register_type::<BlendTree1D>()
            .register_type::<BlendTree1DPoint>()
            .register_type::<BlendTreeParameter>()
            .register_type::<CharacterActionId>()
            .register_type::<CharacterActionRequest>()
            .register_type::<CharacterAnimationActiveBinding>()
            .register_type::<CharacterAnimationBindingId>()
            .register_type::<CharacterAnimationFacts>()
            .register_type::<CharacterAnimationBindingSlot>()
            .register_type::<CharacterAnimationLayer>()
            .register_type::<CharacterAnimationLayerMode>()
            .register_type::<CharacterAnimationLayers>()
            .register_type::<CharacterAnimationRequests>()
            .register_type::<CharacterAnimationSelection>()
            .register_type::<CharacterFactId>()
            .register_type::<CharacterFactTag>()
            .register_type::<CharacterStateId>()
            .register_type::<CharacterStateMachine>()
            .register_type::<CharacterStateMachineDefinitionId>()
            .register_type::<CharacterStateMachineRuntime>()
            .register_type::<locomotion::LocomotionMode>()
            .register_type::<PlaybackRepeat>()
            .register_type::<TransitionDecisionTrace>()
            .register_type::<TransitionOutcome>()
            .register_type::<TransitionRejectionReason>()
            .add_systems(self.activate_schedule, systems::activate_machines)
            .add_systems(self.deactivate_schedule, systems::deactivate_machines)
            .configure_sets(
                self.update_schedule,
                (
                    CharacterStateMachineSystems::GatherFacts,
                    CharacterStateMachineSystems::ResolveTransitions,
                    CharacterStateMachineSystems::ApplyAnimation,
                    CharacterStateMachineSystems::Cleanup,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                (
                    (
                        systems::initialize_new_machines,
                        systems::sync_bevy_animation_facts,
                    )
                        .chain()
                        .in_set(CharacterStateMachineSystems::GatherFacts),
                    systems::advance_machines
                        .in_set(CharacterStateMachineSystems::ResolveTransitions),
                    (
                        systems::fire_animation_events,
                        systems::expand_animation_selection,
                        systems::apply_animation_selection,
                    )
                        .chain()
                        .in_set(CharacterStateMachineSystems::ApplyAnimation),
                    systems::clear_transient_requests.in_set(CharacterStateMachineSystems::Cleanup),
                ),
            );
    }
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod plugin_tests;
