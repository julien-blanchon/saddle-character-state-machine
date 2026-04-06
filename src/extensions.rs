use bevy::prelude::*;

use crate::CharacterAnimationFacts;

pub mod keys {
    pub const SPEED: &str = "speed";
    pub const GROUNDED: &str = "grounded";
    pub const WALL_CONTACT: &str = "wall_contact";
    pub const VERTICAL_VELOCITY: &str = "vertical_velocity";
    pub const MOVEMENT_DIRECTION: &str = "movement_direction";
    pub const FACING_DIRECTION: &str = "facing_direction";
    pub const AIM_DIRECTION: &str = "aim_direction";
    pub const ANIMATION_LOCKED: &str = "animation_locked";
    pub const LOCOMOTION_INTENSITY: &str = "locomotion_intensity";
}

pub mod conditions {
    use crate::TransitionCondition;

    use super::keys;

    pub fn grounded(value: bool) -> TransitionCondition {
        TransitionCondition::bool_is(keys::GROUNDED, value)
    }

    pub fn wall_contact(value: bool) -> TransitionCondition {
        TransitionCondition::bool_is(keys::WALL_CONTACT, value)
    }

    pub fn animation_locked(value: bool) -> TransitionCondition {
        TransitionCondition::bool_is(keys::ANIMATION_LOCKED, value)
    }

    pub fn speed_at_least(value: f32) -> TransitionCondition {
        TransitionCondition::number_at_least(keys::SPEED, value)
    }

    pub fn speed_at_most(value: f32) -> TransitionCondition {
        TransitionCondition::number_at_most(keys::SPEED, value)
    }

    pub fn vertical_velocity_at_least(value: f32) -> TransitionCondition {
        TransitionCondition::number_at_least(keys::VERTICAL_VELOCITY, value)
    }

    pub fn vertical_velocity_at_most(value: f32) -> TransitionCondition {
        TransitionCondition::number_at_most(keys::VERTICAL_VELOCITY, value)
    }
}

pub mod parameters {
    use crate::BlendTreeParameter;

    use super::keys;

    pub fn speed() -> BlendTreeParameter {
        BlendTreeParameter::number(keys::SPEED)
    }

    pub fn locomotion_intensity() -> BlendTreeParameter {
        BlendTreeParameter::number(keys::LOCOMOTION_INTENSITY)
    }

    pub fn vertical_velocity() -> BlendTreeParameter {
        BlendTreeParameter::number(keys::VERTICAL_VELOCITY)
    }

    pub fn movement_direction_x() -> BlendTreeParameter {
        BlendTreeParameter::vec2_x(keys::MOVEMENT_DIRECTION)
    }

    pub fn movement_direction_y() -> BlendTreeParameter {
        BlendTreeParameter::vec2_y(keys::MOVEMENT_DIRECTION)
    }

    pub fn movement_direction_length() -> BlendTreeParameter {
        BlendTreeParameter::vec2_length(keys::MOVEMENT_DIRECTION)
    }

    pub fn facing_direction_x() -> BlendTreeParameter {
        BlendTreeParameter::vec2_x(keys::FACING_DIRECTION)
    }

    pub fn aim_direction_x() -> BlendTreeParameter {
        BlendTreeParameter::vec2_x(keys::AIM_DIRECTION)
    }
}

pub trait CharacterAnimationFactsExt {
    fn speed(&self) -> f32;
    fn set_speed(&mut self, value: f32);

    fn grounded(&self) -> bool;
    fn set_grounded(&mut self, value: bool);

    fn wall_contact(&self) -> bool;
    fn set_wall_contact(&mut self, value: bool);

    fn vertical_velocity(&self) -> f32;
    fn set_vertical_velocity(&mut self, value: f32);

    fn movement_direction(&self) -> Vec2;
    fn set_movement_direction(&mut self, value: Vec2);

    fn facing_direction(&self) -> Vec2;
    fn set_facing_direction(&mut self, value: Vec2);

    fn aim_direction(&self) -> Vec2;
    fn set_aim_direction(&mut self, value: Vec2);

    fn animation_locked(&self) -> bool;
    fn set_animation_locked(&mut self, value: bool);

    fn locomotion_intensity(&self) -> f32;
    fn set_locomotion_intensity(&mut self, value: f32);
}

impl CharacterAnimationFactsExt for CharacterAnimationFacts {
    fn speed(&self) -> f32 {
        self.number_or(keys::SPEED, 0.0)
    }

    fn set_speed(&mut self, value: f32) {
        self.set_number(keys::SPEED, value);
    }

    fn grounded(&self) -> bool {
        self.boolean_or(keys::GROUNDED, false)
    }

    fn set_grounded(&mut self, value: bool) {
        self.set_boolean(keys::GROUNDED, value);
    }

    fn wall_contact(&self) -> bool {
        self.boolean_or(keys::WALL_CONTACT, false)
    }

    fn set_wall_contact(&mut self, value: bool) {
        self.set_boolean(keys::WALL_CONTACT, value);
    }

    fn vertical_velocity(&self) -> f32 {
        self.number_or(keys::VERTICAL_VELOCITY, 0.0)
    }

    fn set_vertical_velocity(&mut self, value: f32) {
        self.set_number(keys::VERTICAL_VELOCITY, value);
    }

    fn movement_direction(&self) -> Vec2 {
        self.vec2_or(keys::MOVEMENT_DIRECTION, Vec2::ZERO)
    }

    fn set_movement_direction(&mut self, value: Vec2) {
        self.set_vec2(keys::MOVEMENT_DIRECTION, value);
    }

    fn facing_direction(&self) -> Vec2 {
        self.vec2_or(keys::FACING_DIRECTION, Vec2::ZERO)
    }

    fn set_facing_direction(&mut self, value: Vec2) {
        self.set_vec2(keys::FACING_DIRECTION, value);
    }

    fn aim_direction(&self) -> Vec2 {
        self.vec2_or(keys::AIM_DIRECTION, Vec2::ZERO)
    }

    fn set_aim_direction(&mut self, value: Vec2) {
        self.set_vec2(keys::AIM_DIRECTION, value);
    }

    fn animation_locked(&self) -> bool {
        self.boolean_or(keys::ANIMATION_LOCKED, false)
    }

    fn set_animation_locked(&mut self, value: bool) {
        self.set_boolean(keys::ANIMATION_LOCKED, value);
    }

    fn locomotion_intensity(&self) -> f32 {
        self.number_or(keys::LOCOMOTION_INTENSITY, 0.0)
    }

    fn set_locomotion_intensity(&mut self, value: f32) {
        self.set_number(keys::LOCOMOTION_INTENSITY, value);
    }
}

#[cfg(test)]
#[path = "extensions_tests.rs"]
mod tests;
