use bevy::{animation::RepeatAnimation, prelude::*};

use crate::config::{BlendDefinition, CharacterAnimationBindingId, CharacterStateId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum PlaybackRepeat {
    #[default]
    Forever,
    Never,
    Count(u32),
}

impl PlaybackRepeat {
    pub fn to_bevy(self) -> RepeatAnimation {
        match self {
            Self::Forever => RepeatAnimation::Forever,
            Self::Never => RepeatAnimation::Never,
            Self::Count(count) => RepeatAnimation::Count(count),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct BevyAnimationBinding {
    pub binding_id: CharacterAnimationBindingId,
    pub node_index: u32,
    pub duration_seconds: Option<f32>,
    pub repeat: PlaybackRepeat,
}

impl BevyAnimationBinding {
    pub fn new(
        binding_id: impl Into<CharacterAnimationBindingId>,
        node_index: u32,
        duration_seconds: Option<f32>,
    ) -> Self {
        Self {
            binding_id: binding_id.into(),
            node_index,
            duration_seconds,
            repeat: PlaybackRepeat::Forever,
        }
    }

    pub fn with_repeat(mut self, repeat: PlaybackRepeat) -> Self {
        self.repeat = repeat;
        self
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct BevyAnimationBridge {
    pub player_entity: Option<Entity>,
    pub graph_handle: Handle<AnimationGraph>,
    pub bindings: Vec<BevyAnimationBinding>,
}

impl BevyAnimationBridge {
    pub fn new(graph_handle: Handle<AnimationGraph>) -> Self {
        Self {
            player_entity: None,
            graph_handle,
            bindings: Vec::new(),
        }
    }

    pub fn with_player_entity(mut self, player_entity: Entity) -> Self {
        self.player_entity = Some(player_entity);
        self
    }

    pub fn add_binding(mut self, binding: BevyAnimationBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    pub fn binding(
        &self,
        binding_id: &CharacterAnimationBindingId,
    ) -> Option<&BevyAnimationBinding> {
        self.bindings
            .iter()
            .find(|binding| &binding.binding_id == binding_id)
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CharacterAnimationSelection {
    pub state: Option<CharacterStateId>,
    pub binding: Option<CharacterAnimationBindingId>,
    pub blend: BlendDefinition,
    pub sync_normalized_time: Option<f32>,
    pub generation: u64,
}
