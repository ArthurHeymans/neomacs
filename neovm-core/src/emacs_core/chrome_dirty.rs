//! Which windows must re-generate their chrome (mode / header / tab line).
//!
//! This is the port of GNU's mode-line dirty flags. GNU keeps two, and the
//! distinction matters because they have different reach:
//!
//! * `update_mode_lines` — a global (`xdisp.c:901-907`, set by
//!   `bset_update_mode_line`). Everything buffer-scoped raises it, because
//!   a buffer can be shown in several windows and GNU does not track which.
//! * `w->update_mode_line` — per window (`xdisp.c:909-920`, set by
//!   `wset_update_mode_line`), raised by the window-scoped events:
//!   `set-window-start`, `set-window-buffer`, the scroll commands.
//!
//! Both feed the same two decisions. The one that matters for cost is that
//! they are *preconditions of GNU's one-line optimization*
//! (`xdisp.c:17572-17610`): while they are clear, an edit confined to one
//! line re-displays that glyph row and jumps straight to the update phase
//! (`goto update`, `:17726`), never entering `redisplay_window` and so never
//! reaching `display_mode_lines`. That, not the guard chain inside
//! `redisplay_window`, is why GNU does not re-walk the mode line on every
//! keystroke. The full extraction is in `tmp/p52-gnu-extraction.md`.
//!
//! Neomacs's analogue of that optimization is the cursor-only / scroll-replay
//! fast path, which today re-walks chrome unconditionally
//! (`buffer_source/render_plan.rs`, "Chrome is always re-walked"). This type
//! is the flag half of the port: the triggers set it and redisplay clears it,
//! but **nothing consults it as a skip yet** — the skip is a separate,
//! separately-measured increment. Until then the flag is observable only to
//! the pins that prove each trigger raises it.
//!
//! GNU also sets `prevent_redisplay_optimizations_p` alongside the flag in
//! several places (notably `Fforce_mode_line_update`). We do not model that
//! separately: in GNU it exists to disqualify optimizations that do not
//! consult `update_mode_line` itself, whereas here the chrome flag will be a
//! precondition of the fast path directly.

use crate::window::WindowId;
use std::collections::HashSet;

/// The set of windows whose chrome must be re-generated.
#[derive(Debug, Default, Clone)]
pub struct ChromeDirty {
    /// GNU `update_mode_lines`: every window on every frame.
    all: bool,
    /// GNU `w->update_mode_line`: these windows specifically.
    windows: HashSet<WindowId>,
}

impl ChromeDirty {
    /// GNU `bset_update_mode_line` (`xdisp.c:901-907`): a buffer-scoped event
    /// raises the global flag, because the buffer may be shown in windows
    /// this call cannot enumerate.
    pub fn mark_all(&mut self) {
        self.all = true;
    }

    /// GNU `wset_update_mode_line` (`xdisp.c:909-920`): a window-scoped event.
    pub fn mark_window(&mut self, window: WindowId) {
        if !self.all {
            self.windows.insert(window);
        }
    }

    /// Whether WINDOW must re-generate its chrome this redisplay.
    pub fn is_dirty(&self, window: WindowId) -> bool {
        self.all || self.windows.contains(&window)
    }

    /// Whether anything at all is dirty.
    pub fn is_any_dirty(&self) -> bool {
        self.all || !self.windows.is_empty()
    }

    /// Clear after a redisplay has re-generated the chrome it was asked for.
    /// GNU clears the equivalent state in `mark_window_display_accurate_1` and
    /// by resetting `update_mode_lines` at the end of `redisplay_internal`.
    pub fn clear(&mut self) {
        self.all = false;
        self.windows.clear();
    }
}
