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
//!   invisible/mouse-face/line-height properties in range, no overlays, no
//!   display table, L2R (guaranteed by ASCII-only content), the row fits
//!   without continuation or truncation, and it ends in a real newline.
//!   FACE-affecting properties (`face`, `font-lock-face`, `fontified`
//!   boundaries) are allowed: they segment the row at property-change
//!   positions exactly like GNU `compute_stop_pos` bounds the iterator's
//!   text runs and `handle_face_prop` re-resolves the face at each stop.
//! * [`BufferAsciiItemSource`] is the `DisplayItemSource` for such a row: it
//!   produces exactly the items `BufferTextSourceCursor` would — one plain
//!   `TextRun` per face segment (one for the whole line when no property
//!   changes in range), then the explicit-newline row break.
//!
//! Rows carrying point are deliberately excluded: cursor capture is a
//! documented buffer-pipeline responsibility (see the cursor-capture note in
//! `row_lifecycle.rs`), mirroring GNU `set_cursor_from_row` operating on
//! buffer positions only.

use crate::composition::needs_complex_shaping;
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayItemLayout, DisplayLineHeightPolicy, DisplayRowBreak,
    DisplaySourcePosition, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_origin::DisplayOrigin;
use crate::display_row::builder::{DisplayRowPosition, DisplayTabPolicy};
use crate::display_row::face_state::stable_face_id_for_resolved;
use crate::display_source::{
    TextSourceCharClassification, classify_text_source_char, nonascii_hyphen_p, nonascii_space_p,
};
use crate::frame_face_arena::FrameFaceAttempt;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, LayoutCharPropertyLookup, ResolvedFace};
use crate::unicode::{decode_utf8, is_cluster_extender, is_regional_indicator};
use neomacs_display_protocol::types::FaceId;
use neovm_core::buffer::{BufferId, CharLen, CharPos0, EmacsBytePos, EmacsByteRange};
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
/// natural measurement the buffer pipeline uses before committing. The tab
/// policy is the append surface's (buffer `tab-width` / `tab-stop-list`), so
/// the classifier's tab expansion is the SAME `DisplayTabPolicy::advance_from`
/// the pipeline's per-char advance resolves (GNU `gui_produce_glyphs`
/// `next_tab_x`); `start_col` is the walk column the first tab expands from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowRouteFit<'a> {
    pub(crate) start_x_px: f32,
    pub(crate) start_col: usize,
    pub(crate) char_width_px: f32,
    pub(crate) right_edge_px: f32,
    pub(crate) tab_policy: &'a DisplayTabPolicy,
}

/// A classified plain row: `line_char_len` routable chars (`line_byte_len`
/// bytes — the row may be multibyte) followed by a real newline.
/// `face_boundaries` are the CHAR offsets strictly inside the line where text
/// properties change — each starts a new face segment, the neomacs mirror of
/// GNU `compute_stop_pos` stops re-resolved by `handle_face_prop`. Empty for
/// a property-constant line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AsciiRowPlan {
    line_byte_len: usize,
    line_char_len: usize,
    has_tab: bool,
    has_wide: bool,
    face_boundaries: Vec<usize>,
}

impl AsciiRowPlan {
    pub(crate) fn line_byte_len(&self) -> usize {
        self.line_byte_len
    }

    pub(crate) fn line_char_len(&self) -> usize {
        self.line_char_len
    }

    #[cfg(test)]
    pub(crate) fn has_tab(&self) -> bool {
        self.has_tab
    }

    #[cfg(test)]
    pub(crate) fn has_wide(&self) -> bool {
        self.has_wide
    }

    #[cfg(test)]
    pub(crate) fn face_boundaries(&self) -> &[usize] {
        &self.face_boundaries
    }

    /// Whether the row renders as more than one face segment.
    pub(crate) fn is_segmented(&self) -> bool {
        !self.face_boundaries.is_empty()
    }

    /// The `[start, end)` char ranges of the row's face segments, in row
    /// order. A property-constant line yields one range covering the line.
    pub(crate) fn segment_ranges(&self, start: CharPos0) -> Vec<(CharPos0, CharPos0)> {
        let mut ranges = Vec::with_capacity(self.face_boundaries.len() + 1);
        let mut seg_start = start;
        for boundary in &self.face_boundaries {
            let seg_end = start.add_len(CharLen::new(*boundary));
            ranges.push((seg_start, seg_end));
            seg_start = seg_end;
        }
        ranges.push((seg_start, start.add_len(CharLen::new(self.line_char_len))));
        ranges
    }
}

/// How a routed row char advances the pen. Only chars this classification
/// accepts may appear in a routed row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoutedRowCharAdvance {
    /// TAB: expands to the next stop of the append surface's tab policy.
    Tab,
    /// A plain char occupying `1` or `2` unambiguous columns.
    Cols(u8),
}

/// Classify one char for the routed row class. Accepted: TAB, printable
/// ASCII, and printable non-ASCII chars whose display width is unambiguously
/// 1 or 2 columns per the SAME width source the buffer pipeline advances by
/// (`neovm_core::encoding::char_width`, the GNU default `char-width-table`,
/// via `base_width_cols`). Refused (each has pipeline machinery the routed
/// render does not replicate):
/// * control chars other than TAB, and every non-Text classification
///   (`^X` caret runs, `\`+octal escapes, glyphless boxes);
/// * cluster extenders (combining marks, zero-width chars, ZWJ, keycap,
///   skin tones) and regional indicators — grapheme clustering composes them
///   with neighbors into `Composite` glyphs;
/// * contextual-shaping scripts (Arabic, Indic, …) — run shaping measures
///   advances per shaped run;
/// * nobreak spaces/hyphens — their display consults the
///   `nobreak-char-display` setting per char (GNU xdisp.c:8594);
/// * anything the shared width table does not size at exactly 1 or 2 cols.
fn classify_routed_row_char(ch: char) -> Option<RoutedRowCharAdvance> {
    if ch == '\t' {
        return Some(RoutedRowCharAdvance::Tab);
    }
    if matches!(ch, '\x20'..='\x7E') {
        return Some(RoutedRowCharAdvance::Cols(1));
    }
    if ch.is_ascii() {
        return None;
    }
    if classify_text_source_char(ch) != TextSourceCharClassification::Text {
        return None;
    }
    if is_cluster_extender(ch)
        || is_regional_indicator(ch as u32)
        || needs_complex_shaping(ch)
        || nonascii_space_p(ch)
        || nonascii_hyphen_p(ch)
    {
        return None;
    }
    match neovm_core::encoding::char_width(ch) {
        1 => Some(RoutedRowCharAdvance::Cols(1)),
        2 => Some(RoutedRowCharAdvance::Cols(2)),
        _ => None,
    }
}

/// A scanned routable line: `byte_len` bytes / `char_len` chars of routed
/// chars terminated by a real `\n` inside `text`.
#[derive(Clone, Copy, Debug)]
struct RoutedLineScan {
    byte_len: usize,
    char_len: usize,
    has_tab: bool,
    has_wide: bool,
}

/// Scan for a routable line at `byte_idx`: at least one char accepted by
/// [`classify_routed_row_char`] terminated by a real `\n` inside `text`.
fn routed_line_scan(text: &[u8], byte_idx: usize) -> Option<RoutedLineScan> {
    let mut idx = byte_idx;
    let mut char_len = 0usize;
    let mut has_tab = false;
    let mut has_wide = false;
    while idx < text.len() {
        if text[idx] == b'\n' {
            return (char_len > 0).then_some(RoutedLineScan {
                byte_len: idx - byte_idx,
                char_len,
                has_tab,
                has_wide,
            });
        }
        let (ch, consumed) = decode_utf8(&text[idx..]);
        // Reject malformed UTF-8 (decode yields U+FFFD over fewer bytes than
        // the char re-encodes to): raw bytes have their own display path.
        if consumed == 0 || ch.len_utf8() != consumed {
            return None;
        }
        match classify_routed_row_char(ch)? {
            RoutedRowCharAdvance::Tab => has_tab = true,
            RoutedRowCharAdvance::Cols(2) => has_wide = true,
            RoutedRowCharAdvance::Cols(_) => {}
        }
        char_len += 1;
        idx += consumed;
    }
    // End of buffer without a newline: the end-of-buffer tail has its own
    // buffer-pipeline lifecycle (cursor at EOB, empty-line indicators).
    None
}

/// The classifier's strict logical-cell fit for a scanned line: walk the
/// line's chars advancing the pen exactly as the pipeline's natural advance
/// does for uniform `char_width_px` cells — tabs through the append surface's
/// `DisplayTabPolicy::advance_from` (GNU `next_tab_x`), wide chars by two
/// cells — and require the end to land STRICTLY inside the right edge. A line
/// exactly filling the row keeps the buffer pipeline (continuation/truncation
/// policy owns that edge). The routed render re-verifies with the pipeline's
/// own per-face natural measurement before committing.
fn routed_line_fits(text: &[u8], byte_idx: usize, fit: RowRouteFit<'_>) -> bool {
    let mut x_px = fit.start_x_px;
    let mut col = fit.start_col;
    let mut idx = byte_idx;
    while idx < text.len() && text[idx] != b'\n' {
        let (ch, consumed) = decode_utf8(&text[idx..]);
        let Some(advance) = classify_routed_row_char(ch) else {
            return false;
        };
        match advance {
            RoutedRowCharAdvance::Tab => {
                let tab = fit
                    .tab_policy
                    .advance_from(DisplayRowPosition::new(x_px, col), fit.char_width_px);
                x_px += tab.pixel_width;
                col += tab.width_cols;
            }
            RoutedRowCharAdvance::Cols(cols) => {
                x_px += f32::from(cols) * fit.char_width_px;
                col += usize::from(cols);
            }
        }
        idx += consumed;
    }
    x_px < fit.right_edge_px
}

/// Text properties that influence acquisition or the line end beyond faces.
/// Any of these present anywhere on the line (or its newline) sends the row
/// to the buffer pipeline. Face-affecting properties (`face`,
/// `font-lock-face`, `fontified` boundaries) are NOT hazards: they only
/// segment the row and are handled by the routed face resolution. Properties
/// are constant between change positions, so probing each segment start (and
/// the newline, when a change lands on it) covers the whole row.
const ROUTE_HAZARD_TEXT_PROPS: [&str; 5] = [
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
    fit: RowRouteFit<'_>,
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
    fit: RowRouteFit<'_>,
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
    let scan = routed_line_scan(row.text, row.byte_idx)?;

    // Strict logical-cell fit (tab expansion + 2-col chars included): a line
    // exactly filling the row keeps the buffer pipeline
    // (continuation/truncation policy owns that edge).
    if !routed_line_fits(row.text, row.byte_idx, fit) {
        return None;
    }

    // Cursor capture stays on the buffer pipeline: exclude any row that
    // contains point, including point sitting on the line's newline.
    let newline_charpos = row.charpos + scan.char_len as i64;
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

    // Walk the property-change positions over the line AND its newline.
    // Hazard properties anywhere in that range refuse the route; changes
    // strictly inside the line become face-segment boundaries (a change ON
    // the newline byte does not split the text but is still hazard-probed —
    // a display/invisible property on the newline would replace it). The row
    // may be multibyte, so boundary BYTE positions convert to CHAR offsets
    // through the buffer's own mapping.
    let start_byte = row.text_start_byte + row.byte_idx;
    let newline_byte = start_byte + scan.byte_len;
    let mut face_boundaries = Vec::new();
    let mut probe_byte = start_byte;
    loop {
        let probe = EmacsBytePos::new(probe_byte);
        for prop in ROUTE_HAZARD_TEXT_PROPS {
            if buffer
                .layout_text_prop_at_emacs_byte_pos(probe, Value::symbol(prop))
                .is_some()
            {
                return None;
            }
        }
        let Some(change) = buffer.layout_next_text_prop_change_after_emacs_byte_pos(probe) else {
            break;
        };
        let change = change.get();
        if change <= probe_byte || change > newline_byte {
            break;
        }
        if change < newline_byte {
            let change_charpos = buffer
                .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(change))
                .get();
            let char_offset = change_charpos.checked_sub(row.charpos.max(0) as usize)?;
            debug_assert!(
                char_offset > 0 && char_offset < scan.char_len,
                "a mid-line property change must land strictly inside the line"
            );
            face_boundaries.push(char_offset);
        }
        probe_byte = change;
    }

    Some(AsciiRowPlan {
        line_byte_len: scan.byte_len,
        line_char_len: scan.char_len,
        has_tab: scan.has_tab,
        has_wide: scan.has_wide,
        face_boundaries,
    })
}

/// The realized face of a routed row position, resolved through the SAME
/// seam the buffer pipeline's face checkpoint uses
/// ([`crate::buffer_source::face_resolution::BufferSourceFaceResolutionContext::resolve_at_checkpoint`]
/// drives `FaceResolver::default_base_face_for_origin`, GNU `face_at_pos` in
/// `handle_face_prop`), stamped with the same content-addressed stable id the
/// checkpoint would produce.
pub(crate) fn resolve_routed_position_face<B: LayoutBufferView>(
    buffer: &B,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceAttempt,
    pos: CharPos0,
) -> (FaceId, ResolvedFace) {
    let mut next_check = 0usize;
    let resolved = face_resolver.default_base_face_for_origin(
        Some(buffer),
        &DisplayOrigin::BufferText { charpos: pos },
        &mut next_check,
    );
    let face_id = stable_face_id_for_resolved(face_ids, &resolved);
    (face_id, resolved)
}

/// A routed row face segment: `[start, end)` rendered with `face_id`.
#[derive(Clone, Debug)]
pub(crate) struct RoutedRowFaceSegment {
    pub(crate) start: CharPos0,
    pub(crate) end: CharPos0,
    pub(crate) face_id: FaceId,
    pub(crate) resolved: ResolvedFace,
}

/// Resolve the face segments of a classified row via
/// [`resolve_routed_position_face`] — one segment per property-change stretch,
/// each carrying the realized face id the buffer pipeline's checkpoint
/// resolution produces for that span.
pub(crate) fn plan_row_face_segments<B: LayoutBufferView>(
    buffer: &B,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceAttempt,
    start: CharPos0,
    plan: &AsciiRowPlan,
) -> Vec<RoutedRowFaceSegment> {
    plan.segment_ranges(start)
        .into_iter()
        .map(|(seg_start, seg_end)| {
            let (face_id, resolved) =
                resolve_routed_position_face(buffer, face_resolver, face_ids, seg_start);
            RoutedRowFaceSegment {
                start: seg_start,
                end: seg_end,
                face_id,
                resolved,
            }
        })
        .collect()
}

/// Whether the buffer pipeline's PER-RUN face chain would stamp a different
/// face id on glyphs at `pos` than the checkpoint chain (`expected_face_id`).
///
/// The pipeline resolves each run's face twice: `BufferTextSourceCursor::
/// face_at` merges the effective `face` property over the DEFAULT face
/// (`resolve_face_ref`, keeping the window base id when the merge lands on
/// base content), while the loop checkpoint merges it over the BUFFER default
/// face (`face_at_pos`). The results content-address to the same stable id
/// except when the two default chains diverge (e.g. buffer face remapping of
/// `default`); such rows must stay on the buffer pipeline, whose machinery
/// expresses the divergent item face.
pub(crate) fn routed_segment_item_face_diverges<B: LayoutBufferView>(
    buffer: &B,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceAttempt,
    default_resolved: &ResolvedFace,
    default_face_id: FaceId,
    pos: CharPos0,
    expected_face_id: FaceId,
) -> bool {
    let bytepos = buffer.layout_char_pos_to_emacs_byte_pos(pos);
    let Some(value) =
        LayoutCharPropertyLookup::new(buffer, Value::symbol("face")).text_value_at(buffer, bytepos)
    else {
        // No face property: the run resolves `Inherit` -> the active
        // (checkpoint) face id, which IS `expected_face_id`.
        return false;
    };
    let Some(resolved) =
        face_resolver.resolve_buffer_face_value_over(buffer, default_resolved, &value)
    else {
        // The value contributes nothing: the ref stays `Inherit` -> active.
        return false;
    };
    let item_face_id =
        if crate::display_source_resolver::same_resolved_face(&resolved, default_resolved) {
            default_face_id
        } else {
            stable_face_id_for_resolved(face_ids, &resolved)
        };
    item_face_id != expected_face_id
}

/// One text segment of a routed row: `[start, end)` rendered with `face`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AsciiRowItemSegment {
    pub(crate) start: CharPos0,
    pub(crate) end: CharPos0,
    pub(crate) face: RenderFaceRef,
}

/// A `DisplayItemSource` over one classified plain-ASCII buffer row. Produces
/// exactly the items `BufferTextSourceCursor` would for the same row — one
/// plain `TextRun` per face segment (one for the whole line when properties
/// are constant), then (when the row break is included) the explicit-newline
/// `RowBreak` — mirroring GNU `next_element_from_buffer` yielding the line's
/// characters, re-segmented at each `compute_stop_pos` stop, and then the
/// newline element.
pub(crate) struct BufferAsciiItemSource {
    items: std::collections::VecDeque<DisplayItem>,
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
        Self::from_segments(
            buffer_id,
            buffer,
            &[AsciiRowItemSegment {
                start,
                end: line_end,
                face,
            }],
            Some(face),
        )
    }

    /// Source over the row's face segments plus the newline row break, the
    /// break carrying the face resolved AT the newline (a face span covering
    /// the newline rides onto the appended newline space through the line-end
    /// plan, mirroring the buffer pipeline's row-break face).
    pub(crate) fn with_row_break_segments<B: LayoutBufferView + ?Sized>(
        buffer_id: BufferId,
        buffer: &B,
        segments: &[AsciiRowItemSegment],
        row_break_face: RenderFaceRef,
    ) -> Self {
        Self::from_segments(buffer_id, buffer, segments, Some(row_break_face))
    }

    /// Source over the line text only; the buffer pipeline's own line-break
    /// lifecycle (line-end plan, appended newline space, row transition)
    /// consumes the newline. Used by the routed production render, one
    /// segment per call so each renders under its own active face.
    pub(crate) fn text_only<B: LayoutBufferView + ?Sized>(
        buffer_id: BufferId,
        buffer: &B,
        start: CharPos0,
        line_end: CharPos0,
        face: RenderFaceRef,
    ) -> Self {
        Self::from_segments(
            buffer_id,
            buffer,
            &[AsciiRowItemSegment {
                start,
                end: line_end,
                face,
            }],
            None,
        )
    }

    fn from_segments<B: LayoutBufferView + ?Sized>(
        buffer_id: BufferId,
        buffer: &B,
        segments: &[AsciiRowItemSegment],
        row_break_face: Option<RenderFaceRef>,
    ) -> Self {
        let byte_at = |pos: CharPos0| buffer.layout_char_pos_to_emacs_byte_pos(pos);
        let span = |from: CharPos0, to: CharPos0| {
            SourceSpan::new(
                DisplaySourcePosition::buffer(buffer_id, from, byte_at(from)),
                DisplaySourcePosition::buffer(buffer_id, to, byte_at(to)),
            )
        };

        let mut items = std::collections::VecDeque::with_capacity(segments.len() + 1);
        for segment in segments {
            if segment.end <= segment.start {
                continue;
            }
            let mut bytes = Vec::new();
            buffer.layout_copy_emacs_byte_range_to(
                EmacsByteRange::new(byte_at(segment.start), byte_at(segment.end)),
                &mut bytes,
            );
            let mut text = String::with_capacity(bytes.len());
            let mut offset = 0usize;
            while offset < bytes.len() {
                let (ch, len) = decode_utf8(&bytes[offset..]);
                debug_assert!(
                    len > 0 && ch.len_utf8() == len,
                    "BufferAsciiItemSource requires well-formed UTF-8 row text"
                );
                if len == 0 {
                    break;
                }
                debug_assert!(
                    classify_routed_row_char(ch).is_some(),
                    "BufferAsciiItemSource requires a classified routable row (got {ch:?})"
                );
                text.push(ch);
                offset += len;
            }
            items.push_back(
                DisplayItem::new(
                    span(segment.start, segment.end),
                    segment.face,
                    DisplayItemKind::TextRun(DisplayTextRun::new(text)),
                )
                .with_layout(DisplayItemLayout::default())
                .with_pointer_appearance(None),
            );
        }

        if let Some(break_face) = row_break_face {
            let line_end = segments
                .last()
                .map(|segment| segment.end)
                .unwrap_or(CharPos0::ZERO);
            // Mirrors `BufferTextSourceCursor::next_text_item_with_layout`:
            // the newline's row break carries the line-height policy resolved
            // from the (absent, for a classified row) `line-height` property.
            let row_break = DisplayRowBreak::explicit_newline()
                .with_line_height(DisplayLineHeightPolicy::from_property(None));
            items.push_back(
                DisplayItem::new(
                    span(line_end, line_end.add_len(CharLen::new(1))),
                    break_face,
                    DisplayItemKind::RowBreak(row_break),
                )
                .with_layout(DisplayItemLayout::default())
                .with_pointer_appearance(None),
            );
        }

        Self { items }
    }

    /// The next `TextRun` item without consuming the source (the routed
    /// production render measures the run before committing to the route).
    pub(crate) fn text_item(&self) -> Option<&DisplayItem> {
        self.items
            .front()
            .filter(|item| matches!(item.kind, DisplayItemKind::TextRun(_)))
    }
}

impl crate::display_source::DisplayItemSource for BufferAsciiItemSource {
    fn next_item(
        &mut self,
        _context: &mut crate::display_source::DisplaySourceContext<'_>,
    ) -> Option<DisplayItem> {
        self.items.pop_front()
    }
}

/// Opt-in gate for the routed acquisition: `NEOMACS_ROW_ITEM_ROUTE=ascii`.
/// Default OFF — with the flag unset the buffer pipeline is untouched and the
/// classifier never runs (a single lazy boolean check per row).
pub(crate) fn row_item_route_ascii_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED
        .get_or_init(|| std::env::var("NEOMACS_ROW_ITEM_ROUTE").is_ok_and(|value| value == "ascii"))
}

/// Test-only engagement proof: rows actually rendered through the routed
/// item-renderer acquisition in this process. Lets the flag-on suite run
/// assert the route is exercised rather than silently unreachable.
#[cfg(test)]
pub(crate) static ROUTED_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the multi-face extension: routed rows that
/// rendered as MORE than one face segment.
#[cfg(test)]
pub(crate) static ROUTED_SEGMENTED_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the tab extension: routed rows containing
/// at least one TAB.
#[cfg(test)]
pub(crate) static ROUTED_TAB_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the wide-char extension: routed rows
/// containing at least one 2-column char.
#[cfg(test)]
pub(crate) static ROUTED_WIDE_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn note_routed_row(plan: &AsciiRowPlan) {
    #[cfg(test)]
    {
        ROUTED_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if plan.is_segmented() {
            ROUTED_SEGMENTED_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if plan.has_tab {
            ROUTED_TAB_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if plan.has_wide {
            ROUTED_WIDE_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    #[cfg(not(test))]
    let _ = plan;
}

/// Outcome of an attempted item-renderer row acquisition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsciiRowRouteOutcome {
    /// The row is not eligible (or measurement rejected it); the buffer
    /// pipeline proceeds unchanged.
    NotRouted,
    /// The row's text was rendered through the unified item renderer; the
    /// walk resumes at the line's newline, which the buffer pipeline's own
    /// line-break lifecycle consumes.
    Rendered,
    /// The renderer reported a stop; the visible loop must end (mirrors the
    /// buffer pipeline mapping a failed append to Stop).
    Stopped,
}

impl<'rows, 'emit, 'surface>
    crate::buffer_source::loop_state::BufferSourceLoopMutableState<'rows, 'emit, 'surface>
{
    /// Attempt to acquire and render the row starting at the current walk
    /// position through the unified item renderer (`NEOMACS_ROW_ITEM_ROUTE=
    /// ascii` route). Only rows [`classify_row_acquisition`] approves are
    /// taken; every candidate face segment is probed (checkpoint face,
    /// per-run face-chain agreement, box-free, natural-measurement fit — the
    /// same measurement the buffer pipeline's whole-run decision uses)
    /// BEFORE any loop state is mutated; everything else falls back to the
    /// buffer pipeline. The only probe-side effects are content-addressed
    /// stable-id mints, which the pipeline performs identically for the same
    /// row.
    ///
    /// The bookkeeping the buffer pipeline performs per item is either
    /// replicated (per-segment face checkpoint via `resolve_at_checkpoint`,
    /// active-face row-extend scope, resolved-face memo) or provably idle
    /// for a classified row: cursor capture (point excluded),
    /// trailing-whitespace tracking and word-wrap candidates (both disabled
    /// by the classifier), overlay-string splits (no overlays).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_render_ascii_row_via_item_renderer<B: LayoutBufferView>(
        &mut self,
        loop_context: crate::buffer_source::loop_context::BufferSourceLoopRequestContext,
        face_resolution_context: crate::buffer_source::face_resolution::BufferSourceFaceResolutionContext<'_, B>,
        source_walk: &mut crate::buffer_source::walk::BufferSourceWalk<'_, B>,
        text: &[u8],
        params: &crate::types::WindowParams,
        active_face_state: &mut crate::display_row::face_state::DisplayRowActiveFaceState,
        buffer: &B,
    ) -> AsciiRowRouteOutcome {
        use crate::buffer_source::item_append::BufferSourceRowAppendContext;
        use crate::display_row::append_context::DisplayRowAppendKind;
        use crate::display_source_append_plan::DisplaySourceAppendRenderPolicy;

        if source_walk.has_pending_render_items() {
            return AsciiRowRouteOutcome::NotRouted;
        }

        let position = self.progress.row_position();
        let row = RowRouteRowStart {
            text,
            byte_idx: self.progress.byte_idx(),
            charpos: self.progress.charpos(),
            text_start_byte: loop_context.text_start_byte(),
        };
        let fit = RowRouteFit {
            start_x_px: position.x_px(),
            start_col: position.col(),
            char_width_px: params.char_width,
            right_edge_px: self.append_surface.right_edge(),
            tab_policy: self.append_surface.tab_policy(),
        };
        let policy = RowRouteWindowPolicy {
            point_charpos: loop_context.point_charpos(),
            hscroll_active: params.hscroll != 0 || self.hscroll_skip.should_skip(),
            selective_display: loop_context.selective_display(),
            word_wrap: params.word_wrap || self.word_wrap.is_enabled(),
            show_trailing_whitespace: params.show_trailing_whitespace
                || self.trailing_whitespace.is_enabled(),
        };
        let Some(plan) = plan_ascii_row(buffer, row, fit, policy) else {
            return AsciiRowRouteOutcome::NotRouted;
        };

        let start = CharPos0::new(row.charpos.max(0) as usize);
        let ranges = plan.segment_ranges(start);
        let line_end = ranges.last().map(|(_, end)| *end).unwrap_or(start);

        // ---- Probe phase: no loop-state mutation. Resolve every segment's
        // checkpoint face (the loop already resolved segment 0's), refuse
        // divergent per-run face chains and box faces on the NEW multi-face
        // class, and verify the strict natural-measurement fit segment by
        // segment at its running position.
        struct ProbedSegment {
            start: CharPos0,
            end: CharPos0,
            active: crate::display_row::face_state::DisplayRowActiveFaceState,
        }
        let mut probed: Vec<ProbedSegment> = Vec::with_capacity(ranges.len());
        for (index, (seg_start, seg_end)) in ranges.iter().enumerate() {
            let active = if index == 0 {
                active_face_state.clone()
            } else {
                let (face_id, resolved) = resolve_routed_position_face(
                    buffer,
                    face_resolution_context.face_resolver(),
                    self.face_ids,
                    *seg_start,
                );
                face_resolution_context.probe_measured_active_face(
                    &mut self.source_render.reborrow(),
                    face_id,
                    resolved,
                )
            };
            // Box faces carry per-run edge bookkeeping (GNU
            // start_of_box_run_p / face_box_p) the routed render does not
            // replicate; keep the multi-face class box-free.
            if plan.is_segmented() && active.resolved_face().box_type != 0 {
                return AsciiRowRouteOutcome::NotRouted;
            }
            // The pipeline stamps glyphs with the PER-RUN face chain; refuse
            // the row when it would diverge from the checkpoint chain (e.g.
            // buffer face remapping of default).
            if routed_segment_item_face_diverges(
                buffer,
                face_resolution_context.face_resolver(),
                self.face_ids,
                face_resolution_context.default_resolved(),
                face_resolution_context.default_face_id(),
                *seg_start,
                active.face_id(),
            ) {
                return AsciiRowRouteOutcome::NotRouted;
            }
            probed.push(ProbedSegment {
                start: *seg_start,
                end: *seg_end,
                active,
            });
        }

        let geometry = *self.row_geometry;
        let mut probe_position = position;
        for segment in &probed {
            let mut source = BufferAsciiItemSource::text_only(
                loop_context.buffer_id(),
                buffer,
                segment.start,
                segment.end,
                RenderFaceRef::FaceId(segment.active.face_id()),
            );
            let Some(text_item) = source.text_item().cloned() else {
                return AsciiRowRouteOutcome::NotRouted;
            };
            let append_context = BufferSourceRowAppendContext::from_active_face_row(
                buffer,
                loop_context.buffer_id(),
                self.append_surface,
                &segment.active,
                0.0,
                loop_context.char_height(),
                self.face_ids.clone(),
            );
            // Advance-based measurement: tab expansion depends on the pen x
            // and 2-col chars advance two cells, so the measured END position
            // (x AND col) seeds the next segment's probe exactly as the
            // pipeline's own natural walk would.
            let measured = {
                let mut measure = self.source_render.measure_state();
                append_context.measure_source_display_item_advance_naturally(
                    &geometry,
                    &mut measure,
                    &text_item,
                    probe_position,
                    DisplayRowAppendKind::SourceText,
                )
            };
            let Some(end_position) = measured else {
                return AsciiRowRouteOutcome::NotRouted;
            };
            probe_position = end_position;
            // Strict fit at every segment boundary: any borderline row
            // (exact fill) keeps the buffer pipeline.
            if !(probe_position.x_px() < self.append_surface.right_edge()) {
                return AsciiRowRouteOutcome::NotRouted;
            }
        }

        // ---- Commit phase: render segment by segment, replaying the
        // pipeline's per-iteration bookkeeping. Segment 0's face checkpoint
        // already ran in the visible loop; each later segment start IS the
        // next property change, so `resolve_at_checkpoint` fires there
        // exactly as the pipeline's next iteration would (installing the
        // measured face, including row extents, scoping row-extend/box).
        let mut render_position = position;
        for (index, segment) in probed.iter().enumerate() {
            if index > 0 {
                face_resolution_context.resolve_at_checkpoint_with_source_state(
                    &mut self.source_render.reborrow(),
                    self.face_scan,
                    self.face_ids,
                    active_face_state,
                    self.row_geometry,
                    self.row_extend,
                    self.box_face,
                    render_position.x_px(),
                    segment.start.get() as i64,
                );
                debug_assert_eq!(
                    active_face_state.face_id(),
                    segment.active.face_id(),
                    "probe and checkpoint face resolution must agree"
                );
            }
            // Per-item bookkeeping the buffer pipeline would perform for
            // this run (item_render.rs): remember the resolved active face
            // for later splits, and scope the row-extend fill to the row.
            source_walk.remember_resolved_source_face_if_absent(
                active_face_state.face_id(),
                active_face_state.resolved_face(),
            );
            if let Some(fill) = active_face_state.row_extend_fill() {
                self.row_extend
                    .activate(self.row_geometry.current_row_marker(), fill);
            } else {
                self.row_extend.clear();
            }

            let mut source = BufferAsciiItemSource::text_only(
                loop_context.buffer_id(),
                buffer,
                segment.start,
                segment.end,
                RenderFaceRef::FaceId(active_face_state.face_id()),
            );
            let append_context = BufferSourceRowAppendContext::from_active_face_row(
                buffer,
                loop_context.buffer_id(),
                self.append_surface,
                active_face_state,
                0.0,
                loop_context.char_height(),
                self.face_ids.clone(),
            );
            let geometry = *self.row_geometry;
            let mut render_policy = DisplaySourceAppendRenderPolicy::natural();
            let mut source_state =
                crate::display_row::source_state::DisplayRowSourceState::default();
            let Some(append_progress) = append_context.render_display_item_source_to_text_row(
                &geometry,
                &mut self.source_render.reborrow(),
                &mut source,
                &mut source_state,
                render_position,
                DisplayRowAppendKind::SourceText,
                &mut render_policy,
            ) else {
                return AsciiRowRouteOutcome::Stopped;
            };
            render_position = append_progress.end();
            self.progress.apply_row_position(render_position);
        }

        self.progress.max_charpos(line_end.get() as i64);
        self.progress
            .set_byte_idx(row.byte_idx + plan.line_byte_len());
        note_routed_row(&plan);
        AsciiRowRouteOutcome::Rendered
    }
}

#[cfg(test)]
#[path = "row_route_test.rs"]
mod tests;
