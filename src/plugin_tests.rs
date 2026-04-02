use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

use crate::CharacterStateMachinePlugin;

#[test]
fn always_on_constructor_uses_post_startup_activation() {
    let plugin = CharacterStateMachinePlugin::always_on(Update);
    let update = Update.intern();
    assert_eq!(plugin.update_schedule, update);
}
