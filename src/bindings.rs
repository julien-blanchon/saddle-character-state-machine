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

#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
pub enum CharacterAnimationBindingSlot {
    Base,
    Layer(String),
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CharacterAnimationActiveBinding {
    pub slot: CharacterAnimationBindingSlot,
    pub binding: CharacterAnimationBindingId,
    pub weight: f32,
    pub sync_normalized_time: Option<f32>,
}

impl CharacterAnimationActiveBinding {
    pub fn base(
        binding: impl Into<CharacterAnimationBindingId>,
        weight: f32,
        sync_normalized_time: Option<f32>,
    ) -> Self {
        Self {
            slot: CharacterAnimationBindingSlot::Base,
            binding: binding.into(),
            weight,
            sync_normalized_time,
        }
    }

    pub fn layer(
        name: impl Into<String>,
        binding: impl Into<CharacterAnimationBindingId>,
        weight: f32,
        sync_normalized_time: Option<f32>,
    ) -> Self {
        Self {
            slot: CharacterAnimationBindingSlot::Layer(name.into()),
            binding: binding.into(),
            weight,
            sync_normalized_time,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum CharacterAnimationLayerMode {
    #[default]
    Override,
    Additive,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CharacterAnimationLayer {
    pub name: String,
    pub enabled: bool,
    pub binding: CharacterAnimationBindingId,
    pub weight: f32,
    pub mode: CharacterAnimationLayerMode,
    pub sync_to_base_time: bool,
}

impl CharacterAnimationLayer {
    pub fn new(name: impl Into<String>, binding: impl Into<CharacterAnimationBindingId>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            binding: binding.into(),
            weight: 1.0,
            mode: CharacterAnimationLayerMode::Override,
            sync_to_base_time: false,
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn additive(mut self) -> Self {
        self.mode = CharacterAnimationLayerMode::Additive;
        self
    }

    pub fn sync_to_base_time(mut self) -> Self {
        self.sync_to_base_time = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CharacterAnimationLayers {
    pub layers: Vec<CharacterAnimationLayer>,
}

impl CharacterAnimationLayers {
    pub fn push(&mut self, layer: CharacterAnimationLayer) {
        self.layers.push(layer);
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default)]
pub struct CharacterAnimationSelection {
    pub state: Option<CharacterStateId>,
    pub binding: Option<CharacterAnimationBindingId>,
    pub active_bindings: Vec<CharacterAnimationActiveBinding>,
    pub blend: BlendDefinition,
    pub sync_normalized_time: Option<f32>,
    pub generation: u64,
}

#[cfg(test)]
#[path = "bindings_tests.rs"]
mod tests;
