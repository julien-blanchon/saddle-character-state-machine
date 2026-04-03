use super::*;

#[test]
fn layer_builder_helpers_keep_expected_flags() {
    let layer = CharacterAnimationLayer::new("UpperBody", "aim")
        .with_weight(0.45)
        .additive()
        .sync_to_base_time();

    assert_eq!(layer.name, "UpperBody");
    assert_eq!(layer.binding.0, "aim");
    assert!((layer.weight - 0.45).abs() < 0.001);
    assert_eq!(layer.mode, CharacterAnimationLayerMode::Additive);
    assert!(layer.sync_to_base_time);
    assert!(layer.enabled);
}
