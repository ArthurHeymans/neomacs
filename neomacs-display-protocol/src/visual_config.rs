//! Complete user-facing visual configuration snapshot.
//!
//! Shader effects, cursor behavior, and window transitions share one control
//! plane even though the renderer stores and executes them in different
//! subsystems.  Keeping that distinction behind `VisualConfig` gives Elisp a
//! single named, typed, atomic interface without flattening the runtime model.

use crate::{CursorAnimStyle, EffectsConfig, ScrollEasing, ScrollEffect};
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
            enabled: true,
            duration: Duration::from_millis(150),
            effect: ScrollEffect::Slide,
            easing: ScrollEasing::EaseOutQuad,
        }
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
            cursor_blink: CursorBlinkConfig::default(),
            cursor_motion: CursorMotionConfig::default(),
            cursor_size_transition: CursorSizeTransitionConfig::default(),
            crossfade_transition: WindowTransitionConfig::crossfade_default(),
            scroll_transition: WindowTransitionConfig::scroll_default(),
        }
    }
}
