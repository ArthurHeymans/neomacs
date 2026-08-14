//! Complete user-facing visual configuration snapshot.
//!
//! Shader effects, cursor behavior, and window transitions share one control
//! plane even though the renderer stores and executes them in different
//! subsystems.  Keeping that distinction behind `VisualConfig` gives Elisp a
//! single named, typed, atomic interface without flattening the runtime model.

use crate::{
    CursorAnimStyle, EffectOperation, EffectValue, EffectsConfig, ScrollEasing, ScrollEffect,
};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CursorBlinkConfig {
    pub enabled: bool,
    pub interval: Duration,
}

impl Default for CursorBlinkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_millis(500),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CursorMotionConfig {
    pub enabled: bool,
    pub speed: f32,
    pub style: CursorAnimStyle,
    pub duration: Duration,
    pub trail_size: f32,
}

impl Default for CursorMotionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            speed: 2.4,
            style: CursorAnimStyle::CriticallyDampedSpring,
            duration: Duration::from_millis(150),
            trail_size: 0.7,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CursorSizeTransitionConfig {
    pub enabled: bool,
    pub duration: Duration,
}

impl Default for CursorSizeTransitionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            duration: Duration::from_millis(150),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WindowTransitionConfig {
    pub enabled: bool,
    pub duration: Duration,
    pub effect: ScrollEffect,
    pub easing: ScrollEasing,
}

impl WindowTransitionConfig {
    fn crossfade_default() -> Self {
        Self {
            enabled: true,
            duration: Duration::from_millis(200),
            effect: ScrollEffect::Crossfade,
            easing: ScrollEasing::EaseOutQuad,
        }
    }

    fn scroll_default() -> Self {
        Self {
            // Off by default: C-v/M-v (and other scrolls) update instantly,
            // matching stock Emacs. Opt in at runtime with
            //   (neomacs-effect-set 'scroll-transition :enabled t)
            // or via the `neomacs-effects' profile. The effect/duration/easing
            // below are the values used once it is enabled.
            enabled: false,
            duration: Duration::from_millis(150),
            effect: ScrollEffect::Slide,
            easing: ScrollEasing::EaseOutQuad,
        }
    }
}

/// Global motion policy (accessibility: the prefers-reduced-motion analog).
///
/// One typed answer to "should non-essential motion run?", enforced at every
/// gate that produces motion - cursor position/size animation, buffer
/// transitions, and the effect catalog - so no single effect can opt back in
/// on its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MotionPolicy {
    /// All configured animations run.
    #[default]
    Full,
    /// Non-essential motion is suppressed: cursor motion and size
    /// transitions snap, buffer transitions cut, and every effect with an
    /// `enabled` property starts disabled.  Cursor blinking is unaffected
    /// (it is state signaling, not motion).
    Reduced,
}

impl MotionPolicy {
    /// True when motion must be suppressed.
    pub fn is_reduced(&self) -> bool {
        matches!(self, MotionPolicy::Reduced)
    }
}

/// Desired visual configuration owned by the evaluator and published as one
/// snapshot to the render thread.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VisualConfig {
    /// The large shader-effect catalog remains a focused renderer type.  Serde
    /// flattening exposes it beside the behavioral configs in the registry.
    #[serde(flatten)]
    pub effects: EffectsConfig,
    /// Accessibility motion policy; gates every motion-producing config below
    /// and the flattened effect catalog.
    #[serde(default)]
    pub motion: MotionPolicy,
    pub cursor_blink: CursorBlinkConfig,
    pub cursor_motion: CursorMotionConfig,
    pub cursor_size_transition: CursorSizeTransitionConfig,
    pub crossfade_transition: WindowTransitionConfig,
    pub scroll_transition: WindowTransitionConfig,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            effects: EffectsConfig::default(),
            motion: MotionPolicy::default(),
            cursor_blink: CursorBlinkConfig::default(),
            cursor_motion: CursorMotionConfig::default(),
            cursor_size_transition: CursorSizeTransitionConfig::default(),
            crossfade_transition: WindowTransitionConfig::crossfade_default(),
            scroll_transition: WindowTransitionConfig::scroll_default(),
        }
    }
}

impl EffectsConfig {
    /// Apply the accessibility motion policy: under [`MotionPolicy::Reduced`]
    /// every effect exposing an `enabled` property is disabled. Effects
    /// without an `enabled` property are unconditional renders and stay as
    /// configured.
    pub fn with_motion_policy(&self, policy: MotionPolicy) -> Self {
        if !policy.is_reduced() {
            return self.clone();
        }
        let operations = self
            .effect_names()
            .into_iter()
            .filter(|name| {
                self.effect_values(name)
                    .map(|props| props.iter().any(|(key, _)| key == "enabled"))
                    .unwrap_or(false)
            })
            .map(|name| {
                EffectOperation::set(name.as_str(), [("enabled", EffectValue::Bool(false))])
            })
            .collect::<Vec<_>>();
        self.apply_effects(&operations)
            .expect("disabling an existing enabled property always validates")
    }
}
