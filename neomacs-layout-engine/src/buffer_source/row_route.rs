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
//!   wrap-vs-truncate unchanged. Since increment 2j the route also RESUMES
//!   mid-line on the continuation rows of a visually wrapped line
//!   ([`RowRouteEntry::ContinuationResume`]): after the pipeline's wrap
//!   transition and rewind (which clears the split-run queue and reseats
//!   the cursor at the wrap char), the remaining tail classifies exactly
//!   like a line from the resume charpos and the loop's live pen, so long
//!   wrapped plain lines route row after row through iterated overflow
//!   handoffs. Composition refuses through the
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
//!   `compute_stop_pos`. Overlay before/after-strings ROUTE since P4.6
//!   sub-step 3b (the increment 2i rung 4 refusal is retired). Its recorded
//!   reason — "overlay strings are INSERTIONS driven by the walk's own
//!   overlay machinery, and unlike the replacement session there is no
//!   single typed request the routed commit can drive without replicating
//!   that walk state" — stopped being true in two independent ways.
//!   Loading and GNU ordering moved OUT of the walk and into the producer
//!   (P4.6 sub-step 1), which surfaces them as one typed insertion element;
//!   and `render_produced_strings_at_text_row` takes exactly the loop state
//!   the routed commit already owns, `overlay_context` included. So the
//!   routed commit DELEGATES — the same call `render.rs`'s loop-level arm
//!   makes — and what stays refused is only what the routed row shape
//!   genuinely cannot express: an anchor at the row start (the loop's route
//!   attempt runs BEFORE the pipeline step that would emit it, so routing
//!   would drop the string), an anchor at the coverage end (the line end and
//!   the overflow handoff char belong to the pipeline's own lifecycle), a
//!   string outside the routable Lisp-string class, and any anchor on an
//!   overflow-prefix plan (the append session clips and breaks rows itself,
//!   so the scan's handoff cut is not the pipeline's overflow point — the
//!   same reason replacement rows never route as overflow prefixes).
//!   Plain-elision `invisible` text (phase 2d) is expressible: hidden spans
//!   simply drop chars, so the routed source emits visible-segment TextRuns
//!   whose charpos bookkeeping jumps the gap, exactly like the pipeline's
//!   invisible checkpoint `skip_chars_until` (GNU `handle_invisible_prop`
//!   advancing `IT_CHARPOS`). The inexpressible invisible sub-cases refuse:
//!   ellipsis (inserts `...` glyphs with their own face/provenance rules),
//!   runs covering the newline (line-structure change), row-start runs
//!   (consumed by the loop checkpoint before the route), overlay-sourced
//!   invisibility (2c allow-list). `display` replacements route since
//!   increment 2i for the narrow routable class — a plain property-less
//!   single-line string (rung 2) or a plain `(space :width N)` spec
//!   (rung 3) anchored strictly inside the line — by rendering through the
//!   pipeline's OWN replacement session at commit (covered-charpos glyph
//!   provenance, string base-face policy, session walk bookkeeping); every
//!   other display shape refuses through [`routed_row_replacement_scan`].
//! * [`BufferAsciiItemSource`] is the `DisplayItemSource` for such a row: it
//!   produces exactly the items `BufferTextSourceCursor` would — one plain
//!   `TextRun` per face segment (one for the whole line when no property
//!   changes in range), then the explicit-newline row break.
//!
//! Rows carrying point are deliberately excluded: cursor capture is a
//! documented buffer-pipeline responsibility (see the cursor-capture note in
//! `row_lifecycle.rs`), mirroring GNU `set_cursor_from_row` operating on
//! buffer positions only.

use crate::buffer_source::producer::frame::{
    DisplayReplacementExtentLookup, ReplacementCoveredSpan,
};
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
use crate::neovm_bridge::{
    CharPropertySource, FaceResolver, LayoutBufferView, LayoutCharPropertyLookup,
    OverlayDisplayString, ResolvedFace,
};
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
    /// The window overlay strings are collected FOR (GNU's `window` overlay-
    /// property filter), or `None` when this row renders none at all. Taken
    /// from the loop's own `BufferOverlayStringTextRowRenderContext`, so the
    /// classifier and the commit's session agree by construction about which
    /// overlays apply.
    pub(crate) overlay_string_window: Option<u64>,
}

/// Why a candidate row stayed on the buffer pipeline. Every refusal point in
/// the classifier and the render probe maps to exactly one variant; the
/// route-coverage telemetry ([`route_stats_report_line`]) histograms them so
/// real workloads can show WHICH refusal dominates (the input that ranks the
/// next migration increment).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteRefusal {
    /// Window policy: horizontal scroll active.
    PolicyHscroll,
    /// Window policy: selective display active.
    PolicySelectiveDisplay,
    /// Window policy: word wrap enabled.
    PolicyWordWrap,
    /// Window policy: trailing-whitespace highlight enabled.
    PolicyTrailingWhitespace,
    /// The walk is mid-line on a NON-continuation row (resuming after a
    /// display element or at a mid-row run boundary). Since increment 2j
    /// the continuation rows of visually wrapped lines enter through
    /// [`RowRouteEntry::ContinuationResume`] instead of refusing here, so
    /// this refusal labels only the phase-4-equivalent residue.
    MidLineStart,
    /// The line's FIRST char already crosses the right edge (no routable
    /// fitting prefix exists).
    ScanNoFitFirstChar,
    /// The line exactly fills the row (the line-end/continuation edge stays
    /// on the buffer pipeline).
    ScanExactFill,
    /// A char outside the routable ladder (control/glyphless/shaped-script/
    /// nobreak/odd-width chars, malformed UTF-8).
    ScanChar,
    /// A composing char outside the routable composite class (joiners,
    /// extenders on wide/tab/row-start tails, shaped runs).
    ScanCompose,
    /// Defensive: the source text ended with ZERO scanned chars (an empty
    /// end-of-source tail — unreachable from the visible loop, which never
    /// attempts a row at `byte_idx == text.len()`). Since phase 2h the
    /// newline-less tail line itself ROUTES ([`RoutedRowLineEnd::EndOfSource`]);
    /// the bare-newline empty line routes RowBreak-only.
    ScanEob,
    /// Point sits inside the routed coverage (cursor capture stays on the
    /// buffer pipeline).
    PointInRow,
    /// An intersecting overlay carries a property outside the face-only
    /// allow-list (or its plist/boundaries are unmappable).
    Overlay,
    /// The buffer has an active display table.
    DisplayTable,
    /// Invisible text outside the plain-elision class (ellipsis,
    /// newline-spanning, row-start, non-advancing).
    Elision,
    /// An overflow-prefix plan intersected an elided span.
    OverflowElision,
    /// A hazard text property (display/mouse-face/line-height, or a
    /// replacing composition) in range.
    HazardProp,
    /// A `display` replacement outside the routed class (increment 2i):
    /// row-start anchor, newline/tab/props in the string, empty string,
    /// covered range reaching the newline, fit overflow, or a combination
    /// with elision/overflow the plan refuses conservatively.
    Replacement,
    /// A property-change boundary failed to convert to a row char offset.
    Boundary,
    /// A visible composed extender sits on a face-segment or elision seam.
    ComposedSeam,
    /// Probe: a multi-face row segment carries a box face.
    ProbeBoxFace,
    /// Probe: the per-run face chain diverges from the checkpoint chain.
    ProbeFaceDiverges,
    /// Probe: natural measurement refused or the measured end missed the
    /// classifier's fit.
    ProbeMeasure,
}

impl RouteRefusal {
    const COUNT: usize = 22;

    const ALL: [RouteRefusal; Self::COUNT] = [
        RouteRefusal::PolicyHscroll,
        RouteRefusal::PolicySelectiveDisplay,
        RouteRefusal::PolicyWordWrap,
        RouteRefusal::PolicyTrailingWhitespace,
        RouteRefusal::MidLineStart,
        RouteRefusal::ScanNoFitFirstChar,
        RouteRefusal::ScanExactFill,
        RouteRefusal::ScanChar,
        RouteRefusal::ScanCompose,
        RouteRefusal::ScanEob,
        RouteRefusal::PointInRow,
        RouteRefusal::Overlay,
        RouteRefusal::DisplayTable,
        RouteRefusal::Elision,
        RouteRefusal::OverflowElision,
        RouteRefusal::HazardProp,
        RouteRefusal::Replacement,
        RouteRefusal::Boundary,
        RouteRefusal::ComposedSeam,
        RouteRefusal::ProbeBoxFace,
        RouteRefusal::ProbeFaceDiverges,
        RouteRefusal::ProbeMeasure,
    ];

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|reason| *reason == self)
            .expect("every refusal variant is listed in ALL")
    }

    fn label(self) -> &'static str {
        match self {
            RouteRefusal::PolicyHscroll => "policy_hscroll",
            RouteRefusal::PolicySelectiveDisplay => "policy_selective_display",
            RouteRefusal::PolicyWordWrap => "policy_word_wrap",
            RouteRefusal::PolicyTrailingWhitespace => "policy_trailing_ws",
            RouteRefusal::MidLineStart => "mid_line_start",
            RouteRefusal::ScanNoFitFirstChar => "scan_no_fit_first_char",
            RouteRefusal::ScanExactFill => "scan_exact_fill",
            RouteRefusal::ScanChar => "scan_char",
            RouteRefusal::ScanCompose => "scan_compose",
            RouteRefusal::ScanEob => "scan_eob",
            RouteRefusal::PointInRow => "point_in_row",
            RouteRefusal::Overlay => "overlay",
            RouteRefusal::DisplayTable => "display_table",
            RouteRefusal::Elision => "elision",
            RouteRefusal::OverflowElision => "overflow_elision",
            RouteRefusal::HazardProp => "hazard_prop",
            RouteRefusal::Replacement => "replacement",
            RouteRefusal::Boundary => "boundary",
            RouteRefusal::ComposedSeam => "composed_seam",
            RouteRefusal::ProbeBoxFace => "probe_box_face",
            RouteRefusal::ProbeFaceDiverges => "probe_face_diverges",
            RouteRefusal::ProbeMeasure => "probe_measure",
        }
    }
}

/// Route-coverage telemetry, mirroring the NEOMACS_LAYOUT_STATS_FILE
/// pattern: when `NEOMACS_ROW_ROUTE_STATS_FILE` names a path, the counters
/// below accumulate (relaxed atomics, only touched when the file env is set
/// — a single cached-bool branch otherwise) and `engine.rs` appends one
/// CUMULATIVE line per accepted frame. Aggregation takes the LAST line per
/// pid, so multi-process suite runs sum cleanly.
fn route_stats_file() -> Option<&'static str> {
    static FILE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    FILE.get_or_init(|| std::env::var("NEOMACS_ROW_ROUTE_STATS_FILE").ok())
        .as_deref()
        .filter(|path| !path.is_empty())
}

static ROUTE_STAT_ATTEMPTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static ROUTE_STAT_ROUTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static ROUTE_STAT_REFUSALS: [std::sync::atomic::AtomicUsize; RouteRefusal::COUNT] =
    [const { std::sync::atomic::AtomicUsize::new(0) }; RouteRefusal::COUNT];

fn note_route_attempt() {
    if route_stats_file().is_some() {
        ROUTE_STAT_ATTEMPTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn note_route_refusal(reason: RouteRefusal) {
    if route_stats_file().is_some() {
        ROUTE_STAT_REFUSALS[reason.index()].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Increment-2j sub-cause telemetry for the `mid_line_start` LOOP-RESUME
/// refusal class (its `pending_items` sibling retired with the pending queue at
/// P4.5). These are walk mechanics, not row classes: most of their hits are per-step ECHOES inside a line the pipeline
/// is already rendering. The sub-counters decompose them into what the
/// migration can actually act on:
/// * `*_rows` — DISTINCT (row, charpos) sites, i.e. how many actual candidate
///   positions the class represents versus raw attempt echoes;
/// * `*_cont` / `*_cont_rows` — hits (and distinct sites) on a row flagged
///   `Continuation`, the wrapped-line resume class whose entry state is the
///   Phase-2j routing question;
static ROUTE_STAT_MIDLINE_CONT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
static ROUTE_STAT_MIDLINE_ROWS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
static ROUTE_STAT_MIDLINE_CONT_ROWS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
static ROUTE_STAT_LAST_MIDLINE_SITE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(u64::MAX);
static ROUTE_STAT_LAST_MIDLINE_CONT_SITE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(u64::MAX);

/// A distinct-site key: the attempt stream is sequential per layout pass, so
/// counting transitions of (row, charpos) approximates distinct candidate
/// sites well enough for a coverage-ranking histogram.
fn route_stats_site_key(row_index: usize, charpos: i64) -> u64 {
    ((row_index as u64) << 40) ^ (charpos as u64 & 0xFF_FFFF_FFFF)
}

fn note_distinct_site(
    last: &std::sync::atomic::AtomicU64,
    counter: &std::sync::atomic::AtomicUsize,
    key: u64,
) {
    if last.swap(key, std::sync::atomic::Ordering::Relaxed) != key {
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn note_route_refusal_midline(continuation_row: bool, site_key: u64) {
    if route_stats_file().is_none() {
        return;
    }
    note_route_refusal(RouteRefusal::MidLineStart);
    if continuation_row {
        ROUTE_STAT_MIDLINE_CONT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        note_distinct_site(
            &ROUTE_STAT_LAST_MIDLINE_CONT_SITE,
            &ROUTE_STAT_MIDLINE_CONT_ROWS,
            site_key,
        );
    }
    note_distinct_site(
        &ROUTE_STAT_LAST_MIDLINE_SITE,
        &ROUTE_STAT_MIDLINE_ROWS,
        site_key,
    );
}

/// The cumulative telemetry line for this process, or `None` when the stats
/// file env is unset. Appended by the engine once per accepted frame.
pub(crate) fn route_stats_append_report() {
    use std::io::Write as _;
    let Some(path) = route_stats_file() else {
        return;
    };
    let mut line = format!(
        "row_route pid={} attempts={} routed={}",
        std::process::id(),
        ROUTE_STAT_ATTEMPTS.load(std::sync::atomic::Ordering::Relaxed),
        ROUTE_STAT_ROUTED.load(std::sync::atomic::Ordering::Relaxed),
    );
    for reason in RouteRefusal::ALL {
        let count = ROUTE_STAT_REFUSALS[reason.index()].load(std::sync::atomic::Ordering::Relaxed);
        line.push_str(&format!(" refuse_{}={}", reason.label(), count));
    }
    for (label, counter) in [
        ("routed_resume", &ROUTE_STAT_ROUTED_RESUME),
        ("midline_cont", &ROUTE_STAT_MIDLINE_CONT),
        ("midline_rows", &ROUTE_STAT_MIDLINE_ROWS),
        ("midline_cont_rows", &ROUTE_STAT_MIDLINE_CONT_ROWS),
    ] {
        line.push_str(&format!(
            " {label}={}",
            counter.load(std::sync::atomic::Ordering::Relaxed)
        ));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{line}");
    }
}

/// How the walk arrived at the candidate position (increment 2j). The
/// classifier trusts this attestation — only the dispatch site can prove it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowRouteEntry {
    /// The walk sits at a true buffer line start (byte 0 or just past a
    /// newline). The only entry the route accepted through increment 2i.
    LineStart,
    /// The walk resumes MID-LINE on a row the visual-wrap transition flagged
    /// `Continuation` (`DisplayRowFlagKind::Continuation`, stamped by
    /// `DisplayRowOverflowTransitionPlan::emit_with_output` before anything
    /// is appended to the new row), with NO pending render items — the
    /// character-wrap rewind (`rewind_source_consumption_to`, GNU
    /// `RESTORE_IT`) cleared the split-run queue and reseated the cursor at
    /// the wrap candidate, so the remaining tail of the continued line is
    /// plain buffer text again. This is the continuation-row resume class:
    /// GNU `display_line` simply keeps calling `get_next_display_element`
    /// across the row boundary with `it->continuation_lines_width` carried
    /// in the iterator; here the carried state is exactly the loop's live
    /// pen (x, col) and charpos/byte position, which the fit walk and the
    /// commit already take as inputs. Everything position-relative in the
    /// classifier (face boundaries, elision spans, replacement anchors,
    /// overlay boundaries) is computed from the resume charpos, so the plan
    /// needs no other carry-over.
    ContinuationResume,
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
    replacements: Vec<RoutedRowReplacement>,
    /// The overlay-string anchors inside the routed coverage, ascending. Each
    /// is an INSERTION: it consumes no chars, so it is not a gap in
    /// [`AsciiRowPlan::segment_ranges`] the way a replacement is.
    overlay_strings: Vec<RoutedRowOverlayStrings>,
    line_end: RoutedRowLineEnd,
}

/// A routed `display` replacement (increment 2i rung 2): the covered CHAR
/// range `[start, end)` of the line renders as the display value through the
/// pipeline's OWN replacement session (`display_property_render.rs` ->
/// `replacement.rs`), producing covered-provenance glyphs (every glyph
/// stamped with the covered start — the rung 1 vocabulary pins). The routed
/// class accepts only single-line property-less strings whose chars have
/// unambiguous column widths, so `advance_cols` is the exact logical-cell
/// advance the classifier's fit walk credits in place of the covered chars.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoutedRowReplacement {
    /// CHAR offset of the covered range start within the line.
    start: usize,
    /// The covered range in absolute positions, as the producer's rule
    /// derived it. The plan carries the span itself rather than a second end
    /// offset so the commit hands the session the range the SCAN resolved,
    /// with no chance of a third party re-deriving E.
    covered: ReplacementCoveredSpan,
    /// The full `display` property value (what the pipeline's walk consumes).
    value: Value,
    /// What the routed class recognized inside the display value.
    content: RoutedReplacementContent,
    /// The replacement's logical-cell width for the classifier's fit walk.
    advance_cols: usize,
}

impl RoutedRowReplacement {
    /// CHAR offset of the covered range end (exclusive) within the line.
    fn end(&self) -> usize {
        self.start + self.covered.covered_char_len()
    }
}

/// The routable display-replacement content kinds (increment 2i).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RoutedReplacementContent {
    /// Rung 2: a plain, property-less, single-line string.
    String { text: Box<str> },
    /// Rung 3: a plain `(space :width N)` spec, N a positive fixnum — one
    /// stretch glyph of N columns with covered-charpos provenance (GNU
    /// stamps the covered buffer position on stretch glyphs; xdisp.c
    /// handle_single_display_spec 6604 + append_stretch_glyph 32684).
    SpaceWidth,
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
    /// Phase 2h rung 2: the line ends AT the end of the source text with no
    /// newline. The window read bound always cuts AFTER a complete line's
    /// newline (`find_nth_newline_after` returns newline+1) or at the
    /// accessible end, so a newline-less tail line is never a mid-line
    /// artifact of the bound — it is the buffer's (or narrowed region's)
    /// last line, GNU's `IT_EOB` exit (xdisp.c:26007, `row->ends_at_zv_p`).
    /// The plan covers the whole tail; the routed render leaves the walk at
    /// the source end and the visible loop exits, after which the pipeline's
    /// post-loop end-of-buffer machinery (EOB cursor/tail request, appended
    /// space, `ends_at_zv` marking in `finish_pending_text_window_row`, the
    /// trailing ZV placeholder row) runs unchanged on both modes. GNU has NO
    /// analogue of a bounded read (its iterator is lazy and stops only on
    /// pixels or ZV), so the faithful semantics here are the pipeline's own:
    /// route only WHO renders the tail's text, never the row's EOB
    /// finalization.
    EndOfSource,
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

    /// Phase 2h rung 1: a bare-newline empty line — zero covered chars, the
    /// production is RowBreak-only (the shared line-end plan consumes the
    /// newline).
    pub(crate) fn is_empty_line(&self) -> bool {
        self.line_char_len == 0
    }

    /// Phase 2h rung 2: the newline-less tail line ending at the source end.
    pub(crate) fn is_end_of_source(&self) -> bool {
        self.line_end == RoutedRowLineEnd::EndOfSource
    }

    /// Whether the row contains a routed `display` replacement.
    pub(crate) fn has_replacement(&self) -> bool {
        !self.replacements.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn replacement_ranges(&self) -> Vec<(usize, usize)> {
        self.replacements
            .iter()
            .map(|replacement| (replacement.start, replacement.end()))
            .collect()
    }

    pub(crate) fn replacements(&self) -> &[RoutedRowReplacement] {
        &self.replacements
    }

    /// Whether the row carries an overlay-string anchor.
    pub(crate) fn has_overlay_strings(&self) -> bool {
        !self.overlay_strings.is_empty()
    }

    pub(crate) fn overlay_strings(&self) -> &[RoutedRowOverlayStrings] {
        &self.overlay_strings
    }

    /// Whether the row renders as more than one run (face segments, elision
    /// gaps, or replacement spans splitting the line).
    pub(crate) fn is_segmented(&self) -> bool {
        !self.face_boundaries.is_empty() || !self.elided.is_empty() || !self.replacements.is_empty()
    }

    /// The `[start, end)` char ranges of the row's VISIBLE face segments, in
    /// row order: the line minus the elided spans, split at each face
    /// boundary that falls strictly inside a visible stretch (boundaries at
    /// an elided edge coincide with the gap and split nothing; boundaries
    /// inside a hidden span never render). A property-constant fully-visible
    /// line yields one range covering the line.
    pub(crate) fn segment_ranges(&self, start: CharPos0) -> Vec<(CharPos0, CharPos0)> {
        // Gaps the text segments skip: elided spans and replacement-covered
        // spans (mutually exclusive by the classifier's composition refusal;
        // each list is ascending and disjoint, so a simple merge sorts them).
        let mut gaps: Vec<(usize, usize)> =
            Vec::with_capacity(self.elided.len() + self.replacements.len());
        gaps.extend(self.elided.iter().copied());
        gaps.extend(
            self.replacements
                .iter()
                .map(|replacement| (replacement.start, replacement.end())),
        );
        gaps.sort_unstable();
        let mut visible: Vec<(usize, usize)> = Vec::with_capacity(gaps.len() + 1);
        let mut cursor = 0usize;
        for &(hidden_start, hidden_end) in &gaps {
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
fn routed_line_scan(
    text: &[u8],
    byte_idx: usize,
    fit: RowRouteFit<'_>,
    replacements: &[RoutedRowReplacement],
    overlay_strings: &[RoutedRowOverlayStrings],
) -> Result<RoutedLineScan, RouteRefusal> {
    let mut idx = byte_idx;
    let mut char_len = 0usize;
    let mut has_tab = false;
    let mut has_wide = false;
    let mut composed = Vec::new();
    let mut x_px = fit.start_x_px;
    let mut col = fit.start_col;
    let mut tail: Option<(char, bool)> = None;
    let mut merge_target = RoutedScanMergeTarget::None;
    let mut next_replacement = replacements.iter().peekable();
    let mut next_anchor = overlay_strings.iter().peekable();
    // The maximal-fitting-prefix cut for an over-wide line: every scanned
    // char so far fits; the char at `idx` would cross the right edge, so the
    // routed coverage ends here and the pipeline resumes at `idx`.
    let overflow_prefix =
        |idx: usize, char_len: usize, has_tab: bool, has_wide: bool, composed: Vec<usize>| {
            if char_len == 0 {
                return Err(RouteRefusal::ScanNoFitFirstChar);
            }
            Ok(RoutedLineScan {
                byte_len: idx - byte_idx,
                char_len,
                has_tab,
                has_wide,
                composed,
                line_end: RoutedRowLineEnd::OverflowHandoff,
            })
        };
    while idx < text.len() {
        // A replacement-covered span (increment 2i rung 2): the pen advances
        // by the REPLACEMENT string's predicted columns, not the covered
        // chars', and the covered chars are consumed without classification
        // (they are never rendered — any well-formed UTF-8 content is fine,
        // exactly like the pipeline's skip_chars_until over the covered
        // range). The session renders into the row, so a following extender
        // finds no routable merge target (tail = None) and refuses through
        // the ordinary ladder. A replacement whose advance crosses the right
        // edge refuses outright: replacement rows never route as overflow
        // prefixes (the handoff cut would not be the pipeline's overflow
        // point mid-replacement).
        if let Some(replacement) = next_replacement.peek()
            && char_len == replacement.start
        {
            let advance_px = replacement.advance_cols as f32 * fit.char_width_px;
            if x_px + advance_px > fit.right_edge_px {
                return Err(RouteRefusal::Replacement);
            }
            x_px += advance_px;
            col += replacement.advance_cols;
            tail = None;
            merge_target = RoutedScanMergeTarget::None;
            while char_len < replacement.end() {
                if idx >= text.len() || text[idx] == b'\n' {
                    return Err(RouteRefusal::Replacement);
                }
                let (ch, consumed) = decode_utf8(&text[idx..]);
                if consumed == 0 || ch.len_utf8() != consumed {
                    return Err(RouteRefusal::ScanChar);
                }
                char_len += 1;
                idx += consumed;
            }
            next_replacement.next();
            continue;
        }
        if text[idx] == b'\n' {
            // A bare-newline empty line routes with ZERO covered chars
            // (phase 2h rung 1): the production is RowBreak-only, driving
            // the shared line-end plan. A non-empty line exactly filling the
            // row keeps the pipeline (the line end interacts with
            // continuation policy); an empty line's pen never moved.
            if char_len > 0 && x_px >= fit.right_edge_px {
                return Err(RouteRefusal::ScanExactFill);
            }
            return Ok(RoutedLineScan {
                byte_len: idx - byte_idx,
                char_len,
                has_tab,
                has_wide,
                composed,
                line_end: RoutedRowLineEnd::Newline,
            });
        }
        // An overlay-string ANCHOR (P4.6): the producer surfaces its strings
        // BEFORE the buffer char at this position and does not advance (GNU
        // `push_it (it, NULL)` insertion semantics), so the pen gains the
        // strings' columns and no char is consumed. Strings that would cross
        // the right edge refuse outright rather than cutting a prefix here:
        // the append session clips and breaks rows on its own, so a cut
        // taken mid-anchor is not the pipeline's overflow point. The strings
        // append real glyphs, so a following extender finds no routable
        // merge target.
        if let Some(anchor) = next_anchor.peek()
            && char_len == anchor.at
        {
            let advance_px = anchor.advance_cols as f32 * fit.char_width_px;
            if x_px + advance_px > fit.right_edge_px {
                return Err(RouteRefusal::Overlay);
            }
            x_px += advance_px;
            col += anchor.advance_cols;
            tail = None;
            merge_target = RoutedScanMergeTarget::None;
            next_anchor.next();
            continue;
        }
        let (ch, consumed) = decode_utf8(&text[idx..]);
        // Reject malformed UTF-8 (decode yields U+FFFD over fewer bytes than
        // the char re-encodes to): raw bytes have their own display path.
        if consumed == 0 || ch.len_utf8() != consumed {
            return Err(RouteRefusal::ScanChar);
        }
        // Pipeline decision order: non-Text chars break the text run into
        // their own items BEFORE any composition (the classify arm below
        // refuses those), while a Text-class char consults the writer's
        // compose ladder first.
        if classify_text_source_char(ch) == TextSourceCharClassification::Text
            && routed_char_would_compose(ch, tail)
        {
            if !(routed_composable_extender(ch) && merge_target == RoutedScanMergeTarget::Simple) {
                return Err(RouteRefusal::ScanCompose);
            }
            // The merge appends no glyph and advances nothing; the cluster's
            // tail becomes the extender (writer: the Composite's last char).
            composed.push(char_len);
            tail = Some((ch, false));
            char_len += 1;
            idx += consumed;
            continue;
        }
        match classify_routed_row_char(ch).ok_or(RouteRefusal::ScanChar)? {
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
    // End of the source text without a newline (phase 2h rung 2): the tail
    // line ends at the accessible end — the read bound never cuts mid-line —
    // and routes as [`RoutedRowLineEnd::EndOfSource`]. Its end-of-buffer
    // finalization (appended default-face space, ends_at_zv, the ZV
    // placeholder) is post-loop pipeline machinery on both modes. The pen
    // may end exactly AT the right edge: with no following char there is no
    // continuation/truncation edge to interact with (every scanned char
    // individually satisfied the fit rule).
    if char_len == 0 {
        return Err(RouteRefusal::ScanEob);
    }
    Ok(RoutedLineScan {
        byte_len: idx - byte_idx,
        char_len,
        has_tab,
        has_wide,
        composed,
        line_end: RoutedRowLineEnd::EndOfSource,
    })
}

/// Overlay properties the routed row class accepts on an intersecting
/// overlay. `face` merges through the SAME resolver seam the pipeline's
/// checkpoint uses (GNU `face_at_buffer_position`'s ascending-priority
/// overlay loop), `priority` orders that merge, and `evaporate` is
/// buffer-maintenance-only. `before-string`/`after-string` joined the list at
/// P4.6 sub-step 3b: the producer owns their collection and GNU ordering, and
/// the routed commit delegates the append to the pipeline's OWN session
/// through [`RoutedRowOverlayStrings`] — the routable-shape and anchor-
/// position conditions are enforced by [`routed_row_overlay_string_scan`],
/// not by keeping the properties off this list. EVERYTHING else refuses the
/// route: `display`/`invisible` rewrite content, `mouse-face`/`line-prefix`/
/// `line-height` and friends have pipeline machinery, `window` restricts
/// applicability per window, and `category` indirects to arbitrary props.
/// Unknown properties are conservatively refused (allow-list, not deny-list).
const ROUTE_SAFE_OVERLAY_PROPS: [&str; 5] = [
    "face",
    "priority",
    "evaporate",
    "before-string",
    "after-string",
];

/// The overlay properties that anchor a Lisp-string INSERTION at an overlay
/// endpoint (before-string at its start, after-string at its end).
const OVERLAY_STRING_PROPS: [&str; 2] = ["before-string", "after-string"];

/// The overlay facts of a candidate row: whether any overlay intersects it
/// and the overlay start/end CHAR boundaries strictly inside the line.
struct RoutedRowOverlayScan {
    has_overlay: bool,
    boundaries: Vec<usize>,
}

/// One overlay-string ANCHOR inside a routed row: the CHAR offset where the
/// producer surfaces its typed insertion element, and the strings collected
/// there in GNU `compare_overlay_entries` order.
///
/// An insertion consumes no buffer characters (GNU `push_it (it, NULL)`
/// resumes at the same position), so `at` is both the start and the end of
/// the part this becomes — which is exactly why the parts of a routed row
/// cannot be ordered by position alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoutedRowOverlayStrings {
    /// CHAR offset of the anchor within the line.
    at: usize,
    /// The producer's collection at `at`, already GNU-ordered. The routed
    /// commit hands this slice to the SAME session `render.rs` drives, so
    /// content and order are the producer's, not a second derivation.
    strings: Vec<OverlayDisplayString>,
    /// The strings' combined logical-cell advance for the classifier's fit
    /// walk. The probe re-verifies it with the session's own base face.
    advance_cols: usize,
}

/// What [`routed_row_overlay_string_scan`] found on the line: the routable
/// anchors, and the offsets of the anchors whose strings are not routable.
/// Both are reported rather than decided, because whether either matters
/// depends on where the routed coverage ends.
#[derive(Default)]
struct RoutedRowOverlayStringScan {
    anchors: Vec<RoutedRowOverlayStrings>,
    /// CHAR offsets of anchors outside the routable string class. The caller
    /// refuses only for the ones the coverage reaches, mirroring how
    /// [`RoutedRowDisplayScan`] treats an unroutable `display` value.
    hazards: Vec<usize>,
}

impl RoutedRowOverlayStrings {
    pub(crate) fn at(&self) -> usize {
        self.at
    }

    pub(crate) fn strings(&self) -> &[OverlayDisplayString] {
        &self.strings
    }

    pub(crate) fn advance_cols(&self) -> usize {
        self.advance_cols
    }
}

/// Scan the overlays intersecting `[start_byte, coverage_end_byte]` (touching
/// endpoints included: an overlay ending at the row start or starting at the
/// coverage end can anchor strings there). Returns `None` — refusing the
/// route — when any intersecting overlay carries a property outside
/// [`ROUTE_SAFE_OVERLAY_PROPS`]. Boundary positions mirror GNU
/// `next_overlay_change` feeding `compute_stop_pos`: every overlay start or
/// end strictly inside the coverage becomes a face-segment boundary (an empty
/// overlay contributes its single position).
///
/// `coverage_end_byte` is the ROUTED coverage end — the newline for a
/// whole-line plan, the handoff char for an overflow-prefix plan — so an
/// overlay living entirely in an over-wide line's unreached tail neither
/// refuses the route nor segments the row.
fn routed_row_overlay_scan<B: LayoutBufferView + ?Sized>(
    buffer: &B,
    row_charpos: i64,
    start_byte: usize,
    coverage_end_byte: usize,
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
        if ov_start > coverage_end_byte || ov_end < start_byte {
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
            if boundary > start_byte && boundary < coverage_end_byte {
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

/// The routable Lisp-string class, shared by `display` replacements and
/// overlay strings: a plain, property-less string whose every char has an
/// unambiguous column width. Returns its total advance in columns.
///
/// A newline would end the row, a tab expands pen-dependently in the
/// session's own frame, and text properties re-face (or re-shape) the string
/// mid-flight — none of which the classifier's logical-cell fit walk can
/// predict, so all three refuse. This is ONE predicate deliberately: the two
/// callers must agree about what "plain enough to route" means, and a second
/// copy would drift.
fn routed_lisp_string_advance_cols(value: Value) -> Option<usize> {
    let text = value.as_utf8_str()?;
    if text.is_empty() {
        return None;
    }
    if neovm_core::emacs_core::value::get_string_text_properties_table_for_value(value).is_some() {
        return None;
    }
    let mut advance_cols = 0usize;
    for ch in text.chars() {
        match classify_routed_row_char(ch) {
            Some(RoutedRowCharAdvance::Cols(cols)) => advance_cols += usize::from(cols),
            Some(RoutedRowCharAdvance::Tab) | None => return None,
        }
    }
    Some(advance_cols)
}

/// Find the overlay-string anchors on the line `[start_byte, line_end_byte]`
/// and collect the strings at each, refusing the route for anything the
/// routed row shape cannot express.
///
/// Anchor positions are the endpoints of the intersecting overlays that carry
/// a string property (before-string at the start, after-string at the end);
/// the CONTENT at each position comes from `overlay_strings_at`, the same
/// producer-side collection the pipeline renders, so the two can never
/// disagree about which strings are there, in which order, or whether a
/// position is an anchor at all (GNU drops empty strings at collection —
/// `STRINGP && SCHARS`, xdisp.c:7171-7182 — so an overlay whose only string
/// is `""` anchors nothing and must not refuse).
///
/// Refuses outright only for an anchor ON the row start: the visible loop
/// attempts the route BEFORE the pipeline step that would emit the strings
/// (loop_render.rs), so a routed row start would DROP them. That is a
/// correctness bound, and it needs no coverage knowledge — offset 0 is inside
/// every coverage.
///
/// Everything else this scan finds is REPORTED, not decided, because both
/// remaining questions turn on where the routed coverage ends and the fit
/// walk has not run yet. An anchor whose strings are outside the routable
/// class is recorded as a HAZARD at its offset, exactly as
/// [`routed_row_replacement_scan`] records an unroutable `display` value, and
/// the caller refuses only if the coverage reaches it. Deciding either here
/// instead cost real routing: refusing a line-end anchor lost every prefix
/// row of a long wrapped line, and refusing an unroutable string lost the
/// prefix rows of a line whose unroutable string sits in the unreached tail.
///
/// Anchors before `start_byte` are simply not this row's: they were emitted
/// on an earlier row.
fn routed_row_overlay_string_scan<B: LayoutBufferView>(
    buffer: &B,
    window_id: Option<u64>,
    row_charpos: i64,
    start_byte: usize,
    line_end_byte: usize,
) -> Result<RoutedRowOverlayStringScan, RouteRefusal> {
    let mut scan = RoutedRowOverlayStringScan::default();
    // No window means this row renders no overlay strings at all, so no
    // position on it is an anchor (the append is a no-op either way).
    let Some(window_id) = window_id else {
        return Ok(scan);
    };
    let overlays = buffer.layout_overlays();
    if overlays.is_empty() {
        return Ok(scan);
    }

    // Candidate positions first, content second: reading the plists of the
    // overlays that intersect the line is cheap, and it keeps the per-anchor
    // collection (which walks a byte range and sorts) off every row that has
    // face-only overlays.
    let mut anchor_bytes: Vec<usize> = Vec::new();
    for overlay in overlays.overlays_in_gnu_lists_order() {
        let (Some(ov_start), Some(ov_end)) = (
            overlays.overlay_start_emacs_byte_pos(overlay),
            overlays.overlay_end_emacs_byte_pos(overlay),
        ) else {
            continue;
        };
        let (ov_start, ov_end) = (ov_start.get(), ov_end.get());
        if ov_start > line_end_byte || ov_end < start_byte {
            continue;
        }
        if OVERLAY_STRING_PROPS.iter().any(|prop| {
            overlays
                .overlay_get_named(overlay, Value::symbol(prop))
                .is_some()
        }) {
            anchor_bytes.push(ov_start);
            anchor_bytes.push(ov_end);
        }
    }
    anchor_bytes.sort_unstable();
    anchor_bytes.dedup();

    let props = crate::neovm_bridge::RustTextPropAccess::new_for_window(buffer, window_id);
    for anchor_byte in anchor_bytes {
        if anchor_byte < start_byte || anchor_byte > line_end_byte {
            continue;
        }
        let anchor_charpos = buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(anchor_byte))
            .get();
        let strings = props.overlay_strings_at(anchor_charpos as i64);
        if strings.is_empty() {
            continue;
        }
        if anchor_byte == start_byte {
            return Err(RouteRefusal::Overlay);
        }
        let at = anchor_charpos
            .checked_sub(row_charpos.max(0) as usize)
            .ok_or(RouteRefusal::Boundary)?;
        let mut advance_cols = 0usize;
        let routable =
            strings.iter().all(
                |string| match routed_lisp_string_advance_cols(string.string) {
                    Some(cols) => {
                        advance_cols += cols;
                        true
                    }
                    None => false,
                },
            );
        if !routable {
            scan.hazards.push(at);
            continue;
        }
        scan.anchors.push(RoutedRowOverlayStrings {
            at,
            strings,
            advance_cols,
        });
    }
    Ok(scan)
}

/// Scan the line `[start_byte, line_end_byte]` for `display` text properties
/// (increment 2i rung 2), walking property-change positions exactly like the
/// hazard walk. Every `display` value found must be a routable replacement
/// (a plain, property-less, single-line string of unambiguous-width chars,
/// anchored strictly inside the line and covering chars strictly before the
/// newline) or the row refuses. The covered extent is not mirrored from the
/// pipeline — it IS the pipeline's, taken from
/// [`ReplacementCoveredSpan::for_property_source`] through
/// [`RoutedScanExtentLookup`] (GNU `display_prop_end` /
/// `next_single_char_property_change(pos, Qdisplay)`: the range over which the
/// resolved value stays the SAME object). `line_end_byte` is the newline's
/// byte (or the source end for the tail line); a value covering it would
/// replace the line end and refuses.
///
/// Overlay-supplied `display` never reaches this scan: the overlay allow-list
/// refuses any intersecting overlay carrying `display`, so the text-property
/// read here resolves the same winner as the pipeline's
/// `get_char_property`-style overlay-or-text read.
/// Parse the rung-3 routable space shape: exactly `(space :width N)` with a
/// positive fixnum N, nothing else. Every other `(space …)` form —
/// `:align-to` (targets a column), `:relative-width` (consults the covered
/// char's font), extra vertical keys, float widths, expression operands
/// riding `calc_pixel_width_or_height` — keeps the buffer pipeline: their
/// widths are pen/metric-dependent in ways the classifier's logical-cell
/// pre-filter cannot predict. For the plain form GNU's width is N times the
/// canonical column width (xdisp.c calc_pixel_width_or_height, bare numbers
/// scale by FRAME_COLUMN_WIDTH on the horizontal axis), which is exactly N
/// advance columns for the fit walk; the probe re-verifies with the
/// session's own resolved stretch width.
fn routed_space_width_cols(spec: Value) -> Option<usize> {
    use crate::display_spec::DisplaySpaceKey;
    if !crate::display_spec::is_display_space_spec(&spec) {
        return None;
    }
    let rest = spec.cons_cdr();
    if !rest.is_cons() {
        return None;
    }
    if DisplaySpaceKey::from_lisp_value(rest.cons_car()) != Some(DisplaySpaceKey::Width) {
        return None;
    }
    let tail = rest.cons_cdr();
    if !tail.is_cons() || !tail.cons_cdr().is_nil() {
        return None;
    }
    let cols = tail.cons_car().as_fixnum()?;
    (1..=512).contains(&cols).then_some(cols as usize)
}

/// Outcome of the display-property line scan: the routable replacement
/// candidates in ascending order, plus the CHAR offsets (with refusal
/// reasons) of unroutable `display` props. The classifier refuses only when
/// an unroutable prop falls inside (or at the end of) the ROUTED coverage —
/// a prop in the unreached tail of an overflow-prefix plan stays with the
/// pipeline at resume, preserving the phase-2f class exactly.
struct RoutedRowDisplayScan {
    replacements: Vec<RoutedRowReplacement>,
    hazards: Vec<(usize, RouteRefusal)>,
}

/// The four lookups [`ReplacementCoveredSpan::for_property_source`] needs, as
/// the routed line scan can answer them: over the buffer view's TEXT
/// properties, bounded by the line end.
///
/// The scan reads only text properties, and that is sound rather than lucky:
/// `display` is not in [`ROUTE_SAFE_OVERLAY_PROPS`], so an overlay carrying one
/// refuses the route outright, well before this scan runs. The source reaching
/// the constructor from here is therefore always `overlay: None`.
struct RoutedScanExtentLookup<'a, B: ?Sized> {
    buffer: &'a B,
    /// The line end (the newline's position, or the source end for the tail
    /// line). Clipping the extent here rather than letting it run past is what
    /// lets the caller ask its one routability question — does the covered
    /// range reach the line end? — as a comparison instead of a second walk.
    line_end: CharPos0,
}

impl<B: LayoutBufferView + ?Sized> DisplayReplacementExtentLookup
    for RoutedScanExtentLookup<'_, B>
{
    fn extent_scan_end(&self) -> CharPos0 {
        self.line_end
    }

    fn extent_overlay_end(&self, _overlay: Value) -> Option<CharPos0> {
        // Unreachable by construction (see the struct doc): an overlay-sourced
        // `display` refuses the route before the scan runs. `None` is the
        // conservative answer if that ever stops holding — the constructor
        // falls back to the property run's own end, which cannot over-cover.
        debug_assert!(false, "the routed scan never resolves an overlay source");
        None
    }

    fn extent_display_prop_at(&self, at: CharPos0) -> Option<Value> {
        self.buffer.layout_text_prop_at_emacs_byte_pos(
            self.buffer.layout_char_pos_to_emacs_byte_pos(at),
            Value::symbol("display"),
        )
    }

    fn extent_next_property_change(&self, at: CharPos0) -> CharPos0 {
        let byte = self.buffer.layout_char_pos_to_emacs_byte_pos(at);
        self.buffer
            .layout_next_text_prop_change_after_emacs_byte_pos(byte)
            .map(|change| change.get())
            .filter(|&change| change > byte.get())
            .map(|change| {
                self.buffer
                    .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(change))
            })
            // No further change means the properties hold to the end of the
            // buffer, so the covered range reaches the line end and past it.
            // Reporting the bound says exactly that, and the caller's line-end
            // test then refuses — which is what the inline walk did directly.
            .unwrap_or(self.line_end)
    }
}

fn routed_row_replacement_scan<B: LayoutBufferView>(
    buffer: &B,
    row_charpos: i64,
    start_byte: usize,
    line_end_byte: usize,
) -> Result<RoutedRowDisplayScan, RouteRefusal> {
    use crate::display_property::{DisplayReplacementProperty, classify_display_property};

    let display_prop_at = |byte: usize| {
        buffer.layout_text_prop_at_emacs_byte_pos(EmacsBytePos::new(byte), Value::symbol("display"))
    };
    let char_offset_at = |byte: usize| -> Result<usize, RouteRefusal> {
        buffer
            .layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(byte))
            .get()
            .checked_sub(row_charpos.max(0) as usize)
            .ok_or(RouteRefusal::Boundary)
    };
    let next_change_after = |byte: usize| {
        buffer
            .layout_next_text_prop_change_after_emacs_byte_pos(EmacsBytePos::new(byte))
            .map(|change| change.get())
            .filter(|&change| change > byte)
    };

    let line_end = buffer.layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(line_end_byte));

    let mut scan = RoutedRowDisplayScan {
        replacements: Vec::new(),
        hazards: Vec::new(),
    };
    let mut probe_byte = start_byte;
    loop {
        if let Some(value) = display_prop_at(probe_byte) {
            let probe_offset = char_offset_at(probe_byte)?;
            // The routable class: a plain, property-less, single-line string
            // of unambiguous-width chars, anchored strictly inside the line
            // (a row-start anchor replays into the loop's segment-0
            // checkpoint; a line-end anchor replaces the newline), covering
            // chars strictly before the line end. Everything else records a
            // hazard at its position: non-string display shapes keep the
            // historical HazardProp refusal, unroutable string shapes the
            // Replacement refusal.
            let classification = classify_display_property(value);
            let spec = classification.replacement_spec();
            // Parse the routable content shape, independent of anchoring:
            // rung 2 — a plain, property-less string whose chars are all
            // single-line unambiguous-width (a newline emits a row break, a
            // tab expands pen-dependently in the session's full-text-width
            // frame); rung 3 — a plain `(space :width N)` spec.
            let content: Option<(RoutedReplacementContent, usize)> = match classification
                .replacement()
            {
                Some(DisplayReplacementProperty::String) => routed_lisp_string_advance_cols(spec)
                    .and_then(|advance_cols| {
                        spec.as_utf8_str().map(|text| {
                            (
                                RoutedReplacementContent::String { text: text.into() },
                                advance_cols,
                            )
                        })
                    }),
                Some(DisplayReplacementProperty::Stretch(_)) => routed_space_width_cols(spec)
                    .map(|cols| (RoutedReplacementContent::SpaceWidth, cols)),
                _ => None,
            };
            // Hazard reasons keep their historic split: string display
            // values (and recognized space shapes) report Replacement;
            // everything else keeps HazardProp.
            let hazard_reason = if content.is_some()
                || matches!(
                    classification.replacement(),
                    Some(DisplayReplacementProperty::String)
                ) {
                RouteRefusal::Replacement
            } else {
                RouteRefusal::HazardProp
            };
            let candidate =
                content.filter(|_| probe_byte > start_byte && probe_byte < line_end_byte);
            let Some((content, advance_cols)) = candidate else {
                scan.hazards.push((probe_offset, hazard_reason));
                let Some(change) = next_change_after(probe_byte) else {
                    break;
                };
                probe_byte = change;
                continue;
            };
            // Covered extent: the producer's rule, not a second copy of it.
            // `ReplacementCoveredSpan` owns "how far does one display property
            // reach", and it takes the SOURCE so the overlay-vs-text branch
            // cannot be skipped; here the source is always a text property
            // (see `RoutedScanExtentLookup`), which is GNU's
            // `display_prop_end` — the run over which the value stays the same
            // object.
            let span = ReplacementCoveredSpan::for_property_source(
                CharPropertySource {
                    value,
                    overlay: None,
                },
                buffer.layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(probe_byte)),
                next_change_after(probe_byte)
                    .map(|change| {
                        buffer.layout_emacs_byte_pos_to_char_pos(EmacsBytePos::new(change))
                    })
                    .unwrap_or(line_end),
                &RoutedScanExtentLookup { buffer, line_end },
            );
            // The one question the shared rule does not answer: routability.
            // The covered range must end strictly BEFORE the line end, since
            // an extent reaching the newline hides it — a line-structure
            // change. Reaching the bound is not itself disqualifying: a range
            // that merely ends there while some other value holds at the line
            // end covers no newline.
            let covers_line_end = span.resume() >= line_end
                && display_prop_at(line_end_byte).is_some_and(|next| next.bits() == value.bits());
            if covers_line_end {
                scan.hazards.push((probe_offset, RouteRefusal::Replacement));
                break;
            }
            scan.replacements.push(RoutedRowReplacement {
                start: probe_offset,
                covered: span,
                value,
                content,
                advance_cols,
            });
            probe_byte = buffer
                .layout_char_pos_to_emacs_byte_pos(span.resume())
                .get();
            continue;
        }
        let Some(change) = next_change_after(probe_byte) else {
            break;
        };
        if change > line_end_byte {
            break;
        }
        probe_byte = change;
    }
    Ok(scan)
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
/// overlay-sourced) refuse through [`routed_row_elision_scan`]. `composition` is
/// not on this list since phase 2e: its refusal is grounded in the
/// pipeline's own replacement predicate ([`routed_composition_prop_replaces`]),
/// so an inert (unparseable) prop no longer refuses.
/// `display` is NOT probed here since increment 2i: the dedicated
/// [`routed_row_replacement_scan`] owns every display-prop decision (routable
/// string replacements become plan spans; everything else records a
/// positioned hazard the classifier applies against the routed coverage).
const ROUTE_HAZARD_TEXT_PROPS: [&str; 2] = ["mouse-face", "line-height"];

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
    plan_ascii_row_classified(buffer, row, fit, policy, RowRouteEntry::LineStart).ok()
}

/// [`plan_ascii_row`] with the refusal reason preserved for the coverage
/// telemetry histogram.
pub(crate) fn plan_ascii_row_classified<B: LayoutBufferView>(
    buffer: &B,
    row: RowRouteRowStart<'_>,
    fit: RowRouteFit<'_>,
    policy: RowRouteWindowPolicy,
    entry: RowRouteEntry,
) -> Result<AsciiRowPlan, RouteRefusal> {
    if policy.hscroll_active {
        return Err(RouteRefusal::PolicyHscroll);
    }
    if policy.selective_display != 0 {
        return Err(RouteRefusal::PolicySelectiveDisplay);
    }
    if policy.word_wrap {
        return Err(RouteRefusal::PolicyWordWrap);
    }
    if policy.show_trailing_whitespace {
        return Err(RouteRefusal::PolicyTrailingWhitespace);
    }
    // Whole rows are routed from a buffer line start; since increment 2j a
    // CONTINUATION-ROW RESUME (the dispatch attests: no pending items, the
    // current row is the wrap transition's flagged continuation row) may
    // also enter mid-line — the remaining tail of a visually wrapped line
    // is classified exactly like a line, from the resume charpos and the
    // live pen. Every other mid-line position (a display element, an
    // overlay string, an overflow handoff char on the FIRST row of a
    // continued line — its row is not flagged Continuation yet, so the
    // pipeline's own overflow machinery keeps consuming it) stays refused.
    if row.byte_idx > 0
        && row.text.get(row.byte_idx - 1) != Some(&b'\n')
        && entry != RowRouteEntry::ContinuationResume
    {
        return Err(RouteRefusal::MidLineStart);
    }
    // Cheap pre-gate BEFORE the pen walk: the refusal the steady-state edit
    // path hits every keystroke — the cursor row, including the common
    // typing-at-EOB shape where the cursor row IS the newline-less tail row
    // — is decidable from a newline search plus arithmetic, so the per-row
    // probe cost on it is one memchr (plus, inside the ambiguous byte range,
    // one branch-light count), not a full classifier walk.
    // * Line extent: up to the newline, or — phase 2h rung 2 — the whole
    //   remaining text when no newline exists (the end-of-source tail line;
    //   the read bound never cuts mid-line).
    // * Point on this line (through its newline, or through the tail's
    //   end-of-buffer position): refuse. Byte length bounds char length from
    //   above, so `point > charpos + line_byte_len` proves point is past
    //   this line with no further work; inside that ambiguous byte range,
    //   one branch-light non-continuation-byte count gives the EXACT char
    //   length, deciding line membership without the pen walk. This
    //   deliberately refuses point in the unrouted tail of an over-wide line
    //   too (which phase 2f used to route): the cursor row is the row the
    //   steady-state edit path re-lays every keystroke, and its refusal must
    //   cost a memchr, not a classifier walk. For the end-of-source tail the
    //   inclusive upper bound is one past the last char — GNU places the EOB
    //   cursor on that row (xdisp.c:26811, first ends_at_zv row wins).
    let line_byte_len =
        memchr::memchr(b'\n', &row.text[row.byte_idx..]).unwrap_or(row.text.len() - row.byte_idx);
    if policy.point_charpos >= row.charpos
        && policy.point_charpos <= row.charpos + line_byte_len as i64
    {
        let line_char_len = row.text[row.byte_idx..row.byte_idx + line_byte_len]
            .iter()
            .filter(|&&byte| (byte & 0xC0) != 0x80)
            .count();
        if policy.point_charpos <= row.charpos + line_char_len as i64 {
            return Err(RouteRefusal::PointInRow);
        }
    }
    // Display-property scan over the whole line FIRST (increment 2i rung 2):
    // routable string replacements become plan spans the fit walk credits
    // with the STRING's width; unroutable display shapes are recorded with
    // their positions and refuse below only when they fall inside the routed
    // coverage (a prop in an over-wide line's unreached tail stays with the
    // pipeline at resume, preserving the phase-2f class).
    let start_byte = row.text_start_byte + row.byte_idx;
    let display_scan =
        routed_row_replacement_scan(buffer, row.charpos, start_byte, start_byte + line_byte_len)?;

    // Overlay-string anchors over the whole line, for the same reason the
    // display scan runs first: their strings widen the pen, so the fit walk
    // below cannot find the coverage end without them.
    let overlay_scan_strings = routed_row_overlay_string_scan(
        buffer,
        policy.overlay_string_window,
        row.charpos,
        start_byte,
        start_byte + line_byte_len,
    )?;
    let mut overlay_strings = overlay_scan_strings.anchors;
    // A row carries EITHER anchors OR replacements, never both: their pen
    // bookkeeping would have to interleave (an anchor exactly at a covered
    // range's start has no defined order against it), and neither the plan's
    // parts nor the fit walk encode that precedence.
    if !overlay_strings.is_empty() && !display_scan.replacements.is_empty() {
        return Err(RouteRefusal::Overlay);
    }

    // One pass scans the chars (refusing anything the pipeline would
    // compose) AND applies the strict logical-cell fit: a line exactly
    // filling the row keeps the buffer pipeline (continuation/truncation
    // policy owns that edge).
    let scan = routed_line_scan(
        row.text,
        row.byte_idx,
        fit,
        &display_scan.replacements,
        &overlay_strings,
    )?;

    // An anchor whose strings are not routable refuses only when the routed
    // coverage reaches it (inclusive of the end position, like the display
    // hazard walk below). One in an over-wide line's unreached tail stays
    // with the pipeline at resume, exactly as an unroutable display prop
    // there does.
    if overlay_scan_strings
        .hazards
        .iter()
        .any(|&offset| offset <= scan.char_len)
    {
        return Err(RouteRefusal::Overlay);
    }

    // Anchor POSITION against the routed coverage, now that the fit walk has
    // found where that coverage ends.
    if scan.line_end == RoutedRowLineEnd::OverflowHandoff {
        // An overflow-prefix plan refuses ANY anchor it reaches: the append
        // session clips at the right edge and can break the row on its own,
        // so a handoff cut taken mid-anchor is not the pipeline's overflow
        // point. Anchors BEYOND the cut are unrouted remainder the pipeline
        // emits at resume, and DROPPING those rather than refusing the row is
        // what lets the continuation rows of a long wrapped line keep routing
        // when the line carries an anchor further down it. It is also what
        // keeps the producer's once-per-anchor marker honest: the routed
        // prefix never emits a string the resumed walk would emit again.
        if overlay_strings
            .iter()
            .any(|anchor| anchor.at <= scan.char_len)
        {
            return Err(RouteRefusal::Overlay);
        }
        overlay_strings.clear();
    } else if overlay_strings
        .iter()
        .any(|anchor| anchor.at >= scan.char_len)
    {
        // A whole-line or end-of-source plan covers the line, so its coverage
        // end IS the line end (or the end-of-buffer position). An anchor
        // there precedes a line end the pipeline's own line-break lifecycle
        // owns, or sits at the EOB position whose strings the post-loop tail
        // collects for itself.
        return Err(RouteRefusal::Overlay);
    }

    // Unroutable display props refuse when they touch the routed coverage
    // (inclusive of the end position, mirroring the historical hazard walk
    // which probed the newline / handoff char too).
    if let Some(&(_, reason)) = display_scan
        .hazards
        .iter()
        .find(|&&(offset, _)| offset <= scan.char_len)
    {
        return Err(reason);
    }
    // Keep only the replacements the routed coverage actually consumed; a
    // candidate at or beyond an overflow handoff is unrouted remainder.
    let mut replacements = display_scan.replacements;
    replacements.retain(|replacement| replacement.end() <= scan.char_len);
    // A replacement row never routes as an overflow prefix: the scan's
    // handoff cut is not the pipeline's overflow point once the covered
    // span's width substitution is in play.
    if scan.line_end == RoutedRowLineEnd::OverflowHandoff && !replacements.is_empty() {
        return Err(RouteRefusal::Replacement);
    }

    // Cursor capture stays on the buffer pipeline: exclude any row whose
    // ROUTED coverage contains point. The pre-gate above already refused the
    // byte-superset (point anywhere on the line), so this precise check is
    // defense-in-depth for the coverage interval itself.
    let routed_end_charpos = row.charpos + scan.char_len as i64;
    if policy.point_charpos >= row.charpos && policy.point_charpos <= routed_end_charpos {
        return Err(RouteRefusal::PointInRow);
    }

    // Overlays intersecting the row (touching endpoints included) may carry
    // ONLY face-affecting properties; their in-line boundaries become face
    // segment boundaries below. Anything else — strings, display, invisible,
    // window restriction, category indirection — keeps the buffer pipeline.
    let routed_end_byte = start_byte + scan.byte_len;
    let overlay_scan = routed_row_overlay_scan(buffer, row.charpos, start_byte, routed_end_byte)
        .ok_or(RouteRefusal::Overlay)?;

    // An active display table can remap any char (including the newline).
    if crate::neovm_bridge::buffer_has_active_display_table(buffer) {
        return Err(RouteRefusal::DisplayTable);
    }

    // Invisible text: accept only the plain-elision class (hidden spans that
    // simply drop chars from the row); ellipsis, newline-spanning folds,
    // row-start runs, and non-advancing skips refuse. Overlay-sourced
    // invisibility never reaches this scan — any intersecting overlay
    // carrying `invisible` already refused through the overlay allow-list.
    let elided = routed_row_elision_scan(buffer, row.charpos, routed_end_charpos)
        .ok_or(RouteRefusal::Elision)?;

    // Conservative composition refusal: a routed row carries EITHER plain
    // elision OR a replacement, never both — their skip bookkeeping would
    // interleave (and GNU's handler order makes a replacing display beat
    // invisible inside the covered range, a precedence the plan's disjoint
    // gap model does not encode).
    if !replacements.is_empty() && !elided.is_empty() {
        return Err(RouteRefusal::Replacement);
    }

    // Same conservatism for an anchor meeting a hidden span: an insertion at
    // a gap edge has no defined order against the skip, and GNU's handler
    // order (invisible before overlay strings at the same stop) is not
    // something the plan's disjoint-gap model encodes.
    if !overlay_strings.is_empty() && !elided.is_empty() {
        return Err(RouteRefusal::Overlay);
    }

    // An overflow-prefix plan refuses ANY elision inside its coverage: the
    // scan's fit walk advanced the pen for every char including hidden ones,
    // so its handoff cut would not be the pipeline's overflow point. (A
    // hidden run beyond the handoff is unrouted remainder — the elision scan
    // above never sees it, and the pipeline handles it at resume.)
    if scan.line_end == RoutedRowLineEnd::OverflowHandoff && !elided.is_empty() {
        return Err(RouteRefusal::OverflowElision);
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
                return Err(RouteRefusal::HazardProp);
            }
        }
        // Static composition: refuse exactly when the pipeline's replacement
        // predicate would fire (an inert prop still segments below, like any
        // other property change).
        if routed_composition_prop_replaces(buffer, probe) {
            return Err(RouteRefusal::HazardProp);
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
            let char_offset = change_charpos
                .checked_sub(row.charpos.max(0) as usize)
                .ok_or(RouteRefusal::Boundary)?;
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
    // An anchor IS an overlay endpoint, so the boundary merge above already
    // cut a text segment there. The routed commit relies on it: an insertion
    // part is ordered ahead of the text segment that STARTS at its position,
    // which only exists because of this cut.
    debug_assert!(
        overlay_strings
            .iter()
            .all(|anchor| face_boundaries.binary_search(&anchor.at).is_ok()),
        "every overlay-string anchor must have cut a face segment"
    );

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
            return Err(RouteRefusal::ComposedSeam);
        }
    }

    Ok(AsciiRowPlan {
        line_byte_len: scan.byte_len,
        line_char_len: scan.char_len,
        has_tab: scan.has_tab,
        has_wide: scan.has_wide,
        has_overlay: overlay_scan.has_overlay,
        face_boundaries,
        elided,
        composed: scan.composed,
        replacements,
        overlay_strings,
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

/// Gate for the routed acquisition. Default ON since phase 3 (2026-08-06):
/// the flip was justified by measured coverage (TUI suite: 37.8% of
/// line-start row attempts routed) at neutral protected-workload cost
/// (TTY keystroke instruction count +0.035% flag-on after the cursor/EOB
/// pre-gate, word-wrap refused-heavy +0.016% — both within run noise).
/// `NEOMACS_ROW_ITEM_ROUTE=off` opts OUT, restoring the pure buffer
/// pipeline (kept for soak time; removal is phase 4 business). Any other
/// value (including the historical opt-in `ascii`) leaves the route on.
pub(crate) fn row_item_route_ascii_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED
        .get_or_init(|| !std::env::var("NEOMACS_ROW_ITEM_ROUTE").is_ok_and(|value| value == "off"))
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

/// Test-only engagement proof for the P4.6 rung-4 un-refusal: routed rows
/// that carried at least one overlay-string anchor.
#[cfg(test)]
pub(crate) static ROUTED_OVERLAY_STRING_ROW_COUNT: std::sync::atomic::AtomicUsize =
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

/// Test-only engagement proof for the phase-2h empty-line extension: routed
/// bare-newline rows rendered RowBreak-only.
#[cfg(test)]
pub(crate) static ROUTED_EMPTY_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the phase-2h EOB-tail extension: routed
/// newline-less tail rows ending at the source end.
#[cfg(test)]
pub(crate) static ROUTED_EOB_TAIL_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the increment 2i display-replacement
/// extension: routed rows containing at least one display-string replacement
/// rendered through the pipeline's replacement session.
#[cfg(test)]
pub(crate) static ROUTED_REPLACEMENT_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Test-only engagement proof for the increment 2j continuation-row resume
/// extension: rows routed through the mid-line
/// [`RowRouteEntry::ContinuationResume`] entry.
#[cfg(test)]
pub(crate) static ROUTED_CONTINUATION_RESUME_ROW_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Telemetry twin of the test-only resume counter: routed rows taken through
/// the continuation-resume entry, reported on the stats line as
/// `routed_resume` so real workloads show the increment-2j contribution.
static ROUTE_STAT_ROUTED_RESUME: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn note_routed_row(plan: &AsciiRowPlan, wrap_mode: LineWrapMode, entry: RowRouteEntry) {
    if route_stats_file().is_some() {
        ROUTE_STAT_ROUTED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if entry == RowRouteEntry::ContinuationResume {
            ROUTE_STAT_ROUTED_RESUME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    #[cfg(not(test))]
    let _ = wrap_mode;
    #[cfg(test)]
    {
        ROUTED_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if entry == RowRouteEntry::ContinuationResume {
            ROUTED_CONTINUATION_RESUME_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
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
        if plan.has_overlay_strings() {
            ROUTED_OVERLAY_STRING_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
        if plan.is_empty_line() {
            ROUTED_EMPTY_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if plan.is_end_of_source() {
            ROUTED_EOB_TAIL_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if plan.has_replacement() {
            ROUTED_REPLACEMENT_ROW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

        note_route_attempt();
        // Increment-2j sub-cause telemetry inputs: the current row index and
        // whether that row is a continuation row of a visually wrapped line
        // (the flag is stamped by the wrap transition BEFORE anything is
        // appended to the new row, so it is already visible here).
        let route_row_index = match self.row_geometry.current_row_marker() {
            crate::display_row::geometry::DisplayRowMarker::Row(index) => index,
            #[cfg(test)]
            crate::display_row::geometry::DisplayRowMarker::Inactive => 0,
        };
        let route_continuation_row = self.row_flags.is_set(
            route_row_index,
            crate::display_row::geometry::DisplayRowFlagKind::Continuation,
        );
        let route_site_key = route_stats_site_key(route_row_index, self.progress.charpos());
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
            overlay_string_window: self.overlay_context.string_window_id(),
        };
        // Increment 2j: a mid-line position qualifies for the continuation-
        // row resume entry exactly when the current row is the wrap
        // transition's flagged continuation row and the walk carries no
        // pending split-run remainders (checked above — the character-wrap
        // rewind cleared them and reseated the cursor at the wrap char).
        let entry = if route_continuation_row {
            RowRouteEntry::ContinuationResume
        } else {
            RowRouteEntry::LineStart
        };
        let plan = match plan_ascii_row_classified(buffer, row, fit, policy, entry) {
            Ok(plan) => plan,
            Err(RouteRefusal::MidLineStart) => {
                note_route_refusal_midline(route_continuation_row, route_site_key);
                return AsciiRowRouteOutcome::NotRouted;
            }
            Err(reason) => {
                note_route_refusal(reason);
                return AsciiRowRouteOutcome::NotRouted;
            }
        };

        // Phase 2h rung 1: a bare-newline empty line renders RowBreak-only —
        // no text probe/commit; the row break drives the shared line-end
        // plan and row transition directly.
        if plan.is_empty_line() {
            return self.render_routed_empty_row_break(
                loop_context,
                source_walk,
                text,
                active_face_state,
                buffer,
                row,
                &plan,
                policy.wrap_mode,
                entry,
            );
        }

        let start = CharPos0::new(row.charpos.max(0) as usize);
        // The routed row renders as an ordered sequence of PARTS: visible
        // text segments and (increment 2i) display-replacement spans, merged
        // by char position. A replacement part renders through the
        // pipeline's OWN replacement session at commit; the probe phase
        // predicts its advance with the session's base-face resolution.
        enum RoutedRowPartKind<'plan> {
            Text,
            Replacement(&'plan RoutedRowReplacement),
            /// P4.6: an INSERTION — `start == end`, no chars consumed.
            OverlayStrings(&'plan RoutedRowOverlayStrings),
        }
        impl RoutedRowPartKind<'_> {
            /// Tie-break for parts sharing a start position. An insertion
            /// belongs BEFORE the text segment that starts where it sits —
            /// GNU's `handle_stop` order, and the same insertion semantics
            /// the producer gives the element. Sorting by position alone
            /// would land every string one character late.
            fn order_rank(&self) -> u8 {
                match self {
                    Self::OverlayStrings(_) => 0,
                    Self::Text | Self::Replacement(_) => 1,
                }
            }

            fn is_insertion(&self) -> bool {
                matches!(self, Self::OverlayStrings(_))
            }
        }
        struct RoutedRowPart<'plan> {
            start: CharPos0,
            end: CharPos0,
            kind: RoutedRowPartKind<'plan>,
        }
        let mut parts: Vec<RoutedRowPart> = plan
            .segment_ranges(start)
            .into_iter()
            .map(|(seg_start, seg_end)| RoutedRowPart {
                start: seg_start,
                end: seg_end,
                kind: RoutedRowPartKind::Text,
            })
            .collect();
        parts.extend(plan.replacements().iter().map(|replacement| RoutedRowPart {
            start: start.add_len(CharLen::new(replacement.start)),
            end: start.add_len(CharLen::new(replacement.end())),
            kind: RoutedRowPartKind::Replacement(replacement),
        }));
        parts.extend(plan.overlay_strings().iter().map(|anchor| {
            let at = start.add_len(CharLen::new(anchor.at()));
            RoutedRowPart {
                start: at,
                end: at,
                kind: RoutedRowPartKind::OverlayStrings(anchor),
            }
        }));
        parts.sort_by_key(|part| (part.start.get(), part.kind.order_rank()));
        debug_assert!(
            parts
                .first()
                .is_none_or(|part| matches!(part.kind, RoutedRowPartKind::Text)),
            "a routed row never starts with a replacement or an insertion \
             (row-start anchors refuse)"
        );
        let ranges: Vec<(CharPos0, CharPos0)> =
            parts.iter().map(|part| (part.start, part.end)).collect();
        let _ = &ranges;
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
        struct ProbedSegment<'plan> {
            start: CharPos0,
            end: CharPos0,
            kind: RoutedRowPartKind<'plan>,
            active: crate::display_row::face_state::DisplayRowActiveFaceState,
        }
        let mut probed: Vec<ProbedSegment> = Vec::with_capacity(ranges.len());
        let mut carried_active: Option<crate::display_row::face_state::DisplayRowActiveFaceState> =
            None;
        for (index, part) in parts.into_iter().enumerate() {
            let (seg_start, seg_end) = (&part.start, &part.end);
            let active = if index == 0 {
                active_face_state.clone()
            } else if part.kind.is_insertion() {
                // An insertion resolves its OWN base face per string (GNU
                // face_for_overlay_string ignores the surrounding run), so
                // resolving a segment face here would mint an id nothing
                // uses. Carry the previous part's face through instead — the
                // box-face check below still sees the row's real faces.
                carried_active
                    .clone()
                    .unwrap_or_else(|| active_face_state.clone())
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
            // replicate; keep the multi-face class box-free. A continuation
            // resume is stricter: the row may already carry glyphs the
            // pipeline appended (a box run could be OPEN across the entry),
            // so ANY box face refuses the resume entry.
            if (plan.is_segmented() || entry == RowRouteEntry::ContinuationResume)
                && active.resolved_face().box_type != 0
            {
                note_route_refusal(RouteRefusal::ProbeBoxFace);
                return AsciiRowRouteOutcome::NotRouted;
            }
            // The pipeline stamps glyphs with the PER-RUN face chain; refuse
            // the row when it would diverge from the checkpoint chain (e.g.
            // buffer face remapping of default). A replacement part's glyphs
            // take the SESSION's base-face resolution instead — the run
            // chain never applies there.
            if matches!(part.kind, RoutedRowPartKind::Text)
                && routed_segment_item_face_diverges(
                    buffer,
                    face_resolution_context.face_resolver(),
                    self.face_ids,
                    face_resolution_context.default_resolved(),
                    face_resolution_context.default_face_id(),
                    *seg_start,
                    active.face_id(),
                )
            {
                note_route_refusal(RouteRefusal::ProbeFaceDiverges);
                return AsciiRowRouteOutcome::NotRouted;
            }
            carried_active = Some(active.clone());
            probed.push(ProbedSegment {
                start: *seg_start,
                end: *seg_end,
                kind: part.kind,
                active,
            });
        }

        let geometry = *self.row_geometry;
        let mut probe_position = position;
        for segment in &probed {
            // Advance-based measurement: tab expansion depends on the pen x
            // and 2-col chars advance two cells, so the measured END position
            // (x AND col) seeds the next segment's probe exactly as the
            // pipeline's own natural walk would. A replacement part measures
            // the SESSION's shape: the string's chars as one covered
            // SourceMappedText run in the session's base face (the same
            // resolution `DisplayPropertyReplacementAppendPlanItemRequest`
            // performs at commit; content-addressed mints only).
            let measured = match &segment.kind {
                // P4.6: the strings the retained session will append at this
                // anchor, measured one after another from the running pen.
                // The base face is resolved through
                // `DisplayOrigin::OverlayString` — GNU
                // `face_for_overlay_string` (xfaces.c:7034-7092) "simply
                // disregards the `face' properties of all overlays", so
                // resolving through `DisplayPropertyString` here would tint
                // the string with the overlay's own face and mis-measure a
                // box or a different font. Resolution only (no pending-face
                // install): the session performs its own at commit.
                RoutedRowPartKind::OverlayStrings(anchor) => {
                    let mut position = Some(probe_position);
                    for overlay_string in anchor.strings() {
                        let Some(at) = position else {
                            break;
                        };
                        let Some(text) = overlay_string.string.as_utf8_str() else {
                            position = None;
                            break;
                        };
                        let origin = DisplayOrigin::OverlayString {
                            overlay_id: overlay_string.overlay_id,
                            anchor_charpos: segment.start,
                            kind: if overlay_string.after_string_p {
                                crate::display_origin::OverlayStringKind::After
                            } else {
                                crate::display_origin::OverlayStringKind::Before
                            },
                        };
                        let base_face = crate::display_source_resolver::resolve_display_string_base_face(
                            buffer,
                            face_resolution_context.face_resolver(),
                            origin,
                            origin.default_base_face_policy(),
                            None,
                            crate::display_source_resolver::DisplayDefaultFaceInstallPolicy::ReuseInstalledDefaultFace,
                            self.face_ids,
                        );
                        // The session appends the string's chars one by one
                        // through the Lisp-string source; measuring them as
                        // one text item in the same face gives the same pen
                        // advance for the routable class (no tabs, no
                        // newlines, no per-char property changes), and the
                        // commit re-reads the session's OWN end position, so
                        // this prediction only has to keep the fit check
                        // honest.
                        let item = DisplayItem::new(
                            SourceSpan::new(
                                DisplaySourcePosition::buffer(
                                    loop_context.buffer_id(),
                                    segment.start,
                                    buffer.layout_char_pos_to_emacs_byte_pos(segment.start),
                                ),
                                DisplaySourcePosition::buffer(
                                    loop_context.buffer_id(),
                                    segment.start,
                                    buffer.layout_char_pos_to_emacs_byte_pos(segment.start),
                                ),
                            ),
                            RenderFaceRef::FaceId(base_face.face_id()),
                            DisplayItemKind::SourceMappedText(
                                crate::display_item::DisplaySourceMappedText::new(text),
                            ),
                        );
                        let append_context = BufferSourceRowAppendContext::from_active_face_row(
                            buffer,
                            loop_context.buffer_id(),
                            self.append_surface,
                            &segment.active,
                            0.0,
                            loop_context.char_height(),
                            self.face_ids.clone(),
                        )
                        .with_resolved_item_face(base_face.face_id(), base_face.face().clone());
                        let mut measure = self.source_render.measure_state();
                        position = append_context.measure_source_display_item_advance_naturally(
                            &geometry,
                            &mut measure,
                            &item,
                            at,
                            DisplayRowAppendKind::SourceText,
                        );
                    }
                    position
                }
                RoutedRowPartKind::Text => {
                    let mut source = BufferAsciiItemSource::text_only(
                        loop_context.buffer_id(),
                        buffer,
                        segment.start,
                        segment.end,
                        RenderFaceRef::FaceId(segment.active.face_id()),
                    );
                    let Some(text_item) = source.text_item().cloned() else {
                        note_route_refusal(RouteRefusal::ProbeMeasure);
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
                    let mut measure = self.source_render.measure_state();
                    append_context.measure_source_display_item_advance_naturally(
                        &geometry,
                        &mut measure,
                        &text_item,
                        probe_position,
                        DisplayRowAppendKind::SourceText,
                    )
                }
                RoutedRowPartKind::Replacement(replacement)
                    if matches!(replacement.content, RoutedReplacementContent::SpaceWidth) =>
                {
                    // Rung 3 probe: resolve the spec through the SAME request
                    // the session renders (resolution only — metric queries,
                    // no append) and take its stretch width; the commit's
                    // append advances by exactly this width, with the column
                    // advance mirroring the builder's rounding.
                    let start_byte_pos = buffer.layout_char_pos_to_emacs_byte_pos(segment.start);
                    let end_byte_pos = buffer.layout_char_pos_to_emacs_byte_pos(segment.end);
                    let replacement_item =
                        crate::display_item::BufferDisplayPropertyReplacementItem::new(
                            replacement.value,
                            crate::display_property::classify_display_property(replacement.value),
                            crate::display_item::BufferDisplayReplacementSource::spanning(
                                loop_context.buffer_id(),
                                segment.start,
                                start_byte_pos,
                                segment.end,
                                end_byte_pos,
                            ),
                            start_byte_pos,
                            end_byte_pos,
                            replacement.covered,
                        );
                    let fallback_metrics =
                        crate::buffer_source::item_append::BufferSourceActiveFaceRowMetrics::from_active_face_row(
                            &segment.active,
                            loop_context.char_height(),
                        )
                        .fallback_metrics();
                    replacement_item
                        .source_text(loop_context.text_start_byte(), text)
                        .and_then(|source_text| {
                            self.source_render
                                .resolve_display_property_replacement_row_request(
                                    replacement_item.descriptor(),
                                    source_text,
                                    &segment.active,
                                    probe_position.x_px(),
                                    loop_context.content_x(),
                                    params,
                                    0.0,
                                    fallback_metrics,
                                    probe_position,
                                )
                        })
                        .and_then(|request| request.stretch_width_px())
                        .map(|width_px| {
                            if width_px <= 0.0 {
                                // A non-positive stretch appends nothing
                                // (the session's from_stretch Empty arm).
                                probe_position
                            } else {
                                DisplayRowPosition::new(
                                    probe_position.x_px() + width_px,
                                    probe_position.col()
                                        + (width_px / params.char_width.max(1.0)).round() as usize,
                                )
                            }
                        })
                }
                RoutedRowPartKind::Replacement(replacement) => {
                    let base_face = crate::display_source_resolver::resolve_display_string_base_face(
                        buffer,
                        face_resolution_context.face_resolver(),
                        DisplayOrigin::DisplayPropertyString {
                            anchor_charpos: segment.start,
                            source: crate::display_origin::DisplayPropertySource::TextProperty,
                        },
                        DisplayOrigin::DisplayPropertyString {
                            anchor_charpos: segment.start,
                            source: crate::display_origin::DisplayPropertySource::TextProperty,
                        }
                        .default_base_face_policy(),
                        Some(crate::display_source_resolver::ActiveDisplayStringBaseFace::new(
                            segment.active.face_id(),
                            segment.active.resolved_face(),
                        )),
                        crate::display_source_resolver::DisplayDefaultFaceInstallPolicy::ReuseInstalledDefaultFace,
                        self.face_ids,
                    );
                    let item = DisplayItem::new(
                        SourceSpan::new(
                            DisplaySourcePosition::buffer(
                                loop_context.buffer_id(),
                                segment.start,
                                buffer.layout_char_pos_to_emacs_byte_pos(segment.start),
                            ),
                            DisplaySourcePosition::buffer(
                                loop_context.buffer_id(),
                                segment.end,
                                buffer.layout_char_pos_to_emacs_byte_pos(segment.end),
                            ),
                        ),
                        RenderFaceRef::FaceId(base_face.face_id()),
                        DisplayItemKind::SourceMappedText(
                            crate::display_item::DisplaySourceMappedText::new(
                                match &replacement.content {
                                    RoutedReplacementContent::String { text } => text.as_ref(),
                                    RoutedReplacementContent::SpaceWidth => unreachable!(
                                        "space replacements probe through the resolved request"
                                    ),
                                },
                            ),
                        ),
                    );
                    let append_context = BufferSourceRowAppendContext::from_active_face_row(
                        buffer,
                        loop_context.buffer_id(),
                        self.append_surface,
                        &segment.active,
                        0.0,
                        loop_context.char_height(),
                        self.face_ids.clone(),
                    )
                    .with_resolved_item_face(base_face.face_id(), base_face.face().clone());
                    let mut measure = self.source_render.measure_state();
                    append_context.measure_source_display_item_advance_naturally(
                        &geometry,
                        &mut measure,
                        &item,
                        probe_position,
                        DisplayRowAppendKind::SourceText,
                    )
                }
            };
            let Some(end_position) = measured else {
                note_route_refusal(RouteRefusal::ProbeMeasure);
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
            // decision fires at the handoff char. End-of-source tail plans
            // (phase 2h rung 2) share the `<=` bound: with no char after
            // the tail there is no continuation/truncation edge, the walk
            // simply ends at the source end.
            let fits = if plan.is_overflow_handoff() || plan.is_end_of_source() {
                probe_position.x_px() <= self.append_surface.right_edge()
            } else {
                probe_position.x_px() < self.append_surface.right_edge()
            };
            if !fits {
                note_route_refusal(RouteRefusal::ProbeMeasure);
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
            // P4.6: DELEGATE the anchor's strings to the pipeline's own
            // session — the identical call `render.rs`'s loop-level element
            // arm makes, with the loop state this commit already owns. The
            // producer decided WHERE and in WHICH order (its collection is
            // what the plan carries); this side owns only the append.
            //
            // No face checkpoint runs first: an insertion consumes no chars,
            // so the position's checkpoint belongs to the text segment that
            // starts here, and it fires on the next iteration. The strings
            // resolve their own base face regardless.
            if let RoutedRowPartKind::OverlayStrings(anchor) = &segment.kind {
                self.progress.apply_row_position(render_position);
                let anchor_charpos = segment.start.get() as i64;
                let (x, col) = self.progress.row_progress_mut().coordinates_mut();
                let continuation = self.overlay_context.render_produced_strings_at_text_row(
                    buffer,
                    anchor_charpos,
                    anchor.strings(),
                    self.source_render.reborrow(),
                    x,
                    col,
                    self.row_geometry,
                    self.cursor_info,
                    self.hit_rows,
                    self.hit_row_range,
                    self.row_y_positions,
                    self.face_ids,
                    self.line_numbers,
                    self.face_scan,
                );
                // A routable overlay string holds no newline and the plan
                // refused every anchor an overflow prefix reaches, so the
                // session has neither a row break nor a clip to report.
                debug_assert!(
                    !continuation.should_break(),
                    "routed overlay strings are single-line and fit the row"
                );
                if continuation.should_break() {
                    return AsciiRowRouteOutcome::Stopped;
                }
                let committed = self.progress.row_position();
                debug_assert_eq!(
                    committed.col(),
                    render_position.col() + anchor.advance_cols(),
                    "the classifier's fit walk credited a different advance than \
                     the session appended"
                );
                render_position = committed;
                continue;
            }

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

            // A replacement part (increment 2i): render through the
            // pipeline's OWN replacement session — the same context
            // `render.rs` consume_replacement builds — so glyph provenance
            // (covered-start charpos), string base-face policy, and the
            // walk/progress bookkeeping are the session's, verbatim. The
            // pipeline performs no remember-face/row-extend bookkeeping for
            // a consumed replacement, so neither does the routed commit.
            if let RoutedRowPartKind::Replacement(replacement) = &segment.kind {
                use crate::buffer_source::display_property_render::{
                    BufferDisplayPropertyTextReplacementApplyOutcome,
                    BufferDisplayPropertyTextReplacementRenderContext,
                    BufferDisplayPropertyTextReplacementRenderState,
                };
                let start_byte_pos = buffer.layout_char_pos_to_emacs_byte_pos(segment.start);
                let end_byte_pos = buffer.layout_char_pos_to_emacs_byte_pos(segment.end);
                let replacement_item =
                    crate::display_item::BufferDisplayPropertyReplacementItem::new(
                        replacement.value,
                        crate::display_property::classify_display_property(replacement.value),
                        crate::display_item::BufferDisplayReplacementSource::spanning(
                            loop_context.buffer_id(),
                            segment.start,
                            start_byte_pos,
                            segment.end,
                            end_byte_pos,
                        ),
                        start_byte_pos,
                        end_byte_pos,
                        replacement.covered,
                    );
                self.progress.apply_row_position(render_position);
                let replacement_context = BufferDisplayPropertyTextReplacementRenderContext::new(
                    replacement_item,
                    loop_context.text_start_byte(),
                    text,
                    loop_context.content_x(),
                    params,
                    0.0,
                    loop_context.char_height(),
                    active_face_state,
                    self.progress.row_progress().x(),
                    self.progress.row_position(),
                );
                match replacement_context.render_and_apply(
                    buffer,
                    BufferDisplayPropertyTextReplacementRenderState::new(
                        self.source_render.reborrow(),
                        self.face_ids,
                        self.append_surface,
                        self.row_geometry,
                        active_face_state,
                    ),
                    &mut self.progress,
                    self.cursor_info,
                    loop_context.point_charpos(),
                ) {
                    BufferDisplayPropertyTextReplacementApplyOutcome::Applied {
                        produced_row_break,
                    } => {
                        // A routed replacement string contains no newline
                        // (classifier), so the session never breaks the row.
                        debug_assert!(
                            !produced_row_break,
                            "routed replacement strings are single-line"
                        );
                        if produced_row_break {
                            return AsciiRowRouteOutcome::Stopped;
                        }
                        render_position = self.progress.row_position();
                    }
                    BufferDisplayPropertyTextReplacementApplyOutcome::Fallback(_) => {
                        // Unreachable for a classified plain string (the
                        // resolver only falls back when the spec is not a
                        // utf8 string). Render the covered text literally —
                        // the same glyphs the pipeline's fallback appends.
                        debug_assert!(false, "a classified replacement string must resolve");
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
                        let Some(append_progress) = append_context
                            .render_display_item_source_to_text_row(
                                &geometry,
                                &mut self.source_render.reborrow(),
                                &mut source,
                                &mut source_state,
                                render_position,
                                DisplayRowAppendKind::SourceText,
                                &mut render_policy,
                            )
                        else {
                            return AsciiRowRouteOutcome::Stopped;
                        };
                        render_position = append_progress.end();
                        self.progress.apply_row_position(render_position);
                    }
                    BufferDisplayPropertyTextReplacementApplyOutcome::Stop => {
                        return AsciiRowRouteOutcome::Stopped;
                    }
                }
                continue;
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
        note_routed_row(&plan, policy.wrap_mode, entry);
        AsciiRowRouteOutcome::Rendered
    }

    /// Phase 2h rung 1 production: render a classified EMPTY line (a bare
    /// newline) through the item vocabulary's RowBreak-only shape. The
    /// [`BufferAsciiItemSource`] yields exactly one explicit-newline
    /// `RowBreak` at the newline's charpos (shadow-proven glyph-identical to
    /// the pipeline's empty row in engine_test), and that break drives the
    /// SAME shared line-end plan + row-transition lifecycle the pipeline's
    /// newline dispatch uses (`BufferSourceLineBreakRenderRequest` ->
    /// `LineEndContext` -> `line_end::plan` -> `emit_line_break_then_row_start`),
    /// so the finished row carries the pinned empty-row semantics unchanged:
    /// start == end == the newline's charpos, `displays_text` false, the
    /// appended newline space in the line's own face (GNU display_line's
    /// at_end_of_line branch, xdisp.c:26517, with `default_face_p = false`).
    ///
    /// The per-char consumption bookkeeping the pipeline would run before
    /// its dispatch is provably idle for a classified empty row: the
    /// selective-display tail probe (policy refused selective display),
    /// cursor capture (point-on-newline refused by the pre-gate), overlay
    /// strings at eol (the overlay allow-list refused string-bearing
    /// overlays touching the newline; face-only overlays merge through the
    /// shared eol collector inside the line-break render), and pending
    /// source-face installation (the loop's face checkpoint already resolved
    /// and installed the face AT the newline's charpos this iteration).
    #[allow(clippy::too_many_arguments)]
    fn render_routed_empty_row_break<B: LayoutBufferView>(
        &mut self,
        loop_context: crate::buffer_source::loop_context::BufferSourceLoopRequestContext,
        source_walk: &mut crate::buffer_source::walk::BufferSourceWalk<'_, B>,
        text: &[u8],
        active_face_state: &crate::display_row::face_state::DisplayRowActiveFaceState,
        buffer: &B,
        row: RowRouteRowStart<'_>,
        plan: &AsciiRowPlan,
        wrap_mode: LineWrapMode,
        entry: RowRouteEntry,
    ) -> AsciiRowRouteOutcome {
        use crate::display_source::DisplayItemSource as _;

        debug_assert_eq!(plan.line_char_len(), 0);
        debug_assert_eq!(text.get(row.byte_idx), Some(&b'\n'));

        let line_end = CharPos0::new(row.charpos.max(0) as usize);
        let mut source = BufferAsciiItemSource::with_row_break_segments(
            loop_context.buffer_id(),
            buffer,
            &[],
            line_end,
            RenderFaceRef::FaceId(active_face_state.face_id()),
        );
        let mut item_context = crate::display_source::DisplaySourceContext::empty();
        let row_break_item = source
            .next_item(&mut item_context)
            .expect("RowBreak-only source yields exactly the row break");
        debug_assert!(
            matches!(
                row_break_item.kind,
                DisplayItemKind::RowBreak(row_break)
                    if row_break == DisplayRowBreak::explicit_newline()
                        .with_line_height(DisplayLineHeightPolicy::from_property(None))
            ),
            "empty-row production must be the explicit-newline row break"
        );
        debug_assert!(source.next_item(&mut item_context).is_none());

        // Mirror the pipeline's explicit-line-break dispatch
        // (item_render.rs): byte_idx advances past the newline BEFORE the
        // line-break render; charpos is advanced/re-synced inside it.
        let source_char =
            crate::display_source::DisplaySourceStepChar::new('\n', row.byte_idx, row.charpos);
        self.progress.set_byte_idx(row.byte_idx + 1);
        let continuation = loop_context
            .line_break_request(
                source_char,
                text,
                self.append_surface,
                self.overlay_context,
                active_face_state,
            )
            .render_and_apply(source_walk, buffer, self.reborrow());
        if continuation.should_break() {
            return AsciiRowRouteOutcome::Stopped;
        }
        note_routed_row(plan, wrap_mode, entry);
        AsciiRowRouteOutcome::Rendered
    }
}

#[cfg(test)]
#[path = "row_route_test.rs"]
mod tests;
