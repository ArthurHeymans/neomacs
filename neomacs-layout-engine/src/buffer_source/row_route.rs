//! Row acquisition routing for the buffer-source migration.
//!
//! GNU has ONE iterator (`struct it`, xdisp.c) and `next_element_from_buffer`
//! is simply a method on it; neomacs still has two row paths — the buffer
//! pipeline (this module's siblings) and the unified item renderer
//! (`display_row/`, driven by `DisplayItemSource`). This module is the seam
//! that migrates the simplest buffer row class onto the item renderer:
//!
//! * [`RowAcquisitionRoute`] + [`classify_row_acquisition`] decide, at the
//!   start of a buffer line, whether the row is plain enough for the item
//!   renderer. "Plain" mirrors what makes GNU `get_next_display_element`
//!   trivial: single-byte printable ASCII only, no display/composition/
//!   invisible/face properties in range, no overlays, no display table, L2R
//!   (guaranteed by ASCII-only content), the row fits without continuation or
//!   truncation, and it ends in a real newline.
//! * [`BufferAsciiItemSource`] is the `DisplayItemSource` for such a row: it
//!   produces exactly the items `BufferTextSourceCursor` would — one plain
//!   `TextRun` over the line, then the explicit-newline row break.
//!
//! Rows carrying point are deliberately excluded: cursor capture is a
//! documented buffer-pipeline responsibility (see the cursor-capture note in
//! `row_lifecycle.rs`), mirroring GNU `set_cursor_from_row` operating on
//! buffer positions only.

use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayLineHeightPolicy, DisplayRowBreak,
    DisplaySourcePosition, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::neovm_bridge::LayoutBufferView;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, EmacsByteRange};
use neovm_core::emacs_core::Value;

/// Which pipeline acquires and renders a buffer row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowAcquisitionRoute {
    /// The full buffer pipeline (`loop_render` / `item_render` orchestration).
    BufferPipeline,
    /// The unified item renderer, fed by [`BufferAsciiItemSource`].
    ItemRenderer,
}

/// Per-window facts that disqualify the item-renderer route regardless of row
/// content. Each active feature has buffer-pipeline bookkeeping (hscroll skip,
/// selective display, word-wrap candidates, trailing-whitespace tracking) that
/// the routed render deliberately does not replicate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowRouteWindowPolicy {
    pub(crate) point_charpos: i64,
    pub(crate) hscroll_active: bool,
    pub(crate) selective_display: i32,
    pub(crate) word_wrap: bool,
    pub(crate) show_trailing_whitespace: bool,
}

impl RowRouteWindowPolicy {
    fn disqualifies(&self) -> bool {
        self.hscroll_active
            || self.selective_display != 0
            || self.word_wrap
            || self.show_trailing_whitespace
    }
}

/// The buffer-walk position at the start of a candidate row.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowRouteRowStart<'a> {
    /// The visible buffer text the walk iterates (starts at `text_start_byte`).
    pub(crate) text: &'a [u8],
    /// Byte index of the row start within `text`.
    pub(crate) byte_idx: usize,
    /// 0-based char position of the row start.
    pub(crate) charpos: i64,
    /// Emacs byte position of `text[0]`.
    pub(crate) text_start_byte: usize,
}

/// Pixel-fit inputs: the row must hold the whole line WITHOUT continuation or
/// truncation. The classifier applies the logical-cell bound strictly (a line
/// exactly filling the row is NOT eligible — its line end interacts with
/// continuation policy), and the routed render re-verifies with the same
/// natural measurement the buffer pipeline uses before committing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowRouteFit {
    pub(crate) start_x_px: f32,
    pub(crate) char_width_px: f32,
    pub(crate) right_edge_px: f32,
}

/// A classified plain-ASCII row: `line_len` single-byte chars followed by a
/// real newline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AsciiRowPlan {
    line_len: usize,
}

impl AsciiRowPlan {
    pub(crate) fn line_len(self) -> usize {
        self.line_len
    }
}

/// Scan for a plain printable-ASCII line at `byte_idx`: at least one char in
/// `0x20..=0x7E` (no tab, no control chars, no non-ASCII bytes) terminated by
/// a real `\n` inside `text`. Returns the line length in chars (== bytes).
fn ascii_plain_line_len(text: &[u8], byte_idx: usize) -> Option<usize> {
    let mut idx = byte_idx;
    while idx < text.len() {
        match text[idx] {
            b'\n' => {
                let len = idx - byte_idx;
                return (len > 0).then_some(len);
            }
            0x20..=0x7E => idx += 1,
            _ => return None,
        }
    }
    // End of buffer without a newline: the end-of-buffer tail has its own
    // buffer-pipeline lifecycle (cursor at EOB, empty-line indicators).
    None
}

/// Text properties that influence acquisition, faces, or the line end. Any of
/// these present at the row start sends the row to the buffer pipeline. The
/// property-CHANGE check in the classifier guarantees they are constant over
/// the whole line including its newline, so probing the start byte suffices.
const ROUTE_BLOCKING_TEXT_PROPS: [&str; 7] = [
    "face",
    "font-lock-face",
    "display",
    "composition",
    "invisible",
    "mouse-face",
    "line-height",
];

/// Classify the row starting at `row` for acquisition routing. Returns
/// [`RowAcquisitionRoute::ItemRenderer`] only for the plain row class
/// described in the module docs; everything else stays on the buffer
/// pipeline. Cheap checks run first; the property/overlay probes only run for
/// content that already passed the ASCII scan.
pub(crate) fn classify_row_acquisition<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    row: RowRouteRowStart<'_>,
    fit: RowRouteFit,
    policy: RowRouteWindowPolicy,
) -> RowAcquisitionRoute {
    if plan_ascii_row(buffer, row, fit, policy).is_some() {
        RowAcquisitionRoute::ItemRenderer
    } else {
        RowAcquisitionRoute::BufferPipeline
    }
}

/// The classifier behind [`classify_row_acquisition`], returning the routed
/// row's plan so the render path does not rescan.
pub(crate) fn plan_ascii_row<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    row: RowRouteRowStart<'_>,
    fit: RowRouteFit,
    policy: RowRouteWindowPolicy,
) -> Option<AsciiRowPlan> {
    if policy.disqualifies() {
        return None;
    }
    // Only whole rows are routed: the walk must be at the start of a buffer
    // line (never resuming mid-line after a wrap or a display element).
    if row.byte_idx > 0 && row.text.get(row.byte_idx - 1) != Some(&b'\n') {
        return None;
    }
    let line_len = ascii_plain_line_len(row.text, row.byte_idx)?;

    // Strict logical-cell fit: a line exactly filling the row keeps the
    // buffer pipeline (continuation/truncation policy owns that edge).
    let line_width_px = line_len as f32 * fit.char_width_px;
    if !(fit.start_x_px + line_width_px < fit.right_edge_px) {
        return None;
    }

    // Cursor capture stays on the buffer pipeline: exclude any row that
    // contains point, including point sitting on the line's newline.
    let newline_charpos = row.charpos + line_len as i64;
    if policy.point_charpos >= row.charpos && policy.point_charpos <= newline_charpos {
        return None;
    }

    // The simplest row class has no overlays at all in the buffer (faces,
    // display specs, before/after strings, mouse-face all arrive by overlay).
    if !buffer.layout_overlays().is_empty() {
        return None;
    }

    // An active display table can remap any char (including the newline).
    if crate::neovm_bridge::buffer_has_active_display_table(buffer) {
        return None;
    }

    // Properties must be constant over the line AND its newline; then probing
    // the start byte covers the whole row.
    let start_byte = EmacsBytePos::new(row.text_start_byte + row.byte_idx);
    let newline_byte = row.text_start_byte + row.byte_idx + line_len;
    if let Some(change) = buffer.layout_next_text_prop_change_after_emacs_byte_pos(start_byte)
        && change.get() <= newline_byte
    {
        return None;
    }
    for prop in ROUTE_BLOCKING_TEXT_PROPS {
        if buffer
            .layout_text_prop_at_emacs_byte_pos(start_byte, Value::symbol(prop))
            .is_some()
        {
            return None;
        }
    }

    Some(AsciiRowPlan { line_len })
}

/// A `DisplayItemSource` over one classified plain-ASCII buffer row. Produces
/// exactly the items `BufferTextSourceCursor` would for the same row — one
/// plain `TextRun` covering the line, then (when the row break is included)
/// the explicit-newline `RowBreak` — mirroring GNU `next_element_from_buffer`
/// yielding the line's characters and then the newline element.
pub(crate) struct BufferAsciiItemSource {
    text_item: Option<DisplayItem>,
    row_break_item: Option<DisplayItem>,
}

impl BufferAsciiItemSource {
    /// Source over `[start, line_end)` text plus the newline row break at
    /// `line_end` — the full row, as the shadow renderer consumes it.
    pub(crate) fn with_row_break<B: LayoutBufferView + ?Sized>(
        buffer_id: BufferId,
        buffer: &B,
        start: CharPos0,
        line_end: CharPos0,
        face: RenderFaceRef,
    ) -> Self {
        Self::new(buffer_id, buffer, start, line_end, face, true)
    }

    /// Source over the line text only; the buffer pipeline's own line-break
    /// lifecycle (line-end plan, appended newline space, row transition)
    /// consumes the newline. Used by the routed production render.
    pub(crate) fn text_only<B: LayoutBufferView + ?Sized>(
        buffer_id: BufferId,
        buffer: &B,
        start: CharPos0,
        line_end: CharPos0,
        face: RenderFaceRef,
    ) -> Self {
        Self::new(buffer_id, buffer, start, line_end, face, false)
    }

    fn new<B: LayoutBufferView + ?Sized>(
        buffer_id: BufferId,
        buffer: &B,
        start: CharPos0,
        line_end: CharPos0,
        face: RenderFaceRef,
        include_row_break: bool,
    ) -> Self {
        let byte_at = |pos: CharPos0| buffer.layout_char_pos_to_emacs_byte_pos(pos);
        let span = |from: CharPos0, to: CharPos0| {
            SourceSpan::new(
                DisplaySourcePosition::buffer(buffer_id, from, byte_at(from)),
                DisplaySourcePosition::buffer(buffer_id, to, byte_at(to)),
            )
        };

        let text_item = (line_end > start).then(|| {
            let mut bytes = Vec::new();
            buffer.layout_copy_emacs_byte_range_to(
                EmacsByteRange::new(byte_at(start), byte_at(line_end)),
                &mut bytes,
            );
            debug_assert!(
                bytes.iter().all(|byte| (0x20..=0x7E).contains(byte)),
                "BufferAsciiItemSource requires a classified printable-ASCII row"
            );
            let text: String = bytes.iter().map(|&byte| byte as char).collect();
            DisplayItem::new(
                span(start, line_end),
                face,
                DisplayItemKind::TextRun(DisplayTextRun::new(text)),
            )
            .with_layout(DisplayItemLayout::default())
            .with_pointer_appearance(None)
        });

        let row_break_item = include_row_break.then(|| {
            // Mirrors `BufferTextSourceCursor::next_text_item_with_layout`:
            // the newline's row break carries the line-height policy resolved
            // from the (absent, for a classified row) `line-height` property.
            let row_break = DisplayRowBreak::explicit_newline()
                .with_line_height(DisplayLineHeightPolicy::from_property(None));
            DisplayItem::new(
                span(
                    line_end,
                    line_end.add_len(neovm_core::buffer::CharLen::new(1)),
                ),
                face,
                DisplayItemKind::RowBreak(row_break),
            )
            .with_layout(DisplayItemLayout::default())
            .with_pointer_appearance(None)
        });

        Self {
            text_item,
            row_break_item,
        }
    }

    /// The line's `TextRun` item without consuming the source (the routed
    /// production render measures the run before committing to the route).
    pub(crate) fn text_item(&self) -> Option<&DisplayItem> {
        self.text_item.as_ref()
    }
}

impl crate::display_source::DisplayItemSource for BufferAsciiItemSource {
    fn next_item(
        &mut self,
        _context: &mut crate::display_source::DisplaySourceContext<'_>,
    ) -> Option<DisplayItem> {
        self.text_item.take().or_else(|| self.row_break_item.take())
    }
}

#[cfg(test)]
#[path = "row_route_test.rs"]
mod tests;
