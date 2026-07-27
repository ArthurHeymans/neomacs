//! Buffer window source read bounds and text extraction.

use crate::neovm_bridge::{LayoutBufferView, RustBufferAccess};
use crate::scroll_policy::{
    ForwardScroll, ScrollPolicy, count_lines_bounded, last_usable_row, line_start_above,
    line_start_below,
};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferWindowSourceRequest {
    requested_window_start: i64,
    point_charpos: i64,
    accessible_start: i64,
    accessible_end: i64,
    max_rows: usize,
    visible_cols: i64,
    kind: WindowKind,
    scroll_policy: ScrollPolicy,
    scroll_margin: i64,
}

impl BufferWindowSourceRequest {
    pub(crate) fn from_window_params(params: &WindowParams, max_rows: usize) -> Self {
        Self::new(
            params.window_start_charpos().get(),
            params.point_charpos().get(),
            params.accessible_start_charpos().get(),
            params.accessible_end_charpos().get(),
            max_rows,
            visible_cols_for_window_params(params),
            params.kind,
            ScrollPolicy::from_window_params(params),
            params.scroll_margin,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        requested_window_start: i64,
        point_charpos: i64,
        accessible_start: i64,
        accessible_end: i64,
        max_rows: usize,
        visible_cols: i64,
        kind: WindowKind,
        scroll_policy: ScrollPolicy,
        scroll_margin: i64,
    ) -> Self {
        Self {
            requested_window_start,
            point_charpos,
            accessible_start,
            accessible_end,
            max_rows,
            visible_cols: visible_cols.max(1),
            kind,
            scroll_policy,
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
                window_start = self.line_start_above_point(
                    (self.max_rows as i64 / 2).max(1),
                    &byte_at_charpos,
                );
            }
        }

        if self.point_charpos >= self.accessible_start && self.point_charpos < window_start {
            let adjusted =
                self.line_start_above_point((self.max_rows as i64 / 4).max(1), &byte_at_charpos);
            tracing::debug!(
                "layout_window_rust: adjusted window_start {} -> {} (point={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos
            );
            return adjusted;
        }

        if let Some(adjusted) = self.forward_scroll_window_start(window_start, &byte_at_charpos) {
            tracing::debug!(
                "layout_window_rust: forward-adjusted window_start {} -> {} (point={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos,
            );
            return adjusted;
        }

        window_start
    }

    /// New window start when point sits below the window, per GNU
    /// `try_scrolling` (src/xdisp.c:19359). `None` when point is already on
    /// screen or this pass must not scroll.
    ///
    /// This is the fast path — it runs before any layout, so it measures the
    /// distance to point in BUFFER lines. That is exact for a window with no
    /// wrapped or invisible text; anything else under-counts, point stays off
    /// screen, and the display-line-accurate visibility retry
    /// (`TextWindowVisibilityRetryRequest`) finishes the scroll.
    fn forward_scroll_window_start(
        self,
        window_start: i64,
        byte_at_charpos: &impl Fn(i64) -> Option<u8>,
    ) -> Option<i64> {
        if self.kind.is_minibuffer() {
            return None;
        }
        // A non-minibuffer window laid out at a degenerate (<= 1 row) height is a
        // transient/probe state — e.g. an intermediate pass while a child-frame
        // (posframe) or frame resize is in flight. Its viewport is too small to
        // estimate a real scroll from: every point past the first row looks "far
        // below", so this would scroll window_start to point. That scrolled start
        // then PERSISTS and corrupts the real (tall) window (the Doom dashboard
        // banner scrolls off when `SPC SPC` opens find-file). GNU never scrolls
        // an editing window from such a state.
        if self.max_rows <= 1 {
            return None;
        }

        let bottom_row = last_usable_row(self.max_rows, self.scroll_margin);
        let (lines_to_point, bounded) = count_lines_bounded(
            window_start,
            self.point_charpos,
            bottom_row + self.scroll_policy.search_limit_lines(),
            byte_at_charpos,
        );
        // GNU's `dy`: how far point falls past the last row the bottom
        // scroll-margin leaves usable (xdisp.c:19443). `<= 0` means point is
        // already visible, which is GNU's `if (dy > 0) scroll_down_p = true`.
        let dy = lines_to_point - bottom_row;
        if dy <= 0 {
            return None;
        }

        Some(
            match self
                .scroll_policy
                .forward_scroll(dy, bounded, self.max_rows, self.scroll_margin)
            {
                ForwardScroll::Advance { lines } => line_start_below(
                    window_start,
                    lines,
                    self.accessible_end,
                    byte_at_charpos,
                ),
                ForwardScroll::Recenter { lines_above_point } => {
                    self.line_start_above_point(lines_above_point, byte_at_charpos)
                }
            },
        )
    }

    fn line_start_above_point(
        self,
        lines_above: i64,
        byte_at_charpos: &impl Fn(i64) -> Option<u8>,
    ) -> i64 {
        line_start_above(
            self.point_charpos,
            lines_above,
            self.accessible_start,
            byte_at_charpos,
        )
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
