use bevy::prelude::*;

use crate::{BlendTreeParameter, CharacterAnimationFacts, TransitionCondition, extensions};

const LOCOMOTION_MODE_TAG_PREFIX: &str = "locomotion_mode:";

#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub enum LocomotionMode {
    #[default]
    Idle,
    Walk,
    Run,
    Sprint,
    Custom(String),
}

impl LocomotionMode {
    fn slug(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Walk => "walk",
            Self::Run => "run",
            Self::Sprint => "sprint",
            Self::Custom(value) => value.as_str(),
        }
    }

    fn from_slug(value: &str) -> Self {
        match value {
            "idle" => Self::Idle,
            "walk" => Self::Walk,
            "run" => Self::Run,
            "sprint" => Self::Sprint,
            other => Self::Custom(other.to_string()),
        }
    }
}

pub fn mode_tag(mode: &LocomotionMode) -> String {
    format!("{LOCOMOTION_MODE_TAG_PREFIX}{}", mode.slug())
}

pub fn mode_is(mode: LocomotionMode) -> TransitionCondition {
    TransitionCondition::tag_present(mode_tag(&mode))
}

pub fn intensity_parameter() -> BlendTreeParameter {
    extensions::parameters::locomotion_intensity()
}

pub trait CharacterAnimationFactsLocomotionExt {
    fn locomotion_mode(&self) -> Option<LocomotionMode>;
    fn set_locomotion_mode(&mut self, mode: LocomotionMode);
    fn clear_locomotion_mode(&mut self);
}

impl CharacterAnimationFactsLocomotionExt for CharacterAnimationFacts {
    fn locomotion_mode(&self) -> Option<LocomotionMode> {
        self.tags
            .iter()
            .find_map(|tag| tag.0.strip_prefix(LOCOMOTION_MODE_TAG_PREFIX))
            .map(LocomotionMode::from_slug)
    }

    fn set_locomotion_mode(&mut self, mode: LocomotionMode) {
        self.clear_locomotion_mode();
        self.insert_tag(mode_tag(&mode));
    }

    fn clear_locomotion_mode(&mut self) {
        self.clear_tags_with_prefix(LOCOMOTION_MODE_TAG_PREFIX);
    }
}

#[cfg(test)]
#[path = "locomotion_tests.rs"]
mod tests;
