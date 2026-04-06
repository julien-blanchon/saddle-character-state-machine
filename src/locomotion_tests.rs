use crate::{
    BlendTreeParameter, CharacterAnimationFacts, TransitionCondition,
    extensions::{CharacterAnimationFactsExt, keys},
    locomotion::{
        CharacterAnimationFactsLocomotionExt, LocomotionMode, intensity_parameter, mode_is,
    },
};

#[test]
fn locomotion_recipe_stores_mode_as_tag_and_intensity_as_number() {
    let mut facts = CharacterAnimationFacts::default();
    facts.set_locomotion_mode(LocomotionMode::Run);
    facts.set_locomotion_intensity(0.85);

    assert_eq!(facts.locomotion_mode(), Some(LocomotionMode::Run));
    assert!((facts.locomotion_intensity() - 0.85).abs() < 0.001);
}

#[test]
fn locomotion_recipe_helpers_emit_generic_conditions() {
    assert_eq!(
        mode_is(LocomotionMode::Sprint),
        TransitionCondition::tag_present("locomotion_mode:sprint"),
    );
    assert_eq!(
        intensity_parameter(),
        BlendTreeParameter::number(keys::LOCOMOTION_INTENSITY),
    );
}
