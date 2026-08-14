//! Shared transition policy config for crossfade/scroll animations.

use crate::{VisualConfig, WindowTransitionConfig};

/// Animation policy for per-window transitions.
///
/// This is the authoritative transition config shared across crates; render
/// code consumes this policy instead of owning separate config fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionPolicy {
    pub crossfade: WindowTransitionConfig,
    pub scroll: WindowTransitionConfig,
}

impl TransitionPolicy {
    /// True when at least one transition path needs offscreen snapshots.
    pub fn needs_offscreen(&self) -> bool {
        self.crossfade.enabled || self.scroll.enabled
    }
}

impl Default for TransitionPolicy {
    fn default() -> Self {
        Self::from(&VisualConfig::default())
    }
}

impl From<&VisualConfig> for TransitionPolicy {
    fn from(config: &VisualConfig) -> Self {
        // Reduced motion cuts between buffer states instead of animating.
        let motion_allowed = !config.motion.is_reduced();
        Self {
            crossfade: WindowTransitionConfig {
                enabled: config.crossfade_transition.enabled && motion_allowed,
                ..config.crossfade_transition
            },
            scroll: WindowTransitionConfig {
                enabled: config.scroll_transition.enabled && motion_allowed,
                ..config.scroll_transition
            },
        }
    }
}
