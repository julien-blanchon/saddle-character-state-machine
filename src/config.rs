use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use bevy::prelude::*;

use crate::components::{CharacterAnimationFacts, LocomotionMode};

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Reflect)]
        #[reflect(Default, PartialEq, Hash)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

define_id!(CharacterStateMachineDefinitionId);
define_id!(CharacterStateId);
define_id!(CharacterActionId);
define_id!(CharacterAnimationBindingId);
define_id!(CharacterTransitionId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum StateKind {
    Persistent,
    Transient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum ResumePolicy {
    PreserveTime,
    ResetTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum TransitionOperation {
    Set,
    Push,
    Pop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum PushConflictPolicy {
    Stack,
    ReplaceTop,
    Reject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum BlendEasing {
    Linear,
    SmoothStep,
    SmootherStep,
    QuadraticInOut,
    CubicInOut,
    SineInOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct BlendDefinition {
    pub duration_seconds: f32,
    pub easing: BlendEasing,
    pub reset_on_entry: bool,
    pub sync_to_source_time: bool,
}

impl Default for BlendDefinition {
    fn default() -> Self {
        Self {
            duration_seconds: 0.15,
            easing: BlendEasing::Linear,
            reset_on_entry: true,
            sync_to_source_time: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum BlendTreeParameter {
    Speed,
    LocomotionIntensity,
    VerticalVelocity,
    MovementDirectionX,
    MovementDirectionY,
    FacingDirectionX,
    AimDirectionX,
}

impl BlendTreeParameter {
    pub fn sample(self, facts: &CharacterAnimationFacts) -> f32 {
        match self {
            Self::Speed => facts.speed,
            Self::LocomotionIntensity => facts.locomotion_intensity,
            Self::VerticalVelocity => facts.vertical_velocity,
            Self::MovementDirectionX => facts.movement_direction.x,
            Self::MovementDirectionY => facts.movement_direction.y,
            Self::FacingDirectionX => facts.facing_direction.x,
            Self::AimDirectionX => facts.aim_direction.x,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct BlendTree1DPoint {
    pub threshold: f32,
    pub binding: CharacterAnimationBindingId,
}

impl BlendTree1DPoint {
    pub fn new(threshold: f32, binding: impl Into<CharacterAnimationBindingId>) -> Self {
        Self {
            threshold,
            binding: binding.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct BlendTree1D {
    pub parameter: BlendTreeParameter,
    pub points: Vec<BlendTree1DPoint>,
}

impl BlendTree1D {
    pub fn new(parameter: BlendTreeParameter) -> Self {
        Self {
            parameter,
            points: Vec::new(),
        }
    }

    pub fn with_point(
        mut self,
        threshold: f32,
        binding: impl Into<CharacterAnimationBindingId>,
    ) -> Self {
        self.points.push(BlendTree1DPoint::new(threshold, binding));
        self
    }

    pub fn evaluate(
        &self,
        facts: &CharacterAnimationFacts,
    ) -> Vec<(CharacterAnimationBindingId, f32)> {
        if self.points.is_empty() {
            return Vec::new();
        }

        let mut points = self.points.clone();
        points.sort_by(|left, right| left.threshold.total_cmp(&right.threshold));

        if points.len() == 1 {
            return vec![(points[0].binding.clone(), 1.0)];
        }

        let value = self.parameter.sample(facts);
        if value <= points[0].threshold {
            return vec![(points[0].binding.clone(), 1.0)];
        }
        if value >= points[points.len() - 1].threshold {
            return vec![(points[points.len() - 1].binding.clone(), 1.0)];
        }

        for window in points.windows(2) {
            let left = &window[0];
            let right = &window[1];
            if value < left.threshold || value > right.threshold {
                continue;
            }

            let span = right.threshold - left.threshold;
            if span.abs() <= f32::EPSILON {
                return vec![(right.binding.clone(), 1.0)];
            }

            let t = ((value - left.threshold) / span).clamp(0.0, 1.0);
            if left.binding == right.binding {
                return vec![(left.binding.clone(), 1.0)];
            }

            return vec![(left.binding.clone(), 1.0 - t), (right.binding.clone(), t)];
        }

        vec![(points[points.len() - 1].binding.clone(), 1.0)]
    }

    fn validate(
        &self,
        state: &CharacterStateId,
    ) -> Result<(), CharacterStateMachineValidationError> {
        if self.points.is_empty() {
            return Err(CharacterStateMachineValidationError::InvalidBlendTree {
                state: state.clone(),
                reason: "blend tree must declare at least one point".into(),
            });
        }

        if self
            .points
            .iter()
            .any(|point| !point.threshold.is_finite() || point.binding.0.is_empty())
        {
            return Err(CharacterStateMachineValidationError::InvalidBlendTree {
                state: state.clone(),
                reason: "blend tree points must use finite thresholds and non-empty bindings"
                    .into(),
            });
        }

        Ok(())
    }
}

impl BlendDefinition {
    pub fn new(duration_seconds: f32) -> Self {
        Self {
            duration_seconds,
            ..default()
        }
    }

    pub fn with_easing(mut self, easing: BlendEasing) -> Self {
        self.easing = easing;
        self
    }

    pub fn without_reset(mut self) -> Self {
        self.reset_on_entry = false;
        self
    }

    pub fn sync_to_source_time(mut self) -> Self {
        self.sync_to_source_time = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct NormalizedTimeWindow {
    pub start: f32,
    pub end: f32,
}

impl NormalizedTimeWindow {
    pub fn new(start: f32, end: f32) -> Self {
        Self { start, end }
    }

    pub fn contains(&self, value: f32) -> bool {
        value >= self.start && value <= self.end
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum TransitionSource {
    Current,
    Any,
    State(CharacterStateId),
}

impl From<CharacterStateId> for TransitionSource {
    fn from(value: CharacterStateId) -> Self {
        Self::State(value)
    }
}

impl From<&str> for TransitionSource {
    fn from(value: &str) -> Self {
        Self::State(value.into())
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum TransitionCondition {
    Always,
    Grounded(bool),
    WallContact(bool),
    AnimationLocked(bool),
    SpeedAtLeast(f32),
    SpeedAtMost(f32),
    VerticalVelocityAtLeast(f32),
    VerticalVelocityAtMost(f32),
    StateTimeAtLeast(f32),
    StateTimeAtMost(f32),
    NormalizedTimeAtLeast(f32),
    NormalizedTimeAtMost(f32),
    ExitWindowOpen,
    AnimationFinished,
    LocomotionMode(LocomotionMode),
    ActionRequested(CharacterActionId),
    InhibitFlagPresent(String),
    InhibitFlagMissing(String),
}

#[derive(Clone, Debug, Default, PartialEq, Reflect)]
pub struct TransitionGuard {
    pub all: Vec<TransitionCondition>,
    pub any: Vec<TransitionCondition>,
    pub none: Vec<TransitionCondition>,
}

impl TransitionGuard {
    pub fn all(conditions: impl IntoIterator<Item = TransitionCondition>) -> Self {
        Self {
            all: conditions.into_iter().collect(),
            ..default()
        }
    }

    pub fn any(conditions: impl IntoIterator<Item = TransitionCondition>) -> Self {
        Self {
            any: conditions.into_iter().collect(),
            ..default()
        }
    }

    pub fn with_all(mut self, condition: TransitionCondition) -> Self {
        self.all.push(condition);
        self
    }

    pub fn with_any(mut self, condition: TransitionCondition) -> Self {
        self.any.push(condition);
        self
    }

    pub fn with_none(mut self, condition: TransitionCondition) -> Self {
        self.none.push(condition);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct StateDefinition {
    pub id: CharacterStateId,
    pub parent: Option<CharacterStateId>,
    pub kind: StateKind,
    pub binding: Option<CharacterAnimationBindingId>,
    pub blend_tree_1d: Option<BlendTree1D>,
    pub expected_duration_seconds: Option<f32>,
    pub minimum_duration_seconds: f32,
    pub interruptible: bool,
    pub resume_policy: ResumePolicy,
}

impl StateDefinition {
    pub fn new(id: impl Into<CharacterStateId>) -> Self {
        Self {
            id: id.into(),
            parent: None,
            kind: StateKind::Persistent,
            binding: None,
            blend_tree_1d: None,
            expected_duration_seconds: None,
            minimum_duration_seconds: 0.0,
            interruptible: true,
            resume_policy: ResumePolicy::PreserveTime,
        }
    }

    pub fn with_parent(mut self, parent: impl Into<CharacterStateId>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    pub fn with_binding(mut self, binding: impl Into<CharacterAnimationBindingId>) -> Self {
        self.binding = Some(binding.into());
        self
    }

    pub fn with_blend_tree_1d(mut self, blend_tree_1d: BlendTree1D) -> Self {
        self.blend_tree_1d = Some(blend_tree_1d);
        self
    }

    pub fn with_expected_duration(mut self, duration_seconds: f32) -> Self {
        self.expected_duration_seconds = Some(duration_seconds);
        self
    }

    pub fn with_minimum_duration(mut self, duration_seconds: f32) -> Self {
        self.minimum_duration_seconds = duration_seconds;
        self
    }

    pub fn transient(mut self) -> Self {
        self.kind = StateKind::Transient;
        self
    }

    pub fn non_interruptible(mut self) -> Self {
        self.interruptible = false;
        self
    }

    pub fn with_resume_policy(mut self, policy: ResumePolicy) -> Self {
        self.resume_policy = policy;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct TransitionDefinition {
    pub id: CharacterTransitionId,
    pub source: TransitionSource,
    pub target: Option<CharacterStateId>,
    pub operation: TransitionOperation,
    pub guard: TransitionGuard,
    pub priority: i32,
    pub minimum_source_duration_seconds: Option<f32>,
    pub exit_window: Option<NormalizedTimeWindow>,
    pub blend: Option<BlendDefinition>,
    pub allow_self_transition: bool,
    pub force_interrupt: bool,
    pub push_conflict_policy: PushConflictPolicy,
}

impl TransitionDefinition {
    pub fn switch(
        id: impl Into<CharacterTransitionId>,
        source: impl Into<TransitionSource>,
        target: impl Into<CharacterStateId>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            target: Some(target.into()),
            operation: TransitionOperation::Set,
            guard: TransitionGuard::default(),
            priority: 0,
            minimum_source_duration_seconds: None,
            exit_window: None,
            blend: None,
            allow_self_transition: false,
            force_interrupt: false,
            push_conflict_policy: PushConflictPolicy::Stack,
        }
    }

    pub fn push(
        id: impl Into<CharacterTransitionId>,
        source: impl Into<TransitionSource>,
        target: impl Into<CharacterStateId>,
    ) -> Self {
        Self {
            operation: TransitionOperation::Push,
            ..Self::switch(id, source, target)
        }
    }

    pub fn pop(id: impl Into<CharacterTransitionId>, source: impl Into<TransitionSource>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            target: None,
            operation: TransitionOperation::Pop,
            guard: TransitionGuard::default(),
            priority: 0,
            minimum_source_duration_seconds: None,
            exit_window: None,
            blend: None,
            allow_self_transition: false,
            force_interrupt: false,
            push_conflict_policy: PushConflictPolicy::Stack,
        }
    }

    pub fn with_guard(mut self, guard: TransitionGuard) -> Self {
        self.guard = guard;
        self
    }

    pub fn when(mut self, condition: TransitionCondition) -> Self {
        self.guard.all.push(condition);
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_minimum_source_duration(mut self, duration_seconds: f32) -> Self {
        self.minimum_source_duration_seconds = Some(duration_seconds);
        self
    }

    pub fn with_exit_window(mut self, start: f32, end: f32) -> Self {
        self.exit_window = Some(NormalizedTimeWindow::new(start, end));
        self
    }

    pub fn with_blend(mut self, blend: BlendDefinition) -> Self {
        self.blend = Some(blend);
        self
    }

    pub fn allow_self_transition(mut self) -> Self {
        self.allow_self_transition = true;
        self
    }

    pub fn force_interrupt(mut self) -> Self {
        self.force_interrupt = true;
        self
    }

    pub fn with_push_conflict_policy(mut self, policy: PushConflictPolicy) -> Self {
        self.push_conflict_policy = policy;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CharacterStateMachineDefinition {
    pub id: CharacterStateMachineDefinitionId,
    pub initial_state: CharacterStateId,
    pub fallback_state: Option<CharacterStateId>,
    pub states: Vec<StateDefinition>,
    pub transitions: Vec<TransitionDefinition>,
    pub default_blend: BlendDefinition,
}

impl CharacterStateMachineDefinition {
    pub fn new(
        id: impl Into<CharacterStateMachineDefinitionId>,
        initial_state: impl Into<CharacterStateId>,
    ) -> Self {
        Self {
            id: id.into(),
            initial_state: initial_state.into(),
            fallback_state: None,
            states: Vec::new(),
            transitions: Vec::new(),
            default_blend: BlendDefinition::default(),
        }
    }

    pub fn with_fallback_state(mut self, state: impl Into<CharacterStateId>) -> Self {
        self.fallback_state = Some(state.into());
        self
    }

    pub fn with_default_blend(mut self, blend: BlendDefinition) -> Self {
        self.default_blend = blend;
        self
    }

    pub fn add_state(mut self, state: StateDefinition) -> Self {
        self.states.push(state);
        self
    }

    pub fn add_transition(mut self, transition: TransitionDefinition) -> Self {
        self.transitions.push(transition);
        self
    }

    pub fn state(&self, id: &CharacterStateId) -> Option<&StateDefinition> {
        self.states.iter().find(|state| &state.id == id)
    }

    pub fn parent_chain<'a>(&'a self, state_id: &CharacterStateId) -> Vec<&'a StateDefinition> {
        let mut chain = Vec::new();
        let mut current = self.state(state_id);
        while let Some(state) = current {
            chain.push(state);
            current = state.parent.as_ref().and_then(|parent| self.state(parent));
        }
        chain
    }

    pub fn resolve_binding(
        &self,
        state_id: &CharacterStateId,
    ) -> (Option<CharacterAnimationBindingId>, bool) {
        if let Some(binding) = self.state(state_id).and_then(|state| state.binding.clone()) {
            return (Some(binding), false);
        }

        let fallback_binding = self
            .fallback_state
            .as_ref()
            .and_then(|fallback| self.state(fallback))
            .and_then(|state| state.binding.clone());

        (fallback_binding, true)
    }

    pub fn validate(&self) -> Result<(), CharacterStateMachineValidationError> {
        if self.states.is_empty() {
            return Err(CharacterStateMachineValidationError::NoStates);
        }

        let mut states = HashMap::new();
        for state in &self.states {
            if states.insert(state.id.clone(), ()).is_some() {
                return Err(CharacterStateMachineValidationError::DuplicateState(
                    state.id.clone(),
                ));
            }

            if let Some(blend_tree) = &state.blend_tree_1d {
                blend_tree.validate(&state.id)?;
            }

            if let Some(parent) = &state.parent
                && self.state(parent).is_none()
            {
                return Err(CharacterStateMachineValidationError::MissingParent {
                    state: state.id.clone(),
                    parent: parent.clone(),
                });
            }
        }

        if self.state(&self.initial_state).is_none() {
            return Err(CharacterStateMachineValidationError::MissingInitialState(
                self.initial_state.clone(),
            ));
        }

        if let Some(fallback) = &self.fallback_state
            && self.state(fallback).is_none()
        {
            return Err(CharacterStateMachineValidationError::MissingFallbackState(
                fallback.clone(),
            ));
        }

        let mut transitions = HashMap::new();
        for transition in &self.transitions {
            if transitions.insert(transition.id.clone(), ()).is_some() {
                return Err(CharacterStateMachineValidationError::DuplicateTransition(
                    transition.id.clone(),
                ));
            }

            if let TransitionSource::State(source) = &transition.source
                && self.state(source).is_none()
            {
                return Err(
                    CharacterStateMachineValidationError::MissingTransitionSource {
                        transition: transition.id.clone(),
                        source: source.clone(),
                    },
                );
            }

            if let Some(target) = &transition.target
                && self.state(target).is_none()
            {
                return Err(
                    CharacterStateMachineValidationError::MissingTransitionTarget {
                        transition: transition.id.clone(),
                        target: target.clone(),
                    },
                );
            }
        }

        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct CharacterStateMachineLibrary {
    definitions: HashMap<CharacterStateMachineDefinitionId, CharacterStateMachineDefinition>,
}

impl CharacterStateMachineLibrary {
    pub fn register(
        &mut self,
        definition: CharacterStateMachineDefinition,
    ) -> Result<CharacterStateMachineDefinitionId, CharacterStateMachineValidationError> {
        definition.validate()?;
        let definition_id = definition.id.clone();
        self.definitions.insert(definition_id.clone(), definition);
        Ok(definition_id)
    }

    pub fn get(
        &self,
        definition_id: &CharacterStateMachineDefinitionId,
    ) -> Option<&CharacterStateMachineDefinition> {
        self.definitions.get(definition_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharacterStateMachineValidationError {
    NoStates,
    DuplicateState(CharacterStateId),
    DuplicateTransition(CharacterTransitionId),
    InvalidBlendTree {
        state: CharacterStateId,
        reason: String,
    },
    MissingParent {
        state: CharacterStateId,
        parent: CharacterStateId,
    },
    MissingInitialState(CharacterStateId),
    MissingFallbackState(CharacterStateId),
    MissingTransitionSource {
        transition: CharacterTransitionId,
        source: CharacterStateId,
    },
    MissingTransitionTarget {
        transition: CharacterTransitionId,
        target: CharacterStateId,
    },
}
