//! Incremental-layout scaffolding (Phase 0a).
//!
//! This module holds the retained-matrix data types and the per-frame layout
//! instrumentation that the incremental-redisplay work is built on. In Phase
//! 0a NONE of the retained state is *read*: the engine still rebuilds every
//! window every cycle. What ships here is (a) the containers a later phase
//! reuses rows out of, written at the accepted `break` only, and (b) the
//! relaid-row-count metric that gates every phase — without it, a phase whose
//! golden matrices pass but which silently relays every row (i.e. regresses to
//! today's full-rebuild) is invisible.
//!
//! Spec: docs/superpowers/specs/2026-06-26-neomacs-incremental-layout-design.md
//! (§4.1 retained structure, §4.6 RowDamage, §5 Phase 0a, §7 go-criteria).

use crate::types::{LineWrapMode, WindowParams};
use neomacs_display_protocol::frame_glyphs::CursorStyle;
pub use neomacs_display_protocol::glyph_matrix::RowDamage;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphMatrix, GlyphRow};
use neomacs_display_protocol::types::Rect;
use neovm_core::buffer::position::LispCharPos1;
use neovm_core::window::{DisplayPointSnapshot, DisplayRowSnapshot};

/// How a window's layout was produced this cycle.
///
/// Phase 0a only ever produces [`LayoutClass::Full`]; the classifier that
/// yields the other variants arrives in Phases 1-3. Tracked per window so the
/// bench can assert, e.g., that an hl-line-on cursor move did NOT silently fall
/// back to `Full` (spec §5 Phase 1 gate).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LayoutClass {
    /// Full rebuild — every row relaid (today's only behavior).
    #[default]
    Full,
    /// Cursor moved; body reused verbatim, only cursor rows re-decorated (Phase 1).
    CursorOnly,
    /// Window scrolled by whole rows; rows shifted, newly-exposed laid (Phase 2).
    Scroll,
    /// Localized edit; only intersecting rows relaid (Phase 3).
    Edit,
}

/// Gate bit on a retained matrix.
///
/// A retained matrix may be reused only after a clean, fully-fontified,
/// non-probe pass set it [`MatrixValidity::Valid`]. Phase 0a sets it but never
/// reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MatrixValidity {
    /// Not safe to reuse (default, and after any escalation).
    #[default]
    Invalid,
    /// Produced by a clean, fully-fontified, accepted (non-probe) pass.
    Valid,
}

/// Snapshot of the layout inputs that, if changed, force a window's retained
/// matrix to be dropped (spec §4.2 window-level escalation list).
///
/// Phase 0a captures the geometry/window inputs that already exist on
/// [`WindowParams`]. Phase 0b/1 adds the neovm-core source-of-truth signals
/// (`chars`/`props`/`overlay` modified ticks + `face_change_count`). Any move
/// of these — or of geometry/window_start — escalates a window out of the
/// cursor-only fast path to a full rebuild ([`Self::cursor_only_eligible`]).
///
/// `PartialEq` is the reuse predicate: two keys are equal iff EVERY layout input
/// is identical. f32 fields compare bitwise-exactly — for an unchanged frame the
/// values are recomputed from identical inputs, so they are bit-identical; any
/// real geometry/metric change makes them differ and forces a full rebuild.
#[derive(Clone, Debug, PartialEq)]
pub struct RetainedWindowKey {
    pub buffer_id: u64,
    pub window_start: i64,
    pub point: i64,
    pub buffer_begv: i64,
    pub buffer_size: i64,
    pub text_bounds: Rect,
    pub bounds: Rect,
    pub hscroll: i32,
    pub vscroll: i32,
    pub wrap_mode: LineWrapMode,
    pub word_wrap: bool,
    pub tab_width: i32,
    pub char_width: f32,
    pub char_height: f32,
    pub font_pixel_size: f32,
    // --- Additional layout-affecting inputs (adversarial-review fixes). These
    // are read fresh from buffer-locals / window state each frame but do NOT
    // bump any of the four ticks below; a change must still force a full rebuild,
    // so they live in the key and the derived PartialEq compares them.
    /// Per-line tab stops (`tab-stop-list`); changes tab glyph widths/columns.
    pub tab_stop_list: Vec<i32>,
    /// `line-spacing` extra pixels per row; changes row height + window_end.
    pub extra_line_spacing: f32,
    /// `selective-display`; hides/shows lines past a column.
    pub selective_display: i32,
    /// Whether this is the frame's selected window; drives the solid-vs-hollow
    /// cursor, so a selection change must re-decorate (full rebuild).
    pub selected: bool,
    /// `show-trailing-whitespace` + the resolved background color.
    pub show_trailing_whitespace: bool,
    pub trailing_ws_bg: u32,
    /// `nobreak-char-display` + `glyphless-char-display` foreground; special-char
    /// rendering.
    pub nobreak_char_display: i32,
    pub glyphless_char_fg: u32,
    /// `indicate-empty-lines` + fringe widths (text-area offset / empty-line
    /// fringe glyphs baked into reused rows).
    pub indicate_empty_lines: i32,
    pub left_fringe_width: f32,
    pub right_fringe_width: f32,
    /// `line-prefix` / `wrap-prefix` (prepended glyphs on each line / wrap).
    pub line_prefix: Vec<u8>,
    pub wrap_prefix: Vec<u8>,
    /// Buffer multibyteness; char→byte mapping (window_end_byte) + char widths
    /// flip on `set-buffer-multibyte`.
    pub is_multibyte: bool,
    /// neovm-core per-buffer character-modification tick (spec §4.2 signal,
    /// chars). Any move means the buffer text changed → not cursor-only.
    pub chars_modified_tick: i64,
    /// Per-buffer text-property tick (face/display/invisible/composition; spec
    /// §4.2 (A)). Catches `put-text-property`, fontification, prettify-symbols.
    pub props_modified_tick: i64,
    /// Per-buffer overlay tick (spec §4.2 (B)). Catches hl-line, show-paren,
    /// region, flymake/lsp, iedit — any of which co-moves with the cursor.
    pub overlay_modified_tick: i64,
    /// Global face-subsystem change counter (spec §4.2 (C)). Catches
    /// `set-face-attribute` / theme load / face-remap that mutate pixels with
    /// no buffer tick.
    pub face_change_count: u64,
    /// Global display-variable change counter (adversarial-review fix). Bumped
    /// by `mark_redisplay_dirty_if_display_var` for the whole DISPLAY_AFFECTING
    /// set (truncate-lines, bidi-*, ctl-arrow, buffer-display-table /
    /// -invisibility-spec, fill-column-indicator, overlay-arrow,
    /// display-line-numbers, …) — none of which move a buffer/face tick.
    pub display_var_change_count: u64,
}

impl RetainedWindowKey {
    /// Snapshot the layout inputs from the resolved window params for this pass,
    /// reading the per-buffer invalidation ticks + global face counter from the
    /// evaluator. A missing buffer falls back to zero ticks (it will not match a
    /// real retained key, so it harmlessly forces a full rebuild).
    pub fn from_params(p: &WindowParams, evaluator: &neovm_core::emacs_core::Context) -> Self {
        // Read buffer_size FRESH (same accessor as WindowParams, neovm_bridge.rs)
        // alongside the ticks, so the key is internally consistent: if Lisp edits
        // the buffer mid-render (after window params were captured), the fresh
        // chars tick + fresh buffer_size both reflect it and the edit/full path
        // sees a correct delta (adversarial-review Phase A staleness fix).
        let (chars_modified_tick, props_modified_tick, overlay_modified_tick, is_multibyte, buffer_size) =
            evaluator
                .buffer_manager()
                .get(neovm_core::buffer::BufferId(p.buffer_id))
                .map(|buffer| {
                    (
                        buffer.chars_modified_tick(),
                        buffer.props_modified_tick(),
                        buffer.overlay_modified_tick(),
                        buffer.get_multibyte(),
                        buffer.point_max_char_pos().get() as i64,
                    )
                })
                .unwrap_or((0, 0, 0, false, p.buffer_size));
        Self {
            buffer_id: p.buffer_id,
            window_start: p.window_start,
            point: p.point,
            buffer_begv: p.buffer_begv,
            buffer_size,
            text_bounds: p.text_bounds,
            bounds: p.bounds,
            hscroll: p.hscroll,
            vscroll: p.vscroll,
            wrap_mode: p.wrap_mode,
            word_wrap: p.word_wrap,
            tab_width: p.tab_width,
            char_width: p.char_width,
            char_height: p.char_height,
            font_pixel_size: p.font_pixel_size,
            tab_stop_list: p.tab_stop_list.clone(),
            extra_line_spacing: p.extra_line_spacing,
            selective_display: p.selective_display,
            selected: p.selected,
            show_trailing_whitespace: p.show_trailing_whitespace,
            trailing_ws_bg: p.trailing_ws_bg,
            nobreak_char_display: p.nobreak_char_display,
            glyphless_char_fg: p.glyphless_char_fg,
            indicate_empty_lines: p.indicate_empty_lines,
            left_fringe_width: p.left_fringe_width,
            right_fringe_width: p.right_fringe_width,
            line_prefix: p.line_prefix.clone(),
            wrap_prefix: p.wrap_prefix.clone(),
            is_multibyte,
            chars_modified_tick,
            props_modified_tick,
            overlay_modified_tick,
            face_change_count: evaluator.face_change_count,
            display_var_change_count: evaluator.display_var_change_count,
        }
    }

    /// Whether a window may take the cursor-only fast path this frame: every
    /// layout input is identical to the retained (`prev`) key EXCEPT possibly
    /// `point`. This covers BOTH a cursor move (point differs, re-decorate the
    /// cursor) AND a no-change re-layout (point equal — e.g. a non-edited window
    /// in a multi-window frame, which must reuse its body verbatim instead of
    /// full-rebuilding; re-decorating the cursor at the same point is a no-op).
    /// Any tick/geometry/window_start move escalates to full (the buffer text,
    /// properties, overlays, faces, or the viewport changed, so the retained body
    /// rows are no longer trustworthy).
    pub fn cursor_only_eligible(prev: &Self, curr: &Self) -> bool {
        let mut aligned = prev.clone();
        aligned.point = curr.point;
        aligned == *curr
    }

    /// Whether a window may take the pure-scroll fast path: every layout input is
    /// identical to the retained (`prev`) key EXCEPT `window_start` (which moved)
    /// and `point` (which may follow the scroll). Any tick/geometry/face move
    /// escalates to a full rebuild. Whether the scroll is by WHOLE rows is decided
    /// separately against the retained matrix ([`RetainedWindowMatrix::scroll_replay`]).
    pub fn scroll_eligible(prev: &Self, curr: &Self) -> bool {
        if prev.window_start == curr.window_start {
            return false;
        }
        let mut aligned = prev.clone();
        aligned.window_start = curr.window_start;
        aligned.point = curr.point;
        aligned == *curr
    }

    /// Whether a window may take the localized-edit fast path: only the CHARS
    /// tick moved (a plain text edit) — props/overlay/face ticks, window_start,
    /// and geometry are all unchanged. A props/overlay/face move (e.g. font-lock
    /// re-fontifying the edited region) escalates to a full rebuild (the honest
    /// degradation, spec §4.4). `point` may also move with the edit.
    pub fn edit_eligible(prev: &Self, curr: &Self) -> bool {
        if prev.chars_modified_tick == curr.chars_modified_tick {
            return false;
        }
        let mut aligned = prev.clone();
        aligned.chars_modified_tick = curr.chars_modified_tick;
        aligned.point = curr.point;
        // A char edit necessarily changes the buffer size; that is the expected
        // consequence, not an escalation. `buffer_begv` is NOT aligned, so a
        // narrowing change (or an edit before BEGV) still escalates to Full.
        aligned.buffer_size = curr.buffer_size;
        aligned == *curr
    }
}

/// One window's retained layout, owned across cycles by `LayoutEngine`.
///
/// Committed at the accepted `break` only — never on a retry-loop `continue`
/// (mini/tab-bar resize) or a ≤1-row probe pass (the scroll-off hazard). Phase
/// 0a writes it; nothing reads it yet.
#[derive(Clone, Debug)]
pub struct RetainedWindowMatrix {
    /// The last clean pass's per-window matrix (the GNU "current matrix" analog).
    pub matrix: GlyphMatrix,
    /// Snapshot of every layout input for the reuse predicate.
    pub key: RetainedWindowKey,
    /// Reuse gate; only `Valid` matrices may be reused (Phases 1+).
    pub validity: MatrixValidity,
    /// Per-row provenance, parallel to `matrix.rows`. Phase 0a: all `New`.
    pub damage: Vec<RowDamage>,
    /// The clean pass's window display snapshot (point-independent body row
    /// snapshots + per-span display points). The cursor-only fast path (Phase 1)
    /// replays its body half verbatim, re-decorating only the cursor; the
    /// position fields are unchanged because the visible region did not move.
    pub display_snapshot: neovm_core::window::WindowDisplaySnapshot,
}

/// Everything the cursor-only fast path (Phase 1) needs to replay a window
/// without re-walking the buffer: the retained body rows to install verbatim,
/// the point-independent emitter body state to seed, and the old/new point so
/// the cursor can be cleared and re-decorated.
///
/// Built in the engine window loop from the previous frame's
/// [`RetainedWindowMatrix`] when [`RetainedWindowKey::cursor_only_eligible`]
/// holds and the cursor neighbourhood is structurally safe; consumed inside the
/// window render path in place of the body walk.
#[derive(Clone, Debug)]
pub struct CursorOnlyReplay {
    /// `(matrix_row_index, finalized GlyphRow)` for each retained NON-chrome
    /// row, installed verbatim (cursor decoration stripped, re-applied for the
    /// new point).
    pub body_rows: Vec<(usize, GlyphRow)>,
    /// Retained body `DisplayRowSnapshot`s (point-independent) to seed the
    /// emitter so `finish_and_install` rebuilds an identical window snapshot.
    pub body_row_snapshots: Vec<DisplayRowSnapshot>,
    /// Retained per-span display points (point-independent).
    pub points: Vec<DisplayPointSnapshot>,
    /// 0-based buffer position the cursor moved to this frame.
    pub new_point: i64,
    /// Matrix row index that now contains the new point (where the cursor is
    /// re-decorated).
    pub new_cursor_row_index: usize,
    /// Cursor style carried over from the retained pass.
    pub cursor_style: CursorStyle,
}

/// Reuse plan for the pure-scroll fast path (Phase 2): the overlapping retained
/// body rows shifted to their new positions, plus the span of newly-exposed rows
/// that still must be laid out. Built when the window scrolled by whole rows with
/// no text/appearance change.
#[derive(Clone, Debug)]
pub struct ScrollReplay {
    /// Uniform vertical shift applied to every reused row's `pixel_y` (negative =
    /// scrolled up / content moved up). The removed top rows' total height.
    pub dvpos: f32,
    /// `(new_matrix_row_index, shifted GlyphRow)` for each reused body row, with
    /// `pixel_y` already shifted by `dvpos` and cursor decoration stripped.
    pub reused_rows: Vec<(usize, GlyphRow)>,
    /// The reused rows' display-snapshot rows, re-indexed (`row -= s`) and
    /// y-shifted (`y += round(dvpos)`), to seed the emitter so `finish` rebuilds
    /// an identical window snapshot.
    pub reused_row_snapshots: Vec<DisplayRowSnapshot>,
    /// The reused rows' display points, re-indexed + y-shifted likewise.
    pub reused_points: Vec<DisplayPointSnapshot>,
    /// 0-based buffer charpos where the newly-exposed rows begin (the partial
    /// walk's start position).
    pub exposed_start_charpos: i64,
    /// Matrix row index of the first newly-exposed row.
    pub exposed_row_base: usize,
    /// Number of newly-exposed rows to lay out (= the whole-row scroll distance).
    pub exposed_row_count: usize,
    /// Window-relative y where the first newly-exposed row begins.
    pub exposed_text_y: f32,
    /// The real new window_start (where the reused region begins) — the partial
    /// walk reads from `exposed_start_charpos`, but the published redisplay
    /// positions + mode-line must use this.
    pub new_window_start: i64,
    /// 0-based point for this frame (the cursor is re-decorated as in Phase 1,
    /// since a scroll usually accompanies a point move).
    pub new_point: i64,
    /// Cursor style carried over from the retained pass.
    pub cursor_style: CursorStyle,
    /// Phase 3 below-reuse: when true, the partial walk is BOUNDED to
    /// `exposed_row_count` rows (the edited line only) — the rows below the edit
    /// are reused (charpos-shifted, same pixel_y) and are already included in
    /// `reused_rows`. When false (scroll, above-only edit) the walk runs to the
    /// window bottom as usual.
    pub bound_walk: bool,
}

impl RetainedWindowMatrix {
    /// True for chrome rows (mode/header/tab line, tab bar) — re-walked every
    /// frame, never reused. NOTE the discriminator is the row ROLE, not the
    /// `GlyphRow::mode_line` flag (which is set only for `ModeLine`, so header /
    /// tab lines have `mode_line == false`).
    pub fn is_chrome_role(role: neomacs_display_protocol::frame_glyphs::GlyphRowRole) -> bool {
        use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
        matches!(
            role,
            GlyphRowRole::ModeLine
                | GlyphRowRole::HeaderLine
                | GlyphRowRole::TabLine
                | GlyphRowRole::TabBar
        )
    }

    /// Build a [`CursorOnlyReplay`] for this window if it can be reused this
    /// frame with only the cursor re-decorated, else `None` (→ full rebuild).
    ///
    /// Bails when the matrix is invalid, the reuse predicate fails
    /// ([`RetainedWindowKey::cursor_only_eligible`]), the new point lands outside
    /// the retained body rows, or the new cursor row is structurally unsafe
    /// (continuation / left-truncation / overlay-arrow fringe — the column
    /// resolve cannot place the cursor on those correctly).
    pub fn cursor_only_replay(&self, curr: &RetainedWindowKey) -> Option<CursorOnlyReplay> {
        if self.validity != MatrixValidity::Valid {
            return None;
        }
        if !RetainedWindowKey::cursor_only_eligible(&self.key, curr) {
            return None;
        }
        let new_point = curr.point;
        let mut body_rows: Vec<(usize, GlyphRow)> = Vec::new();
        let mut body_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut cursor_style: Option<CursorStyle> = None;
        let mut new_cursor: Option<(usize, &GlyphRow)> = None;
        for (idx, row) in self.matrix.rows.iter().enumerate() {
            if !row.enabled || Self::is_chrome_role(row.role) {
                continue;
            }
            if row.cursor_type.is_some() {
                cursor_style = row.cursor_type;
            }
            let start = row.start_charpos as i64;
            let end = row.end_charpos as i64;
            if new_cursor.is_none() && start <= new_point && new_point <= end {
                new_cursor = Some((idx, row));
            }
            body_indices.insert(idx);
            body_rows.push((idx, row.clone()));
        }
        if body_rows.is_empty() {
            return None;
        }
        let (new_cursor_row_index, cursor_row) = new_cursor?;
        if cursor_row.continued || cursor_row.truncated_left || cursor_row.left_fringe_bitmap.is_some()
        {
            return None;
        }
        // Scroll-safety (GNU `try_cursor_movement` / `make_cursor_line_fully_visible`):
        // a point move onto the top or bottom visible row can trigger a window
        // scroll to keep point visible, which the cursor-only path does NOT
        // perform (it never re-derives window_start). Bail on a boundary row
        // unless the window is already pinned to that buffer edge:
        //   * bottom row with more buffer below (`!ends_at_zv`) → may scroll down,
        //   * top row while the window is scrolled off the buffer start → may scroll up.
        let first_body_index = body_rows.first().map(|(idx, _)| *idx);
        let last_body_index = body_rows.last().map(|(idx, _)| *idx);
        let window_at_buffer_top = curr.window_start <= curr.buffer_begv + 1;
        if Some(new_cursor_row_index) == last_body_index && !cursor_row.ends_at_zv {
            return None;
        }
        if Some(new_cursor_row_index) == first_body_index && !window_at_buffer_top {
            return None;
        }
        let body_row_snapshots = self
            .display_snapshot
            .rows
            .iter()
            .filter(|snapshot| body_indices.contains(&(snapshot.row as usize)))
            .cloned()
            .collect();
        Some(CursorOnlyReplay {
            body_rows,
            body_row_snapshots,
            points: self.display_snapshot.points.clone(),
            new_point,
            new_cursor_row_index,
            cursor_style: cursor_style.unwrap_or(CursorStyle::FilledBox),
        })
    }

    /// Build a [`ScrollReplay`] if this window scrolled by WHOLE rows with no
    /// text/appearance change, else `None` (→ full rebuild). Handles forward
    /// (downward) scroll — the new `window_start` lands on a retained row
    /// boundary `s` rows down; rows `[s..]` are reused (shifted up by `dvpos`)
    /// and `s` newly-exposed rows remain to be laid at the bottom. Scroll-up
    /// (new start above the retained top), partial-row scroll, vscroll, line
    /// numbers, and continuation/truncation rows all bail (conservative).
    pub fn scroll_replay(&self, curr: &RetainedWindowKey) -> Option<ScrollReplay> {
        if self.validity != MatrixValidity::Valid {
            return None;
        }
        if !RetainedWindowKey::scroll_eligible(&self.key, curr) || curr.vscroll != 0 {
            return None;
        }
        // Collect body rows in matrix order; bail on anything that the uniform
        // shift cannot reproduce (line numbers renumber; continuation/truncation
        // /fringe rows have position-dependent decoration).
        let mut body: Vec<(usize, &GlyphRow)> = Vec::new();
        for (idx, row) in self.matrix.rows.iter().enumerate() {
            if !row.enabled || Self::is_chrome_role(row.role) {
                continue;
            }
            if !row.glyphs[GlyphArea::LeftMargin.index()].is_empty()
                || row.continued
                || row.truncated_left
                || row.left_fringe_bitmap.is_some()
            {
                return None;
            }
            body.push((idx, row));
        }
        if body.len() < 2 {
            return None;
        }
        // Whole-row scroll distance: the body row whose start matches the new
        // window_start. `None` (no match) → partial-row scroll or scroll-up → bail.
        let s = body
            .iter()
            .position(|(_, row)| row.start_charpos as i64 == curr.window_start)?;
        if s == 0 {
            return None;
        }
        let cursor_style = body
            .iter()
            .find_map(|(_, row)| row.cursor_type)
            .unwrap_or(CursorStyle::FilledBox);
        let last = body.len() - 1;
        let dvpos = body[0].1.pixel_y - body[s].1.pixel_y;
        let dvpos_i64 = dvpos.round() as i64;
        // old matrix row index → new matrix row index for each reused row.
        let mut remap: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
        let mut reused_rows = Vec::with_capacity(last - s + 1);
        for p in s..=last {
            let mut shifted = body[p].1.clone();
            shifted.pixel_y += dvpos;
            shifted.cursor_col = None;
            shifted.cursor_type = None;
            remap.insert(body[p].0 as i64, body[p - s].0 as i64);
            reused_rows.push((body[p - s].0, shifted));
        }
        let reused_row_snapshots = self
            .display_snapshot
            .rows
            .iter()
            .filter_map(|row| {
                remap.get(&row.row).map(|&new_row| {
                    let mut row = row.clone();
                    row.row = new_row;
                    row.y += dvpos_i64;
                    row
                })
            })
            .collect();
        let reused_points = self
            .display_snapshot
            .points
            .iter()
            .filter_map(|point| {
                remap.get(&point.row).map(|&new_row| {
                    let mut point = point.clone();
                    point.row = new_row;
                    point.y += dvpos_i64;
                    point
                })
            })
            .collect();
        let last_row = body[last].1;
        Some(ScrollReplay {
            dvpos,
            reused_rows,
            reused_row_snapshots,
            reused_points,
            exposed_start_charpos: last_row.end_charpos as i64 + 1,
            exposed_row_base: body[last - s + 1].0,
            exposed_row_count: s,
            exposed_text_y: last_row.pixel_y + dvpos + last_row.height_px,
            new_window_start: curr.window_start,
            new_point: curr.point,
            cursor_style,
            bound_walk: false,
        })
    }

    /// Build an edit replay (Phase 3 `try_window_id` analog) reusing the rows
    /// ABOVE the dirty span verbatim and partial-walking the dirty line + every
    /// row below it. `dirty_start` is the buffer's accumulated `changed_char_range`
    /// start. Reuses the [`ScrollReplay`] shape with `dvpos = 0` (no scroll, so
    /// the reused rows keep their position). Returns `None` (→ full rebuild) when
    /// the edit touches the very top visible row (nothing above to reuse) or the
    /// reuse predicate fails. The reused rows' charpos is valid because the edit
    /// is strictly below them.
    pub fn edit_replay(
        &self,
        curr: &RetainedWindowKey,
        dirty_start: i64,
        allow_below_reuse: bool,
    ) -> Option<ScrollReplay> {
        if self.validity != MatrixValidity::Valid {
            return None;
        }
        if !RetainedWindowKey::edit_eligible(&self.key, curr) || curr.vscroll != 0 {
            return None;
        }
        let mut body: Vec<(usize, &GlyphRow)> = Vec::new();
        for (idx, row) in self.matrix.rows.iter().enumerate() {
            if !row.enabled || Self::is_chrome_role(row.role) {
                continue;
            }
            if !row.glyphs[GlyphArea::LeftMargin.index()].is_empty()
                || row.continued
                || row.truncated_left
                || row.left_fringe_bitmap.is_some()
            {
                return None;
            }
            body.push((idx, row));
        }
        if body.is_empty() {
            return None;
        }
        // First dirty row = first body row whose OLD extent reaches `dirty_start`.
        // Rows above it end before the edit, so their charpos is unchanged.
        let first_dirty = body
            .iter()
            .position(|(_, row)| row.end_charpos as i64 >= dirty_start)?;
        if first_dirty == 0 {
            return None;
        }
        let cursor_style = body
            .iter()
            .find_map(|(_, row)| row.cursor_type)
            .unwrap_or(CursorStyle::FilledBox);
        let mut reused_rows = Vec::with_capacity(first_dirty);
        let mut above_indices: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for &(idx, row) in body.iter().take(first_dirty) {
            let mut row = row.clone();
            row.cursor_col = None;
            row.cursor_type = None;
            above_indices.insert(idx as i64);
            reused_rows.push((idx, row));
        }
        let mut reused_row_snapshots: Vec<DisplayRowSnapshot> = self
            .display_snapshot
            .rows
            .iter()
            .filter(|row| above_indices.contains(&row.row))
            .cloned()
            .collect();
        let mut reused_points: Vec<DisplayPointSnapshot> = self
            .display_snapshot
            .points
            .iter()
            .filter(|point| above_indices.contains(&point.row))
            .cloned()
            .collect();
        let dirty_row = body[first_dirty].1;

        // BELOW-REUSE (full try_window_id, post-walk-validated). When this is a
        // simple insert into a monospace edited row that has rows below it, build
        // the plan OPTIMISTICALLY: reuse the rows below the edit too (content
        // unchanged, charpos shifted by the inserted count, same pixel_y) and
        // BOUND the walk to the edited line. The render validates post-walk (the
        // walked edited row must stay one non-continued row ending exactly where
        // the first reused-below row begins); on failure it bails to above-only.
        // Gated by `allow_below_reuse` until the render integration lands.
        if allow_below_reuse {
            let delta = curr.buffer_size - self.key.buffer_size;
            let text_glyphs = &dirty_row.glyphs[GlyphArea::Text.index()];
            let monospace = !text_glyphs.is_empty()
                && text_glyphs
                    .iter()
                    .all(|g| (g.pixel_width - curr.char_width).abs() < 0.5);
            // The edited line must PROVABLY still fit in one row after the insert
            // (no wrap → the rows below keep their pixel_y). `allow_below_reuse`
            // already guarantees the inserted chars are simple/char_width (the
            // caller's ASCII check), so `(cols + delta) * char_width` is exact.
            let stays_one_row = (text_glyphs.len() as f32 + delta as f32) * curr.char_width
                <= curr.text_bounds.width;
            if delta > 0 && first_dirty + 1 < body.len() && monospace && stays_one_row {
                let shift = |p: LispCharPos1| {
                    LispCharPos1::from_one_based_usize(
                        (p.to_one_based_usize() as i64 + delta) as usize,
                    )
                };
                let mut below_indices: std::collections::HashSet<i64> =
                    std::collections::HashSet::new();
                for &(idx, row) in body.iter().skip(first_dirty + 1) {
                    let mut row = row.clone();
                    row.cursor_col = None;
                    row.cursor_type = None;
                    row.start_charpos = (row.start_charpos as i64 + delta) as usize;
                    row.end_charpos = (row.end_charpos as i64 + delta) as usize;
                    for area in row.glyphs.iter_mut() {
                        for g in area.iter_mut() {
                            g.charpos = (g.charpos as i64 + delta) as usize;
                        }
                    }
                    below_indices.insert(idx as i64);
                    reused_rows.push((idx, row));
                }
                for snap in self
                    .display_snapshot
                    .rows
                    .iter()
                    .filter(|r| below_indices.contains(&r.row))
                {
                    let mut snap = snap.clone();
                    snap.start_buffer_pos = snap.start_buffer_pos.map(shift);
                    snap.end_buffer_pos = snap.end_buffer_pos.map(shift);
                    reused_row_snapshots.push(snap);
                }
                for point in self
                    .display_snapshot
                    .points
                    .iter()
                    .filter(|p| below_indices.contains(&p.row))
                {
                    let mut point = point.clone();
                    point.buffer_pos = shift(point.buffer_pos);
                    reused_points.push(point);
                }
                return Some(ScrollReplay {
                    dvpos: 0.0,
                    reused_rows,
                    reused_row_snapshots,
                    reused_points,
                    exposed_start_charpos: dirty_row.start_charpos as i64,
                    exposed_row_base: body[first_dirty].0,
                    // Bound the walk to the edited line (one row — no body row is
                    // continued, so every line occupies exactly one row).
                    exposed_row_count: 1,
                    exposed_text_y: dirty_row.pixel_y,
                    new_window_start: curr.window_start,
                    new_point: curr.point,
                    cursor_style,
                    bound_walk: true,
                });
            }
        }

        Some(ScrollReplay {
            dvpos: 0.0,
            reused_rows,
            reused_row_snapshots,
            reused_points,
            exposed_start_charpos: dirty_row.start_charpos as i64,
            exposed_row_base: body[first_dirty].0,
            exposed_row_count: body.len() - first_dirty,
            exposed_text_y: dirty_row.pixel_y,
            new_window_start: curr.window_start,
            new_point: curr.point,
            cursor_style,
            bound_walk: false,
        })
    }
}

/// Per-frame layout instrumentation — THE gate metric for every phase.
///
/// Reset at the top of each `layout_frame_rust`, populated as the frame is
/// committed, and read by the bench harness. Each phase ships ONLY when its
/// bench cases prove the win on relaid-row-count, not wall-time alone (spec §7).
#[derive(Clone, Debug, Default)]
pub struct LayoutStats {
    /// Buffer-text ("body") rows laid out from scratch this frame. THE
    /// rank-1 gate metric: Phase 1 cursor-only must drive this to 0.
    pub relaid_body_rows: usize,
    /// Chrome rows (mode/header/tab line) laid out this frame. Chrome is always
    /// re-walked (spec §4.2), so this stays nonzero even on the fast paths.
    pub relaid_chrome_rows: usize,
    /// Rows reused verbatim from the retained matrix. Phase 0a: 0.
    pub reused_rows: usize,
    /// Rows reused with a uniform vertical shift. Phase 0a: 0.
    pub reused_shifted_rows: usize,
    /// Windows classified `Full` this frame. Phase 0a: all of them.
    pub full_windows: usize,
    /// Windows that took the cursor-only fast path (Phase 1).
    pub cursor_only_windows: usize,
    /// Windows that took the pure-scroll fast path (Phase 2).
    pub scroll_windows: usize,
    /// Windows that took the localized-edit fast path (Phase 3).
    pub edit_windows: usize,
    /// Wall-time spent evaluating the reuse predicate (Phase 0a: ~0). Tracked
    /// because the predicate could approach relayout cost for screenfuls of
    /// short rows when the dirty set is small (spec §6).
    pub reuse_predicate_cpu: std::time::Duration,
}

impl LayoutStats {
    /// Total rows laid out from scratch this frame (body + chrome).
    pub fn relaid_rows(&self) -> usize {
        self.relaid_body_rows + self.relaid_chrome_rows
    }

    /// Total windows laid out this frame, across all classifications.
    pub fn total_windows(&self) -> usize {
        self.full_windows + self.cursor_only_windows + self.scroll_windows + self.edit_windows
    }

    /// Bump the per-class window counter for one laid-out window.
    pub fn record_window_class(&mut self, class: LayoutClass) {
        match class {
            LayoutClass::Full => self.full_windows += 1,
            LayoutClass::CursorOnly => self.cursor_only_windows += 1,
            LayoutClass::Scroll => self.scroll_windows += 1,
            LayoutClass::Edit => self.edit_windows += 1,
        }
    }
}

#[cfg(test)]
mod scroll_classifier_tests {
    use super::*;
    use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
    use neovm_core::window::{WindowDisplaySnapshot, WindowId};

    fn synthetic_key(window_start: i64, point: i64) -> RetainedWindowKey {
        RetainedWindowKey {
            buffer_id: 1,
            window_start,
            point,
            buffer_begv: 0,
            buffer_size: 1000,
            text_bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
            bounds: Rect::new(0.0, 0.0, 800.0, 616.0),
            hscroll: 0,
            vscroll: 0,
            wrap_mode: LineWrapMode::Truncate,
            word_wrap: false,
            tab_width: 8,
            char_width: 8.0,
            char_height: 16.0,
            font_pixel_size: 16.0,
            tab_stop_list: Vec::new(),
            extra_line_spacing: 0.0,
            selective_display: 0,
            selected: true,
            show_trailing_whitespace: false,
            trailing_ws_bg: 0,
            nobreak_char_display: 0,
            glyphless_char_fg: 0,
            indicate_empty_lines: 0,
            left_fringe_width: 0.0,
            right_fringe_width: 0.0,
            line_prefix: Vec::new(),
            wrap_prefix: Vec::new(),
            is_multibyte: true,
            chars_modified_tick: 5,
            props_modified_tick: 5,
            overlay_modified_tick: 5,
            face_change_count: 5,
            display_var_change_count: 5,
        }
    }

    /// A retained matrix with `n_body` 16px Text rows (10 chars each, the first
    /// starting at `base`) plus a mode-line row.
    fn synthetic_matrix(base: i64, n_body: usize) -> RetainedWindowMatrix {
        let mut matrix = GlyphMatrix::new(n_body + 1, 100);
        for i in 0..n_body {
            let row = &mut matrix.rows[i];
            row.enabled = true;
            row.role = GlyphRowRole::Text;
            row.displays_text = true;
            row.start_charpos = (base + (i as i64) * 10) as usize;
            row.end_charpos = (base + (i as i64) * 10 + 9) as usize;
            row.pixel_y = (i as f32) * 16.0;
            row.height_px = 16.0;
            row.ascent_px = 16.0;
        }
        let ml = &mut matrix.rows[n_body];
        ml.enabled = true;
        ml.role = GlyphRowRole::ModeLine;
        ml.mode_line = true;
        ml.pixel_y = (n_body as f32) * 16.0;
        ml.height_px = 16.0;
        RetainedWindowMatrix {
            matrix,
            key: synthetic_key(base, 0),
            validity: MatrixValidity::Valid,
            damage: Vec::new(),
            display_snapshot: WindowDisplaySnapshot {
                window_id: WindowId(1),
                text_area_left_offset: 0,
                mode_line_height: 16,
                header_line_height: 0,
                tab_line_height: 0,
                logical_cursor: None,
                phys_cursor: None,
                points: Vec::new(),
                rows: Vec::new(),
            },
        }
    }

    #[test]
    fn scroll_replay_detects_whole_row_scroll_down() {
        let m = synthetic_matrix(0, 5); // rows start at 0,10,20,30,40
        let curr = synthetic_key(20, 25); // scrolled to row 2, point followed
        let r = m.scroll_replay(&curr).expect("whole-row scroll-down is eligible");
        assert_eq!(r.dvpos, -32.0, "removed two 16px rows");
        // Rows [2,3,4] reused into matrix indices [0,1,2] with shifted pixel_y.
        assert_eq!(
            r.reused_rows.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(r.reused_rows[0].1.pixel_y, 0.0);
        assert_eq!(r.reused_rows[2].1.pixel_y, 32.0);
        assert_eq!(r.exposed_row_count, 2, "two newly-exposed rows at the bottom");
        assert_eq!(r.exposed_row_base, 3);
        assert_eq!(r.exposed_start_charpos, 50); // after row 4 (chars 40..49)
        assert_eq!(r.exposed_text_y, 48.0); // 3rd visual row top
        assert_eq!(r.new_point, 25);
    }

    #[test]
    fn scroll_replay_bails_on_partial_row_scroll() {
        let m = synthetic_matrix(0, 5);
        let curr = synthetic_key(15, 15); // not a row boundary
        assert!(m.scroll_replay(&curr).is_none());
    }

    #[test]
    fn scroll_replay_bails_on_scroll_up() {
        let m = synthetic_matrix(20, 5); // rows start at 20,30,40,50,60
        let curr = synthetic_key(0, 5); // above the retained top
        assert!(m.scroll_replay(&curr).is_none());
    }

    #[test]
    fn scroll_replay_bails_on_tick_change() {
        let m = synthetic_matrix(0, 5);
        let mut curr = synthetic_key(20, 25);
        curr.props_modified_tick += 1; // a text-property write co-occurred
        assert!(m.scroll_replay(&curr).is_none());
    }

    #[test]
    fn scroll_replay_bails_on_vscroll() {
        let m = synthetic_matrix(0, 5);
        let mut curr = synthetic_key(20, 25);
        curr.vscroll = 1; // pixel-level scroll offset
        assert!(m.scroll_replay(&curr).is_none());
    }

    #[test]
    fn scroll_replay_bails_when_unchanged() {
        let m = synthetic_matrix(0, 5);
        let curr = synthetic_key(0, 0); // window_start did not move
        assert!(m.scroll_replay(&curr).is_none());
    }

    /// Adversarial-review fix: each layout-affecting input that does NOT bump a
    /// modification tick (tab-stop-list, line-spacing, selective-display, window
    /// selection, trailing-whitespace, special-char display, fringes, line/wrap
    /// prefixes, multibyteness) must, when changed alone, force a FULL rebuild —
    /// otherwise the fast paths would reuse rows shaped under the old setting.
    #[test]
    fn fast_paths_bail_when_a_non_tick_layout_param_changes() {
        let prev = synthetic_key(0, 0);
        // Baseline: a pure point move IS cursor-only.
        assert!(RetainedWindowKey::cursor_only_eligible(
            &prev,
            &synthetic_key(0, 7)
        ));

        let mutations: &[(&str, fn(&mut RetainedWindowKey))] = &[
            ("tab_stop_list", |k| k.tab_stop_list = vec![4, 12]),
            ("extra_line_spacing", |k| k.extra_line_spacing = 2.0),
            ("selective_display", |k| k.selective_display = 4),
            ("selected", |k| k.selected = false),
            ("show_trailing_whitespace", |k| {
                k.show_trailing_whitespace = true
            }),
            ("trailing_ws_bg", |k| k.trailing_ws_bg = 0x00ff_00ff),
            ("nobreak_char_display", |k| k.nobreak_char_display = 1),
            ("glyphless_char_fg", |k| k.glyphless_char_fg = 0x00ff_ffff),
            ("indicate_empty_lines", |k| k.indicate_empty_lines = 1),
            ("left_fringe_width", |k| k.left_fringe_width = 8.0),
            ("right_fringe_width", |k| k.right_fringe_width = 8.0),
            ("line_prefix", |k| k.line_prefix = vec![b'>', b' ']),
            ("wrap_prefix", |k| k.wrap_prefix = vec![b' ', b' ']),
            ("is_multibyte", |k| k.is_multibyte = false),
        ];
        for (name, mutate) in mutations {
            // Cursor-only (point also moved): must bail.
            let mut curr = synthetic_key(0, 7);
            mutate(&mut curr);
            assert!(
                !RetainedWindowKey::cursor_only_eligible(&prev, &curr),
                "{name} change must block the cursor-only fast path"
            );
            // Scroll (window_start also moved): must bail.
            let mut scrolled = synthetic_key(20, 7);
            mutate(&mut scrolled);
            assert!(
                !RetainedWindowKey::scroll_eligible(&prev, &scrolled),
                "{name} change must block the scroll fast path"
            );
            // Edit (chars tick + buffer_size also moved): must bail.
            let mut edited = synthetic_key(0, 7);
            edited.chars_modified_tick = prev.chars_modified_tick + 1;
            edited.buffer_size = prev.buffer_size + 1;
            mutate(&mut edited);
            assert!(
                !RetainedWindowKey::edit_eligible(&prev, &edited),
                "{name} change must block the edit fast path"
            );
        }
    }

    /// Phase 3 below-reuse classifier: a simple insert into a monospace edited
    /// row reuses the rows BELOW the edit (charpos shifted by the inserted count,
    /// pixel_y unchanged) and bounds the walk to the edited line. Gated on
    /// `allow_below_reuse`; with it off the plan is the above-only edit replay.
    #[test]
    fn edit_replay_below_reuse_shifts_rows_below_by_inserted_count() {
        use neomacs_display_protocol::glyph_matrix::Glyph;
        let mut m = synthetic_matrix(0, 5); // rows start at 0,10,20,30,40
        // Give the edited row (row 2, chars [20,29]) 10 monospace (8px) glyphs.
        for c in 0..10 {
            let mut g = Glyph::char('a', 0, (20 + c) as usize);
            g.pixel_width = 8.0;
            m.matrix.rows[2].glyphs[GlyphArea::Text.index()].push(g);
        }
        // A 1-char insert at charpos 25 (inside row 2): chars tick moved, point +
        // buffer_size grew by 1, everything else equal.
        let mut curr = synthetic_key(0, 25);
        curr.chars_modified_tick = 6;
        curr.buffer_size = 1001;

        // allow_below_reuse = true → reuse above (0,1) AND below (3,4); below rows
        // are charpos-shifted by +1; the walk is bounded to the one edited row.
        let r = m
            .edit_replay(&curr, 25, true)
            .expect("below-reuse is eligible");
        assert!(r.bound_walk, "walk bounded to the edited line");
        assert_eq!(r.exposed_row_count, 1, "only the edited line is walked");
        assert_eq!(r.exposed_row_base, 2);
        assert_eq!(
            r.reused_rows.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![0, 1, 3, 4],
            "above (0,1) reused verbatim + below (3,4) reused shifted"
        );
        let below3 = r.reused_rows.iter().find(|(i, _)| *i == 3).unwrap();
        assert_eq!(below3.1.start_charpos, 31, "row 3 start 30 -> 31");
        assert_eq!(below3.1.end_charpos, 40, "row 3 end 39 -> 40");
        let below4 = r.reused_rows.iter().find(|(i, _)| *i == 4).unwrap();
        assert_eq!(below4.1.start_charpos, 41, "row 4 start 40 -> 41");

        // allow_below_reuse = false → the above-only edit replay (no below reuse,
        // walk runs to the bottom).
        let above_only = m
            .edit_replay(&curr, 25, false)
            .expect("above-only edit replay");
        assert!(!above_only.bound_walk);
        assert_eq!(
            above_only
                .reused_rows
                .iter()
                .map(|(i, _)| *i)
                .collect::<Vec<_>>(),
            vec![0, 1],
            "only the rows above the edit are reused"
        );
        assert_eq!(above_only.exposed_row_count, 3, "edited line + 2 rows below");
    }
}
