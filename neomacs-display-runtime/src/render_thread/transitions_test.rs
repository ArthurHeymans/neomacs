use super::*;
use neomacs_display_protocol::{TransitionEasing, TransitionEffect};
use std::time::Duration;

#[test]
fn default_transition_state_has_expected_policy_defaults() {
    let ts = TransitionState::default();
    assert!(ts.policy.buffer.enabled);
    // Scroll transition is off by default (stock-Emacs-like instant scroll);
    // opt in via `(neomacs-effect-set 'scroll-transition :enabled t)`.
    assert!(!ts.policy.scroll.enabled);
    assert_eq!(ts.policy.buffer.duration, Duration::from_millis(200));
    assert_eq!(ts.policy.scroll.duration, Duration::from_millis(150));
    assert_eq!(ts.policy.buffer.effect, TransitionEffect::Slide);
    assert_eq!(ts.policy.scroll.effect, TransitionEffect::Slide);
}

#[test]
fn visual_config_is_the_transition_policy_source_of_truth() {
    let mut config = neomacs_display_protocol::VisualConfig::default();
    config.buffer_transition.enabled = false;
    config.scroll_transition.duration = Duration::from_millis(425);
    config.scroll_transition.effect = TransitionEffect::PageCurl;
    config.scroll_transition.easing = TransitionEasing::Spring;

    let policy = TransitionPolicy::from(&config);

    assert!(!policy.buffer.enabled);
    assert_eq!(policy.scroll.duration, Duration::from_millis(425));
    assert_eq!(policy.scroll.effect, TransitionEffect::PageCurl);
    assert_eq!(policy.scroll.easing, TransitionEasing::Spring);
}

#[test]
fn default_transition_state_starts_without_active_transitions() {
    let ts = TransitionState::default();
    assert!(ts.offscreen_a.is_none());
    assert!(ts.offscreen_b.is_none());
    assert!(ts.current_is_a);
    assert!(!ts.has_active());
}
