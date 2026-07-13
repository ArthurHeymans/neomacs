//! Transactional per-window layout outcomes.
//!
//! A window may begin an attempt with retained or face-estimated chrome
//! metrics.  Shaping the tab/header/mode rows produces their actual intrinsic
//! metrics.  A mismatch is a layout invalidation: callers must discard the
//! attempt and retry before publishing any body/cursor/spatial output.

use crate::types::WindowParams;
use neovm_core::window::WindowDisplaySnapshot;

/// The vertical metrics that partition one leaf window's body from its chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WindowChromeMetrics {
    pub(crate) tab_line_height: f32,
    pub(crate) header_line_height: f32,
    pub(crate) mode_line_height: f32,
}

impl WindowChromeMetrics {
    const STABLE_PIXEL_EPSILON: f32 = 0.01;

    pub(crate) fn from_params(params: &WindowParams) -> Self {
        Self {
            tab_line_height: params.tab_line_height,
            header_line_height: params.header_line_height,
            mode_line_height: params.mode_line_height,
        }
    }

    pub(crate) fn from_snapshot(snapshot: &WindowDisplaySnapshot) -> Self {
        Self {
            tab_line_height: snapshot.tab_line_height.max(0) as f32,
            header_line_height: snapshot.header_line_height.max(0) as f32,
            mode_line_height: snapshot.mode_line_height.max(0) as f32,
        }
    }

    /// Seed a new attempt with accepted metrics while respecting the current
    /// window's wants-* decision.  A newly enabled row has no retained positive
    /// metric, so it keeps the bridge's face estimate for its first attempt.
    pub(crate) fn seed_params(self, params: &mut WindowParams) {
        params.tab_line_height =
            retained_or_estimated(params.tab_line_height, self.tab_line_height);
        params.header_line_height =
            retained_or_estimated(params.header_line_height, self.header_line_height);
        params.mode_line_height =
            retained_or_estimated(params.mode_line_height, self.mode_line_height);
    }

    fn is_stable_with(self, measured: Self) -> bool {
        metric_is_stable(self.tab_line_height, measured.tab_line_height)
            && metric_is_stable(self.header_line_height, measured.header_line_height)
            && metric_is_stable(self.mode_line_height, measured.mode_line_height)
    }
}

fn retained_or_estimated(estimated: f32, retained: f32) -> f32 {
    if estimated > 0.0 && retained > 0.0 {
        retained
    } else {
        estimated.max(0.0)
    }
}

fn metric_is_stable(assumed: f32, measured: f32) -> bool {
    (assumed - measured).abs() <= WindowChromeMetrics::STABLE_PIXEL_EPSILON
}

/// Publication outcome for one leaf-window attempt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum WindowLayoutOutcome {
    /// The window had no materializable body for this attempt.
    Skipped,
    /// Body, chrome, cursor, and spatial geometry use the same metrics.
    Stable(WindowChromeMetrics),
    /// Shaping discovered different intrinsic metrics.  The containing frame
    /// must discard the attempt and immediately relayout with `measured`.
    NeedsRelayout {
        assumed: WindowChromeMetrics,
        measured: WindowChromeMetrics,
    },
}

impl WindowLayoutOutcome {
    pub(crate) fn from_metrics(
        assumed: WindowChromeMetrics,
        measured: WindowChromeMetrics,
    ) -> Self {
        if assumed.is_stable_with(measured) {
            Self::Stable(measured)
        } else {
            Self::NeedsRelayout { assumed, measured }
        }
    }
}
