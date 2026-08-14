//! Motion-policy gating of the visual configuration (Task 4 of the
//! animation best-practices plan).

use crate::{EffectsConfig, MotionPolicy, TransitionPolicy, VisualConfig};

fn visual_config_with_motion(motion: MotionPolicy) -> VisualConfig {
    VisualConfig {
        motion,
        ..VisualConfig::default()
    }
}

#[test]
fn reduced_motion_disables_both_transition_paths() {
    let reduced = TransitionPolicy::from(&visual_config_with_motion(MotionPolicy::Reduced));
    assert!(!reduced.crossfade.enabled, "crossfade must cut, not fade");
    assert!(!reduced.scroll.enabled, "scroll slide must cut");
    assert!(!reduced.needs_offscreen(), "no offscreen snapshots needed");

    let full = TransitionPolicy::from(&visual_config_with_motion(MotionPolicy::Full));
    assert_eq!(
        full.crossfade.enabled,
        VisualConfig::default().crossfade_transition.enabled
    );
    assert_eq!(
        full.scroll.enabled,
        VisualConfig::default().scroll_transition.enabled
    );
}

#[test]
fn reduced_motion_disables_every_effect_with_an_enabled_property() {
    let effects = EffectsConfig::default().with_motion_policy(MotionPolicy::Reduced);
    for name in effects.effect_names() {
        if let Ok(values) = effects.effect_values(&name) {
            for (key, value) in values {
                if key == "enabled" {
                    assert_eq!(
                        value,
                        crate::EffectValue::Bool(false),
                        "effect {name} must start disabled under Reduced motion"
                    );
                }
            }
        }
    }
}

#[test]
fn full_motion_keeps_the_effect_catalog_untouched() {
    let original = EffectsConfig::default();
    let gated = original.with_motion_policy(MotionPolicy::Full);
    assert_eq!(original, gated);
}

#[test]
fn motion_policy_reports_reduced() {
    assert!(!MotionPolicy::Full.is_reduced());
    assert!(MotionPolicy::Reduced.is_reduced());
}
