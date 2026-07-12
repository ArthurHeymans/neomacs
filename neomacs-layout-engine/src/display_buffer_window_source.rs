//! Buffer window source read bounds and text extraction.

use crate::neovm_bridge::{LayoutBufferView, RustBufferAccess};
use crate::types::{WindowKind, WindowParams};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferWindowSource {
    window_start: i64,
    text_start_byte: usize,
    bytes_read: usize,
    point_charpos: i64,
    accessible_start: i64,
    accessible_end: i64,
    accessible_end_lisp_char: usize,
    accessible_end_emacs_byte: usize,
}

impl BufferWindowSource {
    pub(crate) const fn window_start(self) -> i64 {
        self.window_start
    }

    pub(crate) const fn text_start_byte(self) -> usize {
        self.text_start_byte
    }

    pub(crate) const fn bytes_read(self) -> usize {
        self.bytes_read
    }

    pub(crate) const fn point_charpos(self) -> i64 {
        self.point_charpos
    }

    pub(crate) const fn accessible_start(self) -> i64 {
        self.accessible_start
    }

    pub(crate) const fn accessible_end(self) -> i64 {
        self.accessible_end
    }

    pub(crate) const fn accessible_end_lisp_char(self) -> usize {
        self.accessible_end_lisp_char
    }

    pub(crate) const fn accessible_end_emacs_byte(self) -> usize {
        self.accessible_end_emacs_byte
    }
}

/// GNU `SCROLL_LIMIT` (src/xdisp.c:19349): a `scroll-conservatively` above this
/// disables recentering — redisplay then always scrolls minimally.
pub(crate) const SCROLL_CONSERVATIVELY_LIMIT: i64 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferWindowSourceRequest {
    requested_window_start: i64,
    previous_window_end: Option<i64>,
    point_charpos: i64,
    accessible_start: i64,
    accessible_end: i64,
    max_rows: usize,
    visible_cols: i64,
    kind: WindowKind,
    scroll_conservatively: i64,
    scroll_margin: i64,
}

impl BufferWindowSourceRequest {
    pub(crate) fn from_window_params(params: &WindowParams, max_rows: usize) -> Self {
        Self::new(
            params.window_start_charpos().get(),
            params.previous_window_end_charpos().map(|pos| pos.get()),
            params.point_charpos().get(),
            params.accessible_start_charpos().get(),
            params.accessible_end_charpos().get(),
            max_rows,
            visible_cols_for_window_params(params),
            params.kind,
            params.scroll_conservatively,
            params.scroll_margin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        requested_window_start: i64,
        previous_window_end: Option<i64>,
        point_charpos: i64,
        accessible_start: i64,
        accessible_end: i64,
        max_rows: usize,
        visible_cols: i64,
        kind: WindowKind,
        scroll_conservatively: i64,
        scroll_margin: i64,
    ) -> Self {
        Self {
            requested_window_start,
            previous_window_end,
            point_charpos,
            accessible_start,
            accessible_end,
            max_rows,
            visible_cols: visible_cols.max(1),
            kind,
            scroll_conservatively,
            scroll_margin,
        }
    }

    pub(crate) fn read_into<B: LayoutBufferView>(
        self,
        access: &RustBufferAccess<'_, B>,
        out: &mut Vec<u8>,
    ) -> BufferWindowSource {
        let window_start =
            self.resolve_window_start(|charpos| access.byte_at(access.charpos_to_bytepos(charpos)));
        self.read_from_resolved_start(window_start, access, out)
    }

    /// Read from an already-resolved partial-layout boundary.
    ///
    /// Incremental replay computes the exact first character that must be
    /// relaid. Applying normal viewport scrolling/recentering to that boundary
    /// would change the requested source range and duplicate retained rows.
    pub(crate) fn read_exact_into<B: LayoutBufferView>(
        self,
        access: &RustBufferAccess<'_, B>,
        out: &mut Vec<u8>,
    ) -> BufferWindowSource {
        let window_start = self
            .requested_window_start
            .clamp(self.accessible_start, self.accessible_end);
        self.read_from_resolved_start(window_start, access, out)
    }

    fn read_from_resolved_start<B: LayoutBufferView>(
        self,
        window_start: i64,
        access: &RustBufferAccess<'_, B>,
        out: &mut Vec<u8>,
    ) -> BufferWindowSource {
        let text_start_byte = access.charpos_to_bytepos(window_start) as usize;
        let read_chars = self.accessible_end - window_start + 1;
        let bytes_read = if read_chars <= 0 {
            out.clear();
            0
        } else {
            let text_end = (window_start + read_chars).min(self.accessible_end);
            let byte_to = access.charpos_to_bytepos(text_end);
            access.copy_text(text_start_byte as i64, byte_to, out);
            out.len()
        };

        BufferWindowSource {
            window_start,
            text_start_byte,
            bytes_read,
            point_charpos: self.point_charpos,
            accessible_start: self.accessible_start,
            accessible_end: self.accessible_end,
            accessible_end_lisp_char: self.accessible_end.max(0) as usize + 1,
            accessible_end_emacs_byte: access.zv().max(0) as usize,
        }
    }

    fn resolve_window_start(self, byte_at_charpos: impl Fn(i64) -> Option<u8>) -> i64 {
        let mut window_start = self.requested_window_start.max(self.accessible_start);

        if window_start > self.accessible_start {
            let remaining_chars = self.accessible_end - window_start;
            if remaining_chars < self.max_rows as i64 && self.accessible_end > self.max_rows as i64
            {
                window_start =
                    self.scan_back_from_point((self.max_rows / 2).max(1), &byte_at_charpos);
            }
        }

        if self.point_charpos >= self.accessible_start && self.point_charpos < window_start {
            let adjusted = self.scan_back_from_point((self.max_rows / 4).max(1), &byte_at_charpos);
            tracing::debug!(
                "layout_window_rust: adjusted window_start {} -> {} (point={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos
            );
            return adjusted;
        }

        if self.should_forward_scroll_without_layout(window_start) {
            let rows_above = self.forward_scroll_rows_above(&byte_at_charpos);
            let adjusted = self.scan_back_from_point(rows_above, &byte_at_charpos);
            tracing::debug!(
                "layout_window_rust: forward-adjusted window_start {} -> {} (point={}, prev_end={}, rows_above={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos,
                self.previous_window_end.unwrap_or(0),
                rows_above,
            );
            return adjusted;
        }

        window_start
    }

    /// Rows of context kept above point when forward-scrolling to show point
    /// that is below the window, per GNU `try_scrolling` (src/xdisp.c:19360).
    ///
    /// A downward jump of more than `scroll-conservatively` lines fails minimal
    /// scrolling (SCROLLING_FAILED) and recenters point to the window middle
    /// (the `recenter:` label, `centering_position = window_box_height / 2`,
    /// xdisp.c:21188). A smaller jump scrolls minimally, leaving point at the
    /// bottom scroll-margin. A `scroll-conservatively` above GNU's SCROLL_LIMIT
    /// disables recentering entirely (always minimal).
    ///
    /// This is the fast forward-scroll path (no layout yet), so it counts buffer
    /// lines — exact for non-wrapped windows; wrapped windows fall through to the
    /// display-line-aware retry instead of this path.
    fn forward_scroll_rows_above(&self, byte_at_charpos: &impl Fn(i64) -> Option<u8>) -> usize {
        // `scan_back_from_point(n)` lands on the n-th newline above point, so the
        // first visible line is the one *after* it — i.e. point ends up with
        // `n - 1` lines of context above it. To leave `k` lines above point we
        // therefore pass `k + 1`.
        let recenter = self.scroll_conservatively >= 0
            && self.scroll_conservatively <= SCROLL_CONSERVATIVELY_LIMIT
            && self.forward_jump_exceeds(self.scroll_conservatively, byte_at_charpos);
        if recenter {
            // Recenter: `window_box_height / 2` lines of context above point
            // (GNU `recenter:` centering_position, xdisp.c:21188).
            (self.max_rows / 2) + 1
        } else {
            // Near jump within `scroll-conservatively`: GNU scrolls minimally,
            // leaving point at the bottom scroll-margin. Placing point on the
            // very last row here can land it on a partially-clipped row (this
            // window fits ~1 fewer full row than GNU), which makes the later
            // visibility retry over-scroll point to the top. Until that geometry
            // gap and the retry's scroll-conservatively handling are addressed,
            // keep the long-standing heuristic that leaves point comfortably
            // above the bottom (~3/4 down) — non-regressing for near jumps.
            ((self.max_rows * 3) / 4).max(1)
        }
    }

    /// Whether point is more than `threshold` lines below the current viewport's
    /// bottom — GNU's `dy > scroll_max` test in `try_scrolling`. The viewport
    /// bottom sits ~`max_rows - 1` lines below `window_start`, so we count lines
    /// from `window_start` to point and discount the on-screen rows (`slack`).
    /// This is robust even when `previous_window_end` is unreliable. Scans only
    /// until the limit is exceeded, so a far jump never walks the whole buffer
    /// (GNU bounds the same `move_it_to` search by `scroll_max`).
    fn forward_jump_exceeds(
        &self,
        threshold: i64,
        byte_at_charpos: &impl Fn(i64) -> Option<u8>,
    ) -> bool {
        let from = self.requested_window_start.max(self.accessible_start);
        let slack = (self.max_rows as i64 - 1).max(0);
        let limit = threshold.saturating_add(slack);
        let mut lines = 0i64;
        let mut pos = from;
        while pos < self.point_charpos {
            if byte_at_charpos(pos) == Some(b'\n') {
                lines += 1;
                if lines > limit {
                    return true;
                }
            }
            pos += 1;
        }
        false
    }

    fn should_forward_scroll_without_layout(self, window_start: i64) -> bool {
        if self.point_charpos <= 0 || self.kind.is_minibuffer() {
            return false;
        }
        // A non-minibuffer window laid out at a degenerate (<= 1 row) height is a
        // transient/probe state — e.g. an intermediate pass while a child-frame
        // (posframe) or frame resize is in flight. Its viewport is too small to
        // estimate a real scroll from: every point past the first row looks "far
        // below", so this heuristic would scroll window_start to point. That
        // scrolled start then PERSISTS and corrupts the real (tall) window (the
        // Doom dashboard banner scrolls off when `SPC SPC` opens find-file). GNU
        // never scrolls an editing window from such a state.
        if self.max_rows <= 1 {
            return false;
        }
        let has_prev_end = self
            .previous_window_end
            .is_some_and(|end| self.point_charpos > end);
        let max_visible_chars = (self.max_rows.max(1) as i64) * self.visible_cols;
        let far_below_without_prev_end = self.previous_window_end.is_none()
            && self.point_charpos - window_start > max_visible_chars;
        has_prev_end || far_below_without_prev_end
    }

    fn scan_back_from_point(
        self,
        target_rows_above: usize,
        byte_at_charpos: &impl Fn(i64) -> Option<u8>,
    ) -> i64 {
        let mut lines_back = 0usize;
        let mut scan_pos = self.point_charpos.max(self.accessible_start);
        while scan_pos > self.accessible_start && lines_back < target_rows_above {
            scan_pos -= 1;
            if byte_at_charpos(scan_pos) == Some(b'\n') {
                lines_back += 1;
            }
        }
        scan_pos.max(self.accessible_start)
    }
}

fn visible_cols_for_window_params(params: &WindowParams) -> i64 {
    let char_width = params.char_width.max(1.0);
    (params.text_bounds.width.max(1.0) / char_width)
        .floor()
        .max(1.0) as i64
}

#[cfg(test)]
#[path = "display_buffer_window_source_test.rs"]
mod tests;
