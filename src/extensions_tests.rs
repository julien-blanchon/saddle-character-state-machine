use bevy::prelude::*;

use crate::{
    CharacterAnimationFacts,
    extensions::{CharacterAnimationFactsExt, conditions, keys, parameters},
};

#[test]
fn extension_trait_reads_and_writes_common_fact_keys() {
    let mut facts = CharacterAnimationFacts::default();

    facts.set_speed(1.2);
    facts.set_grounded(true);
    facts.set_wall_contact(true);
    facts.set_vertical_velocity(-3.5);
    facts.set_movement_direction(Vec2::new(1.0, 0.5));
    facts.set_facing_direction(Vec2::X);
    facts.set_aim_direction(Vec2::Y);
    facts.set_animation_locked(true);
    facts.set_locomotion_intensity(0.7);

    assert!((facts.speed() - 1.2).abs() < 0.001);
    assert!(facts.grounded());
    assert!(facts.wall_contact());
    assert!((facts.vertical_velocity() + 3.5).abs() < 0.001);
    assert_eq!(facts.movement_direction(), Vec2::new(1.0, 0.5));
    assert_eq!(facts.facing_direction(), Vec2::X);
    assert_eq!(facts.aim_direction(), Vec2::Y);
    assert!(facts.animation_locked());
    assert!((facts.locomotion_intensity() - 0.7).abs() < 0.001);
}

#[test]
fn extension_constructor_helpers_target_expected_keys() {
    assert_eq!(
        conditions::speed_at_least(0.25),
        crate::TransitionCondition::number_at_least(keys::SPEED, 0.25)
    );
    assert_eq!(
        conditions::grounded(true),
        crate::TransitionCondition::bool_is(keys::GROUNDED, true)
    );
    assert_eq!(
        parameters::movement_direction_x(),
        crate::BlendTreeParameter::vec2_x(keys::MOVEMENT_DIRECTION)
    );
    assert_eq!(
        parameters::locomotion_intensity(),
        crate::BlendTreeParameter::number(keys::LOCOMOTION_INTENSITY)
    );
}
