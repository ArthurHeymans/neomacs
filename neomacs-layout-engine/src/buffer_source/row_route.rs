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
//!   trivial: single-byte printable ASCII only, no display/mouse-face/
//!   line-height properties in range, no display table, L2R (guaranteed by
//!   ASCII-only content), and either the whole line fits without
//!   continuation or truncation and ends in a real newline, or — phase 2f —
//!   the line overflows and the route covers only its maximal fitting
//!   prefix, handing the walk back to the pipeline at the first char that
//!   does not fit so the pipeline's own overflow machinery (truncation skip
//!   / continuation transition, row flags, carry-over bookkeeping) decides
//!   wrap-vs-truncate unchanged. Composition refuses through the
//!   pipeline's OWN predicates (phase 2e): a char the shared writer would
//!   compose into the previous glyph (`composition::continues_cluster` /
//!   `continues_complex_run` over the scan's mirror of the row tail) and a
//!   static `composition` text property the pipeline's replacement
//!   predicate parses (`composition_display_text_for_property`) both keep
//!   the buffer pipeline; an inert composition prop renders literally and
//!   routes.
//!   FACE-affecting properties (`face`, `font-lock-face`, `fontified`
//!   boundaries) are allowed: they segment the row at property-change
//!   positions exactly like GNU `compute_stop_pos` bounds the iterator's
//!   text runs and `handle_face_prop` re-resolves the face at each stop.
//!   Overlays intersecting the row are allowed when they carry ONLY
//!   face-affecting properties ([`ROUTE_SAFE_OVERLAY_PROPS`]): their faces
//!   merge through the same checkpoint resolver seam (GNU
//!   `face_at_buffer_position`'s ascending-priority overlay loop) and their
//!   starts/ends segment the row like GNU `next_overlay_change` folded into
//!   `compute_stop_pos`. Overlay before/after-strings are NOT expressible on
//!   this path (they are Lisp-string-sourced runs with their own row
//!   lifecycle, GNU `load_overlay_strings`/`push_it`); any intersecting
//!   overlay carrying one refuses the route.
//!   Plain-elision `invisible` text (phase 2d) is expressible: hidden spans
//!   simply drop chars, so the routed source emits visible-segment TextRuns
//!   whose charpos bookkeeping jumps the gap, exactly like the pipeline's
//!   invisible checkpoint `skip_chars_until` (GNU `handle_invisible_prop`
//!   advancing `IT_CHARPOS`). The inexpressible invisible sub-cases refuse:
//!   ellipsis (inserts `...` glyphs with their own face/provenance rules),
//!   runs covering the newline (line-structure change), row-start runs
//!   (consumed by the loop checkpoint before the route), overlay-sourced
//!   invisibility (2c allow-list). `display` properties and `(space …)`
//!   specs stay refused whole (rungs 2-3 decision): replacement rendering
//!   rides the Lisp-string session with covered-charpos glyph provenance the
//!   single-row TextRun probe/commit cannot express.
//! * [`BufferAsciiItemSource`] is the `DisplayItemSource` for such a row: it
//!   produces exactly the items `BufferTextSourceCursor` would — one plain
//!   `TextRun` per face segment (one for the whole line when no property
//!   changes in range), then the explicit-newline row break.
//!
//! Rows carrying point are deliberately excluded: cursor capture is a
//! documented buffer-pipeline responsibility (see the cursor-capture note in
//! `row_lifecycle.rs`), mirroring GNU `set_cursor_from_row` operating on
//! buffer positions only.

use crate::composition::{continues_cluster, continues_complex_run, needs_complex_shaping};
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
use crate::types::LineWrapMode;
use crate::unicode::{decode_utf8, is_regional_indicator};
use neomacs_display_protocol::types::FaceId;
use neovm_core::buffer::{BufferId, CharLen, CharPos0, EmacsBytePos, EmacsByteRange};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::composite::composition_display_text_for_property;

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
    /// The window's effective wrap mode (GNU `it->line_wrap`, minus
    /// WORD_WRAP which the `word_wrap` flag above refuses outright). It does
    /// not change WHAT the route renders — an over-wide line always routes
    /// only its fitting prefix and hands the walk back BEFORE the first char
    /// that does not fit, so the pipeline's own truncation/continuation
    /// machinery makes the wrap-vs-truncate decision — but it labels the
    /// routed class for engagement accounting.
    pub(crate) wrap_mode: LineWrapMode,
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

/// Pixel-fit inputs. A whole-line plan must hold the line WITHOUT
/// continuation or truncation, applied strictly (a line exactly filling the
/// row is NOT eligible — its line end interacts with continuation policy);
/// an overflow-prefix plan (phase 2f) covers the maximal fitting prefix of
/// an over-wide line instead. Either way the routed render re-verifies with
/// the same natural measurement the buffer pipeline uses before committing.
/// The tab
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
/// a property-constant line. `elided` are the CHAR-offset `[start, end)`
/// ranges hidden by plain (no-ellipsis) `invisible` text properties, in
/// ascending disjoint order — the routed render skips them entirely, exactly
/// as the pipeline's invisible checkpoint `skip_chars_until` does (GNU
/// `handle_invisible_prop` advancing `IT_CHARPOS` past the run). `composed`
/// are the CHAR offsets of zero-width extenders the shared writer merges
/// into their preceding base glyph (phase 2e rung 2) — they occupy no
/// column and produce no glyph of their own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AsciiRowPlan {
    line_byte_len: usize,
    line_char_len: usize,
    has_tab: bool,
    has_wide: bool,
    has_overlay: bool,
    face_boundaries: Vec<usize>,
    elided: Vec<(usize, usize)>,
    composed: Vec<usize>,
    line_end: RoutedRowLineEnd,
}

/// How a routed row's coverage ends (phase 2f).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoutedRowLineEnd {
    /// The line's real newline: the plan covers the whole line, strictly
    /// fitting inside the row; the buffer pipeline's line-break lifecycle
    /// consumes the newline afterwards.
    Newline,
    /// The line overflows the row (GNU `display_line`'s "glyph doesn't fit"
    /// branch, xdisp.c:26221): the plan covers only the MAXIMAL FITTING
    /// PREFIX — every covered char satisfies the pipeline's own fit rule
    /// (`DisplayRowTextOverflowDecision::for_char`: `x + advance <=
    /// right_edge`) — and the routed render hands the walk back to the
    /// buffer pipeline AT the first char that does not fit, BEFORE any
    /// wrap-vs-truncate decision. The pipeline's own overflow machinery
    /// (`overflow.rs` truncation skip / continuation transition, row flags,
    /// continuation rows, fringe indicators) then runs unchanged, which is
    /// what keeps the multi-row carry-over bookkeeping byte-identical.
    OverflowHandoff,
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

    #[cfg(test)]
    pub(crate) fn has_overlay(&self) -> bool {
        self.has_overlay
    }

    #[cfg(test)]
    pub(crate) fn elided(&self) -> &[(usize, usize)] {
        &self.elided
    }

    /// Whether the row elides invisible spans.
    pub(crate) fn has_elision(&self) -> bool {
        !self.elided.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn composed(&self) -> &[usize] {
        &self.composed
    }

    /// Whether the row contains a composed grapheme cluster (a zero-width
    /// extender merged into its base glyph).
    pub(crate) fn has_composed(&self) -> bool {
        !self.composed.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn line_end(&self) -> RoutedRowLineEnd {
        self.line_end
    }

    /// Whether this plan covers only the fitting prefix of an over-wide line
    /// and hands the walk back to the pipeline at the first non-fitting char.
    pub(crate) fn is_overflow_handoff(&self) -> bool {
        self.line_end == RoutedRowLineEnd::OverflowHandoff
    }

    /// Whether the row renders as more than one run (face segments or
    /// elision gaps splitting the line).
    pub(crate) fn is_segmented(&self) -> bool {
        !self.face_boundaries.is_empty() || !self.elided.is_empty()
    }

    /// The `[start, end)` char ranges of the row's VISIBLE face segments, in
    /// row order: the line minus the elided spans, split at each face
    /// boundary that falls strictly inside a visible stretch (boundaries at
    /// an elided edge coincide with the gap and split nothing; boundaries
    /// inside a hidden span never render). A property-constant fully-visible
    /// line yields one range covering the line.
    pub(crate) fn segment_ranges(&self, start: CharPos0) -> Vec<(CharPos0, CharPos0)> {
        let mut visible: Vec<(usize, usize)> = Vec::with_capacity(self.elided.len() + 1);
        let mut cursor = 0usize;
        for &(hidden_start, hidden_end) in &self.elided {
            if hidden_start > cursor {
                visible.push((cursor, hidden_start));
            }
            cursor = cursor.max(hidden_end);
        }
        if cursor < self.line_char_len {
            visible.push((cursor, self.line_char_len));
        }

        let mut ranges = Vec::with_capacity(visible.len() + self.face_boundaries.len());
        for (visible_start, visible_end) in visible {
            let mut seg_start = visible_start;
            for &boundary in &self.face_boundaries {
                if boundary > seg_start && boundary < visible_end {
                    ranges.push((
                        start.add_len(CharLen::new(seg_start)),
                        start.add_len(CharLen::new(boundary)),
                    ));
                    seg_start = boundary;
                }
            }
            ranges.push((
                start.add_len(CharLen::new(seg_start)),
                start.add_len(CharLen::new(visible_end)),
            ));
        }
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

/// Classify one STANDALONE char for the routed row class (the scan resolves
/// composition first: a char the pipeline would compose into the previous
/// glyph never reaches this ladder). Accepted: TAB, printable ASCII, and
/// printable non-ASCII chars whose display width is unambiguously 1 or 2
/// columns per the SAME width source the buffer pipeline advances by
/// (`neovm_core::encoding::char_width`, the GNU default `char-width-table`,
/// via `base_width_cols`). Refused (each has pipeline machinery the routed
/// render does not replicate):
/// * control chars other than TAB, and every non-Text classification
///   (`^X` caret runs, `\`+octal escapes, glyphless boxes — this arm also
///   catches the zero-width chars the glyphless policy does NOT preserve
///   for composition, e.g. ZWSP);
/// * regional indicators — the shared writer's width model
///   (`composition::base_width_cols`) forces them to 2 columns in
///   anticipation of flag-pair composition, diverging from the plain
///   `char_width` cell model this classifier fits with;
/// * contextual-shaping script chars (Arabic, Indic, …) — every such char
///   can START a run the pipeline shapes as a unit
///   (`composition::continues_complex_run` decides membership only at the
///   NEXT char), so run entry refuses here;
/// * nobreak spaces/hyphens — their display consults the
///   `nobreak-char-display` setting per char (GNU xdisp.c:8594);
/// * anything the shared width table does not size at exactly 1 or 2 cols
///   (which also refuses zero-width cluster extenders defensively — the
///   scan's compose branch normally intercepts them first).
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
    if is_regional_indicator(ch as u32)
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

/// Whether the pipeline's shared writer would COMPOSE `ch` into the
/// previously produced glyph instead of appending a standalone one. This is
/// the actual seam predicate, not a parallel heuristic: the writer's advance
/// ladder (`DisplayRowTextNaturalAdvanceKind::for_tail`, display_row/
/// append_context.rs) routes a text char to `ClusterContinuation` /
/// `ComplexRunMember` — merging it into a `Composite` glyph — on exactly
/// these two checks, fed by the row's `last_text_cluster_tail_in_glyphs`
/// view, which the scan mirrors as `tail`.
fn routed_char_would_compose(ch: char, tail: Option<(char, bool)>) -> bool {
    continues_cluster(ch, tail) || continues_complex_run(ch, tail)
}

/// Whether `ch` is an extender the routed composite class accepts: a
/// zero-width cluster extender (combining marks, variation selectors, the
/// enclosing keycap) that the shared writer merges into the previous glyph
/// WITHOUT advancing the pen. Grounded in the same width source the writer's
/// composed-cluster metric sums (`composed_cluster_cols` = `string_width`,
/// GNU `cmp->width`): a zero-width extender leaves the cluster at its base's
/// columns, so the scan's fit walk and the writer agree exactly. ZWJ/ZWNJ
/// are excluded — a joiner makes the FOLLOWING char compose too
/// (`continues_cluster`'s prev-is-ZWJ arm), an open-ended sequence shape
/// (emoji ZWJ sequences) that stays refused.
fn routed_composable_extender(ch: char) -> bool {
    !matches!(ch as u32, 0x200C | 0x200D) && neovm_core::encoding::char_width(ch) == 0
}

/// What the last scanned char left in the row for a following extender to
/// merge into, mirroring the writer's merge targets: `Simple` is a 1-column
/// non-padding Char glyph (or a Composite already grown from one) — the ONLY
/// shape the routed class lets an extender merge into. A tab's stretch glyph
/// and a wide char's base+padding pair are not routable merge targets (the
/// writer would push an orphan glyph or merge into the padding cell), and at
/// the row start there is nothing to merge into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoutedScanMergeTarget {
    None,
    Simple,
    Wide,
}

/// A scanned routable line: `byte_len` bytes / `char_len` chars of routed
/// chars terminated by a real `\n` inside `text`, fitting strictly inside
/// the row. `composed` are the CHAR offsets of extenders the shared writer
/// merges into their preceding base glyph (ascending).
#[derive(Clone, Debug)]
struct RoutedLineScan {
    byte_len: usize,
    char_len: usize,
    has_tab: bool,
    has_wide: bool,
    composed: Vec<usize>,
    line_end: RoutedRowLineEnd,
}

/// Scan for a routable line at `byte_idx`: at least one char accepted by
/// the routed ladder, either terminated by a real `\n` inside `text` with
/// the pen walk ending STRICTLY inside the right edge
/// ([`RoutedRowLineEnd::Newline`]), or — phase 2f — overflowing the row, in
/// which case the scan covers the maximal fitting prefix and stops at the
/// first char whose advance would cross the right edge
/// ([`RoutedRowLineEnd::OverflowHandoff`]). The prefix cut mirrors the
/// pipeline's fit rule (`x + advance <= right_edge` fits), with one
/// deliberate conservatism: a TAB whose expansion crosses the edge ends the
/// prefix too (the pipeline treats a tab as always fitting and clips it;
/// handing the tab back keeps that clip on the pipeline's own append).
///
/// The walk advances the pen exactly as the pipeline's natural advance does
/// for uniform `char_width_px` cells — tabs through the append surface's
/// `DisplayTabPolicy::advance_from` (GNU `next_tab_x`), wide chars by two
/// cells — and mirrors the writer's composition ladder in decision order:
/// a Text-class char is first tested against the SAME compose predicate the
/// writer applies ([`routed_char_would_compose`] over the running `tail`,
/// the scan's mirror of `last_text_cluster_tail_in_glyphs`). A composing
/// char is accepted only in the rung-2 routed class — a zero-width extender
/// ([`routed_composable_extender`]) merging into a simple 1-col base in this
/// row ([`RoutedScanMergeTarget::Simple`]); it advances the pen by ZERO
/// (the writer's merge appends no glyph and the composed-cluster metric
/// `composed_cluster_cols` counts its width as 0). Every other composing
/// shape — joiners, 2-col extenders, shaped-script runs, extenders on
/// wide/tab/row-start tails — refuses, keeping those clusters on the buffer
/// pipeline deliberately. A line exactly filling the row keeps the buffer
/// pipeline too (continuation/truncation policy owns that edge); the routed
/// render re-verifies with the pipeline's own per-face natural measurement
/// before committing. `tail` evolves as the writer's row view would: a
/// pushed char becomes `(ch, lone-regional-indicator)`, a merged extender
/// becomes the cluster's last char, a tab's stretch glyph clears it.
fn routed_line_scan(text: &[u8], byte_idx: usize, fit: RowRouteFit<'_>) -> Option<RoutedLineScan> {
    let mut idx = byte_idx;
    let mut char_len = 0usize;
    let mut has_tab = false;
    let mut has_wide = false;
    let mut composed = Vec::new();
    let mut x_px = fit.start_x_px;
    let mut col = fit.start_col;
    let mut tail: Option<(char, bool)> = None;
    let mut merge_target = RoutedScanMergeTarget::None;
    // The maximal-fitting-prefix cut for an over-wide line: every scanned
    // char so far fits; the char at `idx` would cross the right edge, so the
    // routed coverage ends here and the pipeline resumes at `idx`.
    let overflow_prefix =
        |idx: usize, char_len: usize, has_tab: bool, has_wide: bool, composed: Vec<usize>| {
            if char_len == 0 {
                return None;
            }
            Some(RoutedLineScan {
                byte_len: idx - byte_idx,
                char_len,
                has_tab,
                has_wide,
                composed,
                line_end: RoutedRowLineEnd::OverflowHandoff,
            })
        };
    while idx < text.len() {
        if text[idx] == b'\n' {
            if char_len == 0 || x_px >= fit.right_edge_px {
                return None;
            }
            return Some(RoutedLineScan {
                byte_len: idx - byte_idx,
                char_len,
                has_tab,
                has_wide,
                composed,
                line_end: RoutedRowLineEnd::Newline,
            });
        }
        let (ch, consumed) = decode_utf8(&text[idx..]);
        // Reject malformed UTF-8 (decode yields U+FFFD over fewer bytes than
        // the char re-encodes to): raw bytes have their own display path.
        if consumed == 0 || ch.len_utf8() != consumed {
            return None;
        }
        // Pipeline decision order: non-Text chars break the text run into
        // their own items BEFORE any composition (the classify arm below
        // refuses those), while a Text-class char consults the writer's
        // compose ladder first.
        if classify_text_source_char(ch) == TextSourceCharClassification::Text
            && routed_char_would_compose(ch, tail)
        {
            if !(routed_composable_extender(ch) && merge_target == RoutedScanMergeTarget::Simple) {
                return None;
            }
            // The merge appends no glyph and advances nothing; the cluster's
            // tail becomes the extender (writer: the Composite's last char).
            composed.push(char_len);
            tail = Some((ch, false));
            char_len += 1;
            idx += consumed;
            continue;
        }
        match classify_routed_row_char(ch)? {
            RoutedRowCharAdvance::Tab => {
                let tab = fit
                    .tab_policy
                    .advance_from(DisplayRowPosition::new(x_px, col), fit.char_width_px);
                // A tab crossing the right edge is clipped in place by the
                // pipeline (GNU xdisp.c:26390, tab never split): end the
                // routed prefix BEFORE it and let the pipeline append it.
                if x_px + tab.pixel_width > fit.right_edge_px {
                    return overflow_prefix(idx, char_len, has_tab, has_wide, composed);
                }
                has_tab = true;
                x_px += tab.pixel_width;
                col += tab.width_cols;
                // A tab renders a Stretch glyph: the writer's cluster-tail
                // view yields None over it.
                tail = None;
                merge_target = RoutedScanMergeTarget::None;
            }
            RoutedRowCharAdvance::Cols(cols) => {
                // The pipeline's fit rule: a char fits when its END lands at
                // or inside the right edge (`x + advance <= right_edge`,
                // DisplayRowTextOverflowDecision::for_char). The first char
                // crossing the edge — including a 2-col char straddling it —
                // ends the routed prefix; the pipeline's overflow machinery
                // consumes it (truncation skip / continuation transition).
                if x_px + f32::from(cols) * fit.char_width_px > fit.right_edge_px {
                    return overflow_prefix(idx, char_len, has_tab, has_wide, composed);
                }
                has_wide |= cols == 2;
                x_px += f32::from(cols) * fit.char_width_px;
                col += usize::from(cols);
                tail = Some((ch, is_regional_indicator(ch as u32)));
                merge_target = if cols == 1 {
                    RoutedScanMergeTarget::Simple
                } else {
                    RoutedScanMergeTarget::Wide
                };
            }
        }
        char_len += 1;
        idx += consumed;
    }
    // End of buffer without a newline: the end-of-buffer tail has its own
    // buffer-pipeline lifecycle (cursor at EOB, empty-line indicators).
    None
}

/// Overlay properties the routed row class accepts on an intersecting
/// overlay. `face` merges through the SAME resolver seam the pipeline's
/// checkpoint uses (GNU `face_at_buffer_position`'s ascending-priority
/// overlay loop), `priority` orders that merge, and `evaporate` is
/// buffer-maintenance-only. EVERYTHING else refuses the route: before/
/// after-strings inject Lisp-string runs (GNU `load_overlay_strings`),
/// `display`/`invisible` rewrite content, `mouse-face`/`line-prefix`/
/// `line-height` and friends have pipeline machinery, `window` restricts
/// applicability per window, and `category` indirects to arbitrary props.
/// Unknown properties are conservatively refused (allow-list, not
/// deny-list).
const ROUTE_SAFE_OVERLAY_PROPS: [&str; 3] = ["face", "priority", "evaporate"];

/// The overlay facts of a candidate row: whether any overlay intersects it
/// and the overlay start/end CHAR boundaries strictly inside the line.
struct RoutedRowOverlayScan {
    has_overlay: bool,
    boundaries: Vec<usize>,
}

/// Scan the overlays intersecting `[start_byte, newline_byte]` (touching
/// endpoints included: an overlay ending at the row start or starting at the
/// newline can anchor strings there). Returns `None` — refusing the route —
/// when any intersecting overlay carries a property outside
/// [`ROUTE_SAFE_OVERLAY_PROPS`]. Boundary positions mirror GNU
/// `next_overlay_change` feeding `compute_stop_pos`: every overlay start or
/// end strictly inside the line becomes a face-segment boundary (an empty
/// overlay contributes its single position).
fn routed_row_overlay_scan<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    row_charpos: i64,
    start_byte: usize,
    newline_byte: usize,
) -> Option<RoutedRowOverlayScan> {
    let overlays = buffer.layout_overlays();
    let mut scan = RoutedRowOverlayScan {
        has_overlay: false,
        boundaries: Vec::new(),
    };
    if overlays.is_empty() {
        return Some(scan);
    }
    for overlay in overlays.overlays_in_gnu_lists_order() {
        let (Some(ov_start), Some(ov_end)) = (
            overlays.overlay_start_emacs_byte_pos(overlay),
            overlays.overlay_end_emacs_byte_pos(overlay),
        ) else {
            continue;
        };
        let (ov_start, ov_end) = (ov_start.get(), ov_end.get());
        if ov_start > newline_byte || ov_end < start_byte {
            continue;
        }
        // Every property of an intersecting overlay must be on the
        // allow-list; a non-symbol key or malformed plist refuses too.
        let plist = overlays.overlay_plist(overlay)?;
        let mut tail = plist;
        while tail.is_cons() {
            let prop = tail.cons_car();
            let rest = tail.cons_cdr();
            if !rest.is_cons() {
                return None;
            }
            let name = prop.as_symbol_name()?;
            if !ROUTE_SAFE_OVERLAY_PROPS
                .iter()
                .any(|allowed| *allowed == name)
            {
                return None;
            }
            tail = rest.cons_cdr();
        }
        scan.has_overlay = true;
        for boundary in [ov_start, ov_end] {
            if boundary > start_byte && boundary < newline_byte {
                let char_offset = buffer
                    .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(boundary))
                    .get()
                    .checked_sub(row_charpos.max(0) as usize)?;
                scan.boundaries.push(char_offset);
            }
        }
    }
    Some(scan)
}

/// Text properties that influence acquisition or the line end beyond faces.
/// Any of these present anywhere on the line (or its newline) sends the row
/// to the buffer pipeline. Face-affecting properties (`face`,
/// `font-lock-face`, `fontified` boundaries) are NOT hazards: they only
/// segment the row and are handled by the routed face resolution. Properties
/// are constant between change positions, so probing each segment start (and
/// the newline, when a change lands on it) covers the whole row. `invisible`
/// is not on this list since phase 2d: the plain-elision sub-case is routed
/// and the inexpressible sub-cases (ellipsis, newline-spanning, row-start,
/// overlay-sourced) refuse through [`routed_row_elision_scan`]. `display`
/// stays a hazard EVERYWHERE, including inside hidden spans, mirroring GNU's
/// handler order where a replacing `display` spec beats `invisible`
/// (it_props: display before invisible + HANDLED_RETURN). `composition` is
/// not on this list since phase 2e: its refusal is grounded in the
/// pipeline's own replacement predicate ([`routed_composition_prop_replaces`]),
/// so an inert (unparseable) prop no longer refuses.
const ROUTE_HAZARD_TEXT_PROPS: [&str; 3] = ["display", "mouse-face", "line-height"];

/// Whether a static `composition` text property at `probe` would REPLACE its
/// covered chars in the pipeline. This is the same predicate the pipeline's
/// item production applies (`BufferTextSourceCursor::next_text_item_with_layout`
/// -> `composition_display_text_for_property`, the neomacs stand-in for GNU
/// `handle_composition_prop`'s `composition_valid_p` gate): a prop that
/// parses to display text composes — the row refuses; a prop the predicate
/// rejects renders its chars literally through the ordinary text run and
/// stays routable. Refusal here is deliberately extent-agnostic (the
/// pipeline additionally requires the composition to fit inside the run and
/// walk bounds); refusing the superset is always safe.
fn routed_composition_prop_replaces<B: LayoutBufferView>(buffer: &B, probe: EmacsBytePos) -> bool {
    buffer
        .layout_text_prop_at_emacs_byte_pos(probe, Value::symbol("composition"))
        .is_some_and(|prop| composition_display_text_for_property(prop).is_some())
}

/// Scan the `[row_charpos, newline_charpos)` line for invisible text through
/// the SAME semantics the pipeline's invisible checkpoint consumes
/// (`RustTextPropAccess::check_invisible`: overlay value shadows the text
/// property, values judged against `buffer-invisibility-spec`, adjacent
/// hidden runs collapsed with the ENTRY run's ellipsis flag). Returns the
/// hidden CHAR-offset ranges — the expressible plain-elision class — or
/// `None`, refusing the route, when a hidden run:
/// * shows an ellipsis (the pipeline appends `...` glyphs with their own
///   face/provenance rules, GNU `setup_for_ellipsis`);
/// * starts AT the row start (the visible loop's invisible checkpoint
///   consumes it BEFORE the route attempt; the walk then resumes mid-line —
///   classifying it here keeps direct classification aligned with the
///   production ordering);
/// * covers the newline (hiding the line end joins buffer lines into one
///   display row — a line-structure change; a run ending exactly AT the
///   newline keeps it visible and is fine);
/// * fails to advance (defensive: a skip that does not move would loop).
///
/// The scan walks exactly the checkpoint cadence: probe, jump to the
/// returned `next_visible`, re-probe — the same positions the pipeline's
/// `InvisibleTextScanCheckpoint` re-checks at.
fn routed_row_elision_scan<B: LayoutBufferView>(
    buffer: &B,
    row_charpos: i64,
    newline_charpos: i64,
) -> Option<Vec<(usize, usize)>> {
    let text_props = crate::neovm_bridge::RustTextPropAccess::new(buffer);
    let mut elided = Vec::new();
    let mut pos = row_charpos;
    // Probe through the newline INCLUSIVE: a hidden run starting at the
    // newline itself covers the line end just as one running into it does.
    while pos <= newline_charpos {
        let (status, next_visible) = text_props.check_invisible(pos);
        if status.hidden {
            if status.ellipsis
                || pos == row_charpos
                || pos >= newline_charpos
                || next_visible > newline_charpos
                || next_visible <= pos
            {
                return None;
            }
            elided.push((
                (pos - row_charpos) as usize,
                (next_visible - row_charpos) as usize,
            ));
        }
        if next_visible <= pos {
            break;
        }
        pos = next_visible;
    }
    Some(elided)
}

/// Classify the row starting at `row` for acquisition routing. Returns
/// [`RowAcquisitionRoute::ItemRenderer`] only for the plain row class
/// described in the module docs; everything else stays on the buffer
/// pipeline. Cheap checks run first; the property/overlay probes only run for
/// content that already passed the ASCII scan.
pub(crate) fn classify_row_acquisition<B: LayoutBufferView>(
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
pub(crate) fn plan_ascii_row<B: LayoutBufferView>(
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
    // One pass scans the chars (refusing anything the pipeline would
    // compose) AND applies the strict logical-cell fit: a line exactly
    // filling the row keeps the buffer pipeline (continuation/truncation
    // policy owns that edge).
    let scan = routed_line_scan(row.text, row.byte_idx, fit)?;

    // Cursor capture stays on the buffer pipeline: exclude any row whose
    // ROUTED coverage contains point — through the line's newline for a
    // whole-line plan, through the handoff position for an overflow-prefix
    // plan (point in the unrouted remainder is fine: the pipeline resumes
    // there and captures it exactly as with the flag off).
    let routed_end_charpos = row.charpos + scan.char_len as i64;
    if policy.point_charpos >= row.charpos && policy.point_charpos <= routed_end_charpos {
        return None;
    }

    // Overlays intersecting the row (touching endpoints included) may carry
    // ONLY face-affecting properties; their in-line boundaries become face
    // segment boundaries below. Anything else — strings, display, invisible,
    // window restriction, category indirection — keeps the buffer pipeline.
    let start_byte = row.text_start_byte + row.byte_idx;
    let routed_end_byte = start_byte + scan.byte_len;
    let overlay_scan = routed_row_overlay_scan(buffer, row.charpos, start_byte, routed_end_byte)?;

    // An active display table can remap any char (including the newline).
    if crate::neovm_bridge::buffer_has_active_display_table(buffer) {
        return None;
    }

    // Invisible text: accept only the plain-elision class (hidden spans that
    // simply drop chars from the row); ellipsis, newline-spanning folds,
    // row-start runs, and non-advancing skips refuse. Overlay-sourced
    // invisibility never reaches this scan — any intersecting overlay
    // carrying `invisible` already refused through the overlay allow-list.
    let elided = routed_row_elision_scan(buffer, row.charpos, routed_end_charpos)?;

    // An overflow-prefix plan refuses ANY elision inside its coverage: the
    // scan's fit walk advanced the pen for every char including hidden ones,
    // so its handoff cut would not be the pipeline's overflow point. (A
    // hidden run beyond the handoff is unrouted remainder — the elision scan
    // above never sees it, and the pipeline handles it at resume.)
    if scan.line_end == RoutedRowLineEnd::OverflowHandoff && !elided.is_empty() {
        return None;
    }

    // Walk the property-change positions over the routed coverage AND its
    // end position (the newline for a whole-line plan — a display/invisible
    // property on the newline would replace it; the handoff char for an
    // overflow-prefix plan — probing it too is conservative, the pipeline
    // could handle a hazard there). Hazard properties anywhere in that range
    // refuse the route; changes strictly inside the coverage become
    // face-segment boundaries. The row may be multibyte, so boundary BYTE
    // positions convert to CHAR offsets through the buffer's own mapping.
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
        // Static composition: refuse exactly when the pipeline's replacement
        // predicate would fire (an inert prop still segments below, like any
        // other property change).
        if routed_composition_prop_replaces(buffer, probe) {
            return None;
        }
        let Some(change) = buffer.layout_next_text_prop_change_after_emacs_byte_pos(probe) else {
            break;
        };
        let change = change.get();
        if change <= probe_byte || change > routed_end_byte {
            break;
        }
        if change < routed_end_byte {
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

    // Overlay starts/ends are face-change stops exactly like text-property
    // changes (GNU compute_stop_pos takes the MIN of the two); merge, sort,
    // and dedupe into one ascending boundary list.
    for char_offset in overlay_scan.boundaries {
        debug_assert!(
            char_offset > 0 && char_offset < scan.char_len,
            "an in-line overlay boundary must land strictly inside the line"
        );
        face_boundaries.push(char_offset);
    }
    face_boundaries.sort_unstable();
    face_boundaries.dedup();

    // A VISIBLE composed extender must merge into a base rendered
    // immediately before it in the SAME routed segment. If a face boundary
    // lands ON the extender, or its base is hidden (the extender sits
    // exactly at a hidden span's end), the pipeline's writer still merges it
    // across that seam — into the previous segment's glyph, keeping that
    // glyph's face — a cross-segment shape the per-segment routed render
    // does not replicate. An extender INSIDE a hidden span is fine: it is
    // simply dropped (its property-change boundaries coincide with the gap
    // and split nothing).
    for &offset in &scan.composed {
        let hidden = elided
            .iter()
            .any(|&(hidden_start, hidden_end)| offset >= hidden_start && offset < hidden_end);
        if hidden {
            continue;
        }
        if face_boundaries.binary_search(&offset).is_ok()
            || elided.iter().any(|&(_, hidden_end)| offset == hidden_end)
        {
            return None;
        }
    }

    Some(AsciiRowPlan {
        line_byte_len: scan.byte_len,
        line_char_len: scan.char_len,
        has_tab: scan.has_tab,
        has_wide: scan.has_wide,
        has_overlay: overlay_scan.has_overlay,
        face_boundaries,
        elided,
        composed: scan.composed,
        line_end: scan.line_end,
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
    let text_face =
        LayoutCharPropertyLookup::new(buffer, Value::symbol("face")).text_value_at(buffer, bytepos);
    // Overlay faces merge AFTER the text face, ascending priority, via the
    // SAME shared collector the run resolution uses
    // (`BufferTextSourceCursor::face_at` -> `overlay_faces_at`).
    let overlay_faces =
        crate::neovm_bridge::overlay_faces_at(buffer, bytepos, face_resolver.current_window_id())
            .faces;
    if text_face.is_none() && overlay_faces.is_empty() {
        // No face sources: the run resolves `Inherit` -> the active
        // (checkpoint) face id, which IS `expected_face_id`.
        return false;
    }
    // Replay the run's `resolve_face_ref` chain: each merge that changes
    // nothing keeps the current id (the window base id at the start), each
    // effective merge mints the content-addressed stable id.
    let mut current_id = default_face_id;
    let mut current = std::borrow::Cow::Borrowed(default_resolved);
    // A merge that resolves but changes nothing still pins the ref to the
    // base id (`resolve_source_face_ref` returns `FaceId(base_face_id)`); a
    // value that contributes nothing leaves the ref alone.
    let mut pinned = false;
    for value in text_face.iter().chain(overlay_faces.iter()) {
        let Some(resolved) = face_resolver.resolve_buffer_face_value_over(buffer, &current, value)
        else {
            continue;
        };
        pinned = true;
        if crate::display_source_resolver::same_resolved_face(&resolved, &current) {
            continue;
        }
        current_id = stable_face_id_for_resolved(face_ids, &resolved);
        current = std::borrow::Cow::Owned(resolved);
    }
    if !pinned {
        // Every source contributed nothing: the ref stays `Inherit` ->
        // active (checkpoint) face id.
        return false;
    }
    current_id != expected_face_id
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
            Some((line_end, face)),
        )
    }

    /// Source over the row's face segments plus the newline row break at
    /// `line_end` (the newline's OWN char position — with a trailing elision
    /// the last visible segment ends before it), the break carrying the face
    /// resolved AT the newline (a face span covering the newline rides onto
    /// the appended newline space through the line-end plan, mirroring the
    /// buffer pipeline's row-break face).
    pub(crate) fn with_row_break_segments<B: LayoutBufferView + ?Sized>(
        buffer_id: BufferId,
        buffer: &B,
        segments: &[AsciiRowItemSegment],
        line_end: CharPos0,
        row_break_face: RenderFaceRef,
    ) -> Self {
        Self::from_segments(
            buffer_id,
            buffer,
            segments,
            Some((line_end, row_break_face)),
        )
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
        row_break: Option<(CharPos0, RenderFaceRef)>,
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
                    classify_routed_row_char(ch).is_some() || routed_composable_extender(ch),
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

        if let Some((line_end, break_face)) = row_break {
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

/// Test-only engagement proof for the overlay-face extension: routed rows
/// intersected by at least one (face-only) overlay.
#[cfg(test)]
pub(crate) static ROUTED_OVERLAY_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the invisible-elision extension: routed
/// rows that elide at least one plain-invisible span.
#[cfg(test)]
pub(crate) static ROUTED_ELIDED_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the composed-cluster extension: routed
/// rows containing at least one merged zero-width extender.
#[cfg(test)]
pub(crate) static ROUTED_COMPOSED_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the phase-2f truncation extension: routed
/// overflow-prefix rows in a truncating window (the pipeline truncates at
/// the handoff).
#[cfg(test)]
pub(crate) static ROUTED_TRUNCATION_PREFIX_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the phase-2f continuation extension:
/// routed overflow-prefix rows in a wrapping window (the pipeline continues
/// the line at the handoff).
#[cfg(test)]
pub(crate) static ROUTED_WRAP_PREFIX_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn note_routed_row(plan: &AsciiRowPlan, wrap_mode: LineWrapMode) {
    #[cfg(not(test))]
    let _ = wrap_mode;
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
        if plan.has_overlay {
            ROUTED_OVERLAY_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if plan.has_elision() {
            ROUTED_ELIDED_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if plan.has_composed() {
            ROUTED_COMPOSED_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if plan.is_overflow_handoff() {
            match wrap_mode {
                LineWrapMode::Truncate => {
                    ROUTED_TRUNCATION_PREFIX_ROW_COUNT
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                LineWrapMode::Wrap => {
                    ROUTED_WRAP_PREFIX_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
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
    /// walk resumes at the end of the routed coverage — the line's newline
    /// (consumed by the buffer pipeline's own line-break lifecycle) for a
    /// whole-line plan, or the first non-fitting char (consumed by the
    /// pipeline's own overflow machinery: truncation skip or continuation
    /// transition) for an overflow-prefix plan.
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
    /// by the classifier), overlay-string splits (no intersecting overlay
    /// carries a before/after-string — the classifier's overlay allow-list
    /// admits only face-affecting properties).
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
            wrap_mode: params.wrap_mode,
        };
        let Some(plan) = plan_ascii_row(buffer, row, fit, policy) else {
            return AsciiRowRouteOutcome::NotRouted;
        };

        let start = CharPos0::new(row.charpos.max(0) as usize);
        let ranges = plan.segment_ranges(start);
        // The walk resumes at the end of the ROUTED COVERAGE — the newline
        // for a whole-line plan (not the last visible segment's end: with a
        // trailing elision they differ, the hidden span sits between them
        // and produces nothing, exactly like the pipeline's invisible skip),
        // or the first non-fitting char for an overflow-prefix plan (the
        // pipeline's own overflow machinery consumes it and everything
        // after).
        let line_end = start.add_len(CharLen::new(plan.line_char_len()));

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
            // Fit re-verification with the pipeline's OWN natural
            // measurement. Whole-line plans stay strict (any borderline row
            // — exact fill — keeps the buffer pipeline). Overflow-prefix
            // plans allow the prefix to end exactly AT the right edge: the
            // pipeline's fit rule is `x + advance <= right_edge`, and pen x
            // is monotonic over the run, so a measured end at or inside the
            // edge proves every routed char individually fits — the same
            // chars the flag-off pipeline would append before its overflow
            // decision fires at the handoff char.
            let fits = if plan.is_overflow_handoff() {
                probe_position.x_px() <= self.append_surface.right_edge()
            } else {
                probe_position.x_px() < self.append_surface.right_edge()
            };
            if !fits {
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
        note_routed_row(&plan, policy.wrap_mode);
        AsciiRowRouteOutcome::Rendered
    }
}

#[cfg(test)]
#[path = "row_route_test.rs"]
mod tests;
