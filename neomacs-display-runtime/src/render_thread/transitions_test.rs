use super::*;
use std::time::Duration;

#[test]
fn default_transition_state_has_expected_policy_defaults() {
    let ts = TransitionState::default();
    assert!(ts.policy.crossfade.enabled);
    assert!(ts.policy.scroll.enabled);
    assert_eq!(ts.policy.crossfade.duration, Duration::from_millis(200));
    assert_eq!(ts.policy.scroll.duration, Duration::from_millis(150));
    assert_eq!(ts.policy.crossfade.effect, ScrollEffect::Crossfade);
    assert_eq!(ts.policy.scroll.effect, ScrollEffect::Slide);
}

#[test]
fn visual_config_is_the_transition_policy_source_of_truth() {
    let mut config = neomacs_display_protocol::VisualConfig::default();
    config.crossfade_transition.enabled = false;
    config.scroll_transition.duration = Duration::from_millis(425);
    config.scroll_transition.effect = ScrollEffect::PageCurl;
    config.scroll_transition.easing = ScrollEasing::Spring;

    let policy = TransitionPolicy::from(&config);

    assert!(!policy.crossfade.enabled);
    assert_eq!(policy.scroll.duration, Duration::from_millis(425));
    assert_eq!(policy.scroll.effect, ScrollEffect::PageCurl);
    assert_eq!(policy.scroll.easing, ScrollEasing::Spring);
}

#[test]
fn default_transition_state_starts_without_active_transitions() {
    let ts = TransitionState::default();
    assert!(ts.offscreen_a.is_none());
    assert!(ts.offscreen_b.is_none());
    assert!(ts.current_is_a);
    assert!(!ts.has_active());
}
