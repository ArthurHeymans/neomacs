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
        Self {
            crossfade: config.crossfade_transition,
            scroll: config.scroll_transition,
        }
    }
}
