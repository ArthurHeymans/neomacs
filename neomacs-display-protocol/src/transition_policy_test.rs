use super::*;
use crate::{
    DirectionlessTransitionEffect, ResolvedTransitionEffect, TransitionAxis,
    TransitionAxisPreference, TransitionDirection, TransitionEdge, TransitionEffect,
    VerticalPostProcessTransitionEffect, VerticalTransitionEffect, types::Rect,
};

#[test]
fn buffer_slide_auto_orientation_resolves_to_horizontal_window_motion() {
    let mut config = VisualConfig::default();
    config.buffer_transition.effect = TransitionEffect::Slide;
    let policy = TransitionPolicy::from(&config);

    let plan = policy
        .buffer_plan(Rect::new(10.0, 20.0, 800.0, 600.0))
        .expect("default buffer transitions are enabled");

    assert_eq!(plan.duration, std::time::Duration::from_millis(200));
    assert!(matches!(
        plan.effect,
        ResolvedTransitionEffect::AxisMotion {
            axis: TransitionAxis::Horizontal,
            direction: TransitionDirection::Forward,
            distance: 800.0,
            ..
        }
    ));
}

#[test]
fn directionless_effects_do_not_carry_an_axis_or_direction() {
    let mut config = VisualConfig::default();
    config.buffer_transition.effect = TransitionEffect::Crossfade;
    let policy = TransitionPolicy::from(&config);

    let plan = policy
        .buffer_plan(Rect::new(0.0, 0.0, 800.0, 600.0))
        .expect("default buffer transitions are enabled");

    assert_eq!(
        plan.effect,
        ResolvedTransitionEffect::Directionless(DirectionlessTransitionEffect::Crossfade)
    );
}

#[test]
fn an_explicit_vertical_buffer_slide_spans_the_window_height() {
    let mut config = VisualConfig::default();
    config.buffer_transition.effect = TransitionEffect::Slide;
    config.buffer_transition.axis = TransitionAxisPreference::Vertical;
    config.buffer_transition.direction = TransitionDirection::Backward;
    let policy = TransitionPolicy::from(&config);

    let plan = policy
        .buffer_plan(Rect::new(10.0, 20.0, 800.0, 600.0))
        .expect("buffer transitions are enabled");

    assert!(matches!(
        plan.effect,
        ResolvedTransitionEffect::AxisMotion {
            axis: TransitionAxis::Vertical,
            direction: TransitionDirection::Backward,
            distance: 600.0,
            ..
        }
    ));
}

#[test]
fn intrinsic_vertical_buffer_effects_ignore_an_incompatible_axis_preference() {
    let mut config = VisualConfig::default();
    config.buffer_transition.effect = TransitionEffect::Cascade;
    config.buffer_transition.axis = TransitionAxisPreference::Horizontal;
    let policy = TransitionPolicy::from(&config);

    let plan = policy
        .buffer_plan(Rect::new(10.0, 20.0, 800.0, 600.0))
        .expect("buffer transitions are enabled");

    assert_eq!(
        plan.effect,
        ResolvedTransitionEffect::Vertical {
            effect: VerticalTransitionEffect::Cascade,
            direction: TransitionDirection::Forward,
            distance: 600.0,
        }
    );
}

#[test]
fn page_curl_resolves_axis_and_direction_to_a_concrete_edge() {
    let mut config = VisualConfig::default();
    config.buffer_transition.effect = TransitionEffect::PageCurl;
    config.buffer_transition.axis = TransitionAxisPreference::Vertical;
    config.buffer_transition.direction = TransitionDirection::Backward;
    let policy = TransitionPolicy::from(&config);

    let plan = policy
        .buffer_plan(Rect::new(10.0, 20.0, 800.0, 600.0))
        .expect("buffer transitions are enabled");

    assert_eq!(
        plan.effect,
        ResolvedTransitionEffect::PageCurl {
            edge: TransitionEdge::Top,
        }
    );
}

#[test]
fn viewport_effects_use_the_actual_scroll_distance() {
    let mut config = VisualConfig::default();
    config.scroll_transition.enabled = true;
    config.scroll_transition.effect = TransitionEffect::Cascade;
    let policy = TransitionPolicy::from(&config);

    let plan = policy
        .scroll_plan(
            Rect::new(10.0, 20.0, 800.0, 600.0),
            TransitionDirection::Backward,
            42.0,
        )
        .expect("scroll transitions were enabled");

    assert_eq!(
        plan.effect,
        ResolvedTransitionEffect::Vertical {
            effect: VerticalTransitionEffect::Cascade,
            direction: TransitionDirection::Backward,
            distance: 42.0,
        }
    );
}

#[test]
fn renderer_plan_owns_the_bounds_used_to_resolve_its_geometry() {
    let bounds = Rect::new(10.0, 20.0, 800.0, 600.0);
    let plan = TransitionPolicy::default()
        .buffer_plan(bounds)
        .expect("default buffer transitions are enabled");

    assert_eq!(plan.bounds, bounds);
}

#[test]
fn card_flip_carries_only_the_axis_consumed_by_the_renderer() {
    let mut config = VisualConfig::default();
    config.buffer_transition.effect = TransitionEffect::CardFlip;
    config.buffer_transition.axis = TransitionAxisPreference::Vertical;

    let plan = TransitionPolicy::from(&config)
        .buffer_plan(Rect::new(10.0, 20.0, 800.0, 600.0))
        .expect("buffer transitions are enabled");

    assert_eq!(
        plan.effect,
        ResolvedTransitionEffect::CardFlip {
            axis: TransitionAxis::Vertical,
        }
    );
}

#[test]
fn post_process_effects_have_a_renderer_safe_typed_family() {
    let mut config = VisualConfig::default();
    config.buffer_transition.effect = TransitionEffect::MotionBlur;
    let policy = TransitionPolicy::from(&config);

    let plan = policy
        .buffer_plan(Rect::new(10.0, 20.0, 800.0, 600.0))
        .expect("buffer transitions are enabled");

    assert_eq!(
        plan.effect,
        ResolvedTransitionEffect::VerticalPostProcess {
            effect: VerticalPostProcessTransitionEffect::MotionBlur,
            direction: TransitionDirection::Forward,
            distance: 600.0,
        }
    );
}
