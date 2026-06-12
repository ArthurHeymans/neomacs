//! The Rust layout engine — Phase 1+2: Monospace layout with face resolution.
//!
//! Reads buffer text and display state from neovm-core, resolves faces per
//! character position, computes line breaks, positions glyphs on a fixed-width
//! grid, and publishes `FrameDisplayState` snapshots for render backends.

use super::display_space::{DisplaySpaceKey, display_space_positive_number};
use super::font_metrics::FontMetricsService;
use super::gui_chrome::{collect_gui_menu_bar_items_for_frame, collect_gui_tool_bar_items};
use super::hit_test::*;
use super::types::*;
use super::unicode::*;
use super::window_output::{
    ChromeRowOutput, RowMetricsSnapshot, TextMatrixRowOutput, WindowOutputEmitter,
};
use crate::coords::{layout_i64_char_pos_to_lisp_char_pos, lisp_char_pos_to_layout_i64};
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_face_policy::BaseFacePolicy;
use crate::display_origin::{DisplayOrigin, DisplayPropertySource, OverlayStringKind};
use crate::display_property::{DisplayReplacementProperty, classify_display_property};
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowBoundsPolicy, DisplayRowFace, DisplayRowFallbackMetrics,
    DisplayRowGeometry, DisplayRowMeasurementPolicy, DisplayRowOutputProgress, DisplayRowOwner,
    DisplayRowRenderBounds, DisplayRowRenderStop, DisplayRowRenderer, DisplayRowSourceState,
    DisplayRowSpec, FrameChromeKind, MeasuredDisplayRow, RenderedDisplayRow, WindowChromeKind,
    insert_resolved_display_row_face, install_measured_frame_chrome_row,
    install_rendered_display_row,
};
use crate::display_row_append::{
    DisplayRowAppendArea, DisplayRowAppendMetrics, DisplayRowAppendSurface,
    append_buffer_text_fragment_to_text_row, append_buffer_text_item_fragment_to_text_row_and_emit,
    append_display_replacement_item_to_text_row_and_emit,
    append_display_replacement_string_source_to_text_row,
    append_lisp_string_fragment_to_text_row_and_emit, append_synthetic_text_to_display_row,
    render_natural_display_item_source_into_current_text_row_and_emit,
};
use crate::display_row_builder::{
    DisplayRowItemMeasurement, DisplayRowItemMeasurer, DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::{
    DisplayRowBoundaryTarget, DisplayRowGeometryDefaults, DisplayRowGeometryState,
    DisplayRowHitRange, DisplayRowVisibilityLimit, DisplayRowYFallback, DisplayRowYPositions,
    DisplayRowYRecording, LegacyDisplayRowGeometryVars,
};
use crate::display_source::{
    BufferDisplayReplacementSource, BufferDisplayReplacementStringSource, DisplayReplacementBox,
};
use crate::display_source_resolver::resolve_display_property_media;
use crate::display_text::{DisplayTextFragment, DisplayTextStorage};
use crate::display_text_run_measurement::DisplayTextRunByteAdvance;
use crate::fontconfig::FontSizing;
use crate::neovm_bridge::LayoutBufferView;
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, FrameGlyphBuffer, GlyphRowRole, PhysCursor, WindowEffectHint,
    WindowInfo, WindowTransitionHint, WindowTransitionKind,
};
use neomacs_display_protocol::glyph_matrix::ScrollBarItem;
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, EmacsByteRange, LispCharPos1};
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::keymap::{KeymapMarker, is_list_keymap};
use neovm_core::emacs_core::value::{get_string_text_properties_table_for_value, list_to_vec};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::window::{
    DisplayPointSnapshot, DisplayRowSnapshot, WindowCursorKind, WindowCursorPos,
    WindowCursorSnapshot, WindowDisplaySnapshot, WindowId,
};
use strum::{EnumString, IntoStaticStr};

/// Maximum number of characters in a ligature run before forced flush.
const MAX_LIGATURE_RUN_LEN: usize = 64;
/// Bound redisplay convergence work when point begins outside the visible span.
const MAX_WINDOW_VISIBILITY_RETRIES: usize = 128;
const SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS: u64 = 3;
const SYNTHETIC_SOURCE_HSCROLL_TRUNCATION: u64 = 4;
const SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS: u64 = 5;

#[derive(Clone, Copy, Debug)]
struct ScrollBarMetrics {
    position: i64,
    portion: i64,
    whole: i64,
    thumb_start: f32,
    thumb_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
enum ResizeMiniWindowsMode {
    #[strum(to_string = "nil")]
    Disabled,
    #[strum(to_string = "grow-only")]
    GrowOnly,
    #[strum(to_string = "t")]
    Exact,
}

impl ResizeMiniWindowsMode {
    fn from_lisp_value(value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return Self::Exact;
        };
        if value.is_nil() {
            return Self::Disabled;
        }
        value
            .as_symbol_name()
            .and_then(|name| name.parse().ok())
            .unwrap_or(Self::Exact)
    }

    fn should_grow(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    fn should_shrink(self, visible_region_empty: bool) -> bool {
        match self {
            Self::Disabled => false,
            Self::GrowOnly => visible_region_empty,
            Self::Exact => true,
        }
    }
}

/// Buffer for accumulating same-face text runs for ligature shaping.
struct LigatureRunBuffer {
    chars: Vec<char>,
    advances: Vec<f32>,
    start_x: f32,
    start_y: f32,
    face_h: f32,
    face_ascent: f32,
    face_id: u32,
    total_advance: f32,
    is_overlay: bool,
}

#[derive(Clone, Copy, Debug)]
struct CapturedCursorInfo {
    x: f32,
    y: f32,
    face_w: f32,
    face_h: f32,
    face_ascent: f32,
    bg: Color,
    byte_idx: usize,
    col: usize,
    matrix_row: usize,
    slot_width: Option<f32>,
    stretch_like: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CapturedCursorSlotWidth {
    FaceChar,
    Explicit(f32),
}

impl CapturedCursorSlotWidth {
    fn resolve(self, face_char_width: f32) -> f32 {
        match self {
            Self::FaceChar => face_char_width,
            Self::Explicit(width) => width,
        }
        .max(1.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct CapturedCursorPlacement {
    x: f32,
    y: f32,
    byte_idx: usize,
    col: usize,
    matrix_row: usize,
    slot_width: CapturedCursorSlotWidth,
    stretch_like: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CapturedCursorVisualState {
    face_width: f32,
    face_height: f32,
    face_ascent: f32,
    background: Color,
}

impl CapturedCursorVisualState {
    fn from_active_face_state(active_face_state: &DisplayRowActiveFaceState) -> Self {
        let metrics = active_face_state.metrics();
        Self {
            face_width: metrics.char_width,
            face_height: metrics.row_height,
            face_ascent: metrics.ascent,
            background: active_face_state.background(),
        }
    }

    fn display_box_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        face_height: f32,
        face_ascent: f32,
    ) -> Self {
        let metrics = active_face_state.metrics();
        Self {
            face_width: metrics.char_width,
            face_height,
            face_ascent,
            background: active_face_state.background(),
        }
    }

    fn line_break_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        line_height: f32,
    ) -> Self {
        let metrics = active_face_state.metrics();
        Self::display_box_from_active_face_state(active_face_state, line_height, metrics.ascent)
    }
}

impl CapturedCursorInfo {
    fn logical_cursor_position(
        &self,
        row_metric: RowMetricsSnapshot,
        text_matrix_row_base: usize,
        text_area_left: f32,
        window_top: f32,
    ) -> WindowCursorPos {
        WindowCursorPos {
            x: (self.x - text_area_left).round() as i64,
            y: (row_metric.pixel_y - window_top).round() as i64,
            row: text_matrix_row_base as i64 + self.matrix_row as i64,
            col: self.col as i64,
        }
    }

    fn resolved_slot_width(&self, style: CursorStyle, text: &[u8], params: &WindowParams) -> f32 {
        if let Some(slot_width) = self.slot_width {
            slot_width.max(1.0)
        } else {
            cursor_width_for_style(
                style,
                text,
                self.byte_idx,
                self.col as i32,
                params,
                self.face_w,
            )
            .max(1.0)
        }
    }

    fn from_visual_state(
        visual_state: CapturedCursorVisualState,
        placement: CapturedCursorPlacement,
    ) -> Self {
        Self {
            x: placement.x,
            y: placement.y,
            face_w: visual_state.face_width,
            face_h: visual_state.face_height,
            face_ascent: visual_state.face_ascent,
            bg: visual_state.background,
            byte_idx: placement.byte_idx,
            col: placement.col,
            matrix_row: placement.matrix_row,
            slot_width: Some(placement.slot_width.resolve(visual_state.face_width)),
            stretch_like: placement.stretch_like,
        }
    }

    fn from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        placement: CapturedCursorPlacement,
    ) -> Self {
        Self::from_visual_state(
            CapturedCursorVisualState::from_active_face_state(active_face_state),
            placement,
        )
    }

    fn display_box_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        placement: CapturedCursorPlacement,
        face_height: f32,
        face_ascent: f32,
    ) -> Self {
        Self::from_visual_state(
            CapturedCursorVisualState::display_box_from_active_face_state(
                active_face_state,
                face_height,
                face_ascent,
            ),
            placement,
        )
    }

    fn line_break_from_active_face_state(
        active_face_state: &DisplayRowActiveFaceState,
        placement: CapturedCursorPlacement,
        line_height: f32,
    ) -> Self {
        Self::from_visual_state(
            CapturedCursorVisualState::line_break_from_active_face_state(
                active_face_state,
                line_height,
            ),
            placement,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct ResolvedCursorGeometry {
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    ascent: f32,
    style: CursorStyle,
    color: Color,
    cursor_fg: Color,
}

#[derive(Clone, Copy, Debug)]
struct CursorGeometrySource {
    slot_id: DisplaySlotId,
    x: f32,
    y: f32,
    slot_width: f32,
    face_height: f32,
    face_ascent: f32,
    row_height: f32,
    row_ascent: f32,
    default_line_height: f32,
    stretch_like: bool,
    ends_at_visible_eob: bool,
    cursor_fg: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CursorGeometryContext {
    window_id: i64,
    slot_width: f32,
    default_line_height: f32,
    ends_at_visible_eob: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct VisualCursorGeometryContext {
    window_id: i64,
    text_area_left: f32,
    window_top: f32,
}

impl CursorGeometrySource {
    fn from_captured_cursor(
        cursor: &CapturedCursorInfo,
        row_metric: RowMetricsSnapshot,
        context: CursorGeometryContext,
    ) -> Self {
        Self {
            slot_id: DisplaySlotId {
                window_id: context.window_id,
                row: row_metric.row as u32,
                col: cursor.col as u16,
            },
            x: cursor.x,
            y: cursor.y,
            slot_width: context.slot_width.max(1.0),
            face_height: cursor.face_h,
            face_ascent: cursor.face_ascent,
            row_height: row_metric.height,
            row_ascent: row_metric.ascent,
            default_line_height: context.default_line_height,
            stretch_like: cursor.stretch_like,
            ends_at_visible_eob: context.ends_at_visible_eob,
            cursor_fg: cursor.bg,
        }
    }

    fn from_display_point(
        point: &DisplayPointSnapshot,
        context: VisualCursorGeometryContext,
    ) -> Self {
        let point_h = (point.height as f32).max(1.0);
        Self {
            slot_id: DisplaySlotId {
                window_id: context.window_id,
                row: point.row.max(0) as u32,
                col: point.col.max(0) as u16,
            },
            x: context.text_area_left + point.x as f32,
            y: context.window_top + point.y as f32,
            slot_width: (point.width as f32).max(1.0),
            face_height: point_h,
            face_ascent: point_h,
            row_height: point_h,
            row_ascent: point_h,
            default_line_height: point_h,
            stretch_like: false,
            ends_at_visible_eob: false,
            cursor_fg: Color::BLACK,
        }
    }
}

impl ResolvedCursorGeometry {
    fn window_id(&self) -> i64 {
        self.slot_id.window_id
    }

    fn row(&self) -> usize {
        self.slot_id.row as usize
    }

    fn col(&self) -> u16 {
        self.slot_id.col
    }
}

fn window_cursor_kind(style: CursorStyle) -> WindowCursorKind {
    match style {
        CursorStyle::FilledBox => WindowCursorKind::FilledBox,
        CursorStyle::Hollow => WindowCursorKind::HollowBox,
        CursorStyle::Bar(_) => WindowCursorKind::Bar,
        CursorStyle::Hbar(_) => WindowCursorKind::Hbar,
    }
}

fn capture_cursor_info(target: &mut Option<CapturedCursorInfo>, info: CapturedCursorInfo) {
    if target.is_none() {
        *target = Some(info);
    }
}

fn update_cursor_info_for_main_char(
    target: &mut Option<CapturedCursorInfo>,
    byte_idx: usize,
    advance: f32,
) {
    let Some(cursor) = target.as_mut() else {
        return;
    };
    if cursor.byte_idx != byte_idx {
        return;
    }
    cursor.slot_width = Some(advance.max(1.0));
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DisplaySpaceGeometry {
    width: f32,
    height: f32,
    ascent: f32,
}

#[cfg(test)]
fn eval_status_line_format(
    evaluator: &mut neovm_core::emacs_core::Context,
    format_symbol: &str,
    window_id: i64,
    buffer_id: u64,
    target_cols: usize,
) -> Option<String> {
    eval_status_line_format_value(evaluator, format_symbol, window_id, buffer_id, target_cols)
        .and_then(|val| val.as_runtime_string_owned())
        .filter(|s| !s.is_empty())
}

fn eval_status_line_format_value(
    evaluator: &mut neovm_core::emacs_core::Context,
    format_symbol: &str,
    window_id: i64,
    buffer_id: u64,
    target_cols: usize,
) -> Option<Value> {
    evaluator.setup_thread_locals();
    // GNU Emacs (xdisp.c:28187): format-mode-line reads the format
    // variable from the TARGET buffer, not the caller's current
    // buffer. We must read the buffer-local value of mode-line-format
    // from the specified buffer BEFORE calling the walker.
    let window_format_value = evaluator
        .frame_manager()
        .window_parameter(WindowId(window_id as u64), &Value::symbol(format_symbol));
    let format_value = window_format_value
        .filter(|value| !value.is_nil())
        .unwrap_or_else(|| {
            evaluator
                .buffer_manager()
                .get(BufferId(buffer_id))
                .and_then(|buf| buf.buffer_local_value(format_symbol))
                .unwrap_or_else(|| {
                    // Fall back to the global default
                    evaluator
                        .obarray()
                        .symbol_value(format_symbol)
                        .copied()
                        .unwrap_or(Value::NIL)
                })
        });
    // GNU `display_mode_line` (xdisp.c:27911) runs the mode-line
    // walker in `MODE_LINE_DISPLAY` mode, which makes `%-` expand to
    // dashes filling the remaining row width. Our layout engine is the
    // equivalent redisplay path, so we call
    // `format_mode_line_for_display` directly rather than going
    // through the Lisp-facing `format-mode-line` builtin (which uses
    // `MODE_LINE_STRING` and returns `"--"` for `%-`).
    //
    // `target_cols` is the window's width in character cells, which
    // the DISPLAY walker uses to size the dash fill for `%-`.
    let rendered = neovm_core::emacs_core::xdisp::format_mode_line_for_display(
        evaluator,
        format_value,
        Value::make_window(window_id as u64),
        Value::make_buffer(BufferId(buffer_id)),
        target_cols,
    );
    if rendered
        .as_runtime_string_owned()
        .is_some_and(|s| !s.is_empty())
    {
        Some(rendered)
    } else {
        None
    }
}

fn tab_bar_menu_item_caption(entry: Value) -> Option<Value> {
    if let Some(items) = list_to_vec(&entry) {
        if items
            .get(1)
            .is_some_and(|value| KeymapMarker::MenuItem.is_value(*value))
        {
            let caption = *items.get(2)?;
            return caption.is_string().then_some(caption);
        }
    }

    if !entry.is_cons() {
        return None;
    }
    let pair_cdr = entry.cons_cdr();
    let items = list_to_vec(&pair_cdr)?;
    if !items
        .first()
        .is_some_and(|value| KeymapMarker::MenuItem.is_value(*value))
    {
        return None;
    }
    let caption = *items.get(1)?;
    caption.is_string().then_some(caption)
}

struct BuiltTabBar {
    text: Value,
    items: Vec<neomacs_display_protocol::ui_types::TabBarItem>,
}

struct ScratchGcRootScope {
    saved_len: usize,
}

impl ScratchGcRootScope {
    fn new() -> Self {
        Self {
            saved_len: neovm_core::emacs_core::eval::save_scratch_gc_roots(),
        }
    }

    fn root(&self, value: Value) {
        neovm_core::emacs_core::eval::push_scratch_gc_root(value);
    }
}

impl Drop for ScratchGcRootScope {
    fn drop(&mut self) {
        neovm_core::emacs_core::eval::restore_scratch_gc_roots(self.saved_len);
    }
}

fn build_tab_bar_display(
    evaluator: &mut neovm_core::emacs_core::Context,
    frame_id: u64,
    gc_roots: &ScratchGcRootScope,
) -> Option<BuiltTabBar> {
    evaluator.setup_thread_locals();
    if !evaluator.obarray().fboundp("tab-bar-make-keymap-1") {
        return None;
    }

    let saved_frame = evaluator
        .eval_form(Value::list(vec![Value::symbol("selected-frame")]))
        .ok();
    if let Some(frame) = saved_frame {
        gc_roots.root(frame);
    }
    let saved_window = evaluator
        .eval_form(Value::list(vec![Value::symbol("selected-window")]))
        .ok();
    if let Some(window) = saved_window {
        gc_roots.root(window);
    }
    let saved_buffer = evaluator
        .buffer_manager()
        .current_buffer()
        .map(|buffer| buffer.id());

    evaluator
        .eval_form(Value::list(vec![
            Value::symbol("select-frame"),
            Value::make_frame(frame_id),
            Value::NIL,
        ]))
        .ok()?;

    let result = evaluator
        .eval_form(Value::list(vec![Value::symbol("tab-bar-make-keymap-1")]))
        .ok()
        .and_then(|keymap| list_to_vec(&keymap))
        .and_then(|entries| {
            let mut text_values = Vec::new();
            let mut items = Vec::new();
            for (index, entry) in entries.iter().enumerate() {
                if index == 0 && KeymapMarker::Keymap.is_value(*entry) {
                    continue;
                }

                if is_list_keymap(entry) {
                    break;
                }

                if let Some(caption) = tab_bar_menu_item_caption(*entry) {
                    let label = caption.as_runtime_string_owned().unwrap_or_default();
                    text_values.push(caption);
                    items.push(neomacs_display_protocol::ui_types::TabBarItem {
                        index: items.len() as u32,
                        label,
                        help: String::new(),
                        enabled: true,
                        selected: false,
                        is_separator: false,
                    });
                }
            }

            if text_values.is_empty() {
                return None;
            }
            let mut concat_form = Vec::with_capacity(text_values.len() + 1);
            concat_form.push(Value::symbol("concat"));
            concat_form.extend(text_values);
            let text = evaluator.eval_form(Value::list(concat_form)).ok()?;
            text.as_runtime_string_owned()
                .is_some_and(|text| !text.is_empty())
                .then_some(BuiltTabBar { text, items })
        });
    if let Some(tab_bar) = &result {
        gc_roots.root(tab_bar.text);
    }

    if let Some(frame) = saved_frame {
        let _ = evaluator.eval_form(Value::list(vec![
            Value::symbol("select-frame"),
            frame,
            Value::NIL,
        ]));
    }
    if let Some(window) = saved_window {
        let _ = evaluator.eval_form(Value::list(vec![
            Value::symbol("select-window"),
            window,
            Value::NIL,
        ]));
    }
    if let Some(buffer_id) = saved_buffer {
        if evaluator.buffer_manager().get(buffer_id).is_some() {
            evaluator.buffer_manager_mut().set_current(buffer_id);
        }
    }

    result
}

impl LigatureRunBuffer {
    fn new() -> Self {
        Self {
            chars: Vec::with_capacity(MAX_LIGATURE_RUN_LEN),
            advances: Vec::with_capacity(MAX_LIGATURE_RUN_LEN),
            start_x: 0.0,
            start_y: 0.0,
            face_h: 0.0,
            face_ascent: 0.0,
            face_id: 0,
            total_advance: 0.0,
            is_overlay: false,
        }
    }

    fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    fn len(&self) -> usize {
        self.chars.len()
    }

    fn clear(&mut self) {
        self.chars.clear();
        self.advances.clear();
        self.total_advance = 0.0;
    }

    /// Push a character and its advance width into the run.
    fn push(&mut self, ch: char, advance: f32) {
        self.chars.push(ch);
        self.advances.push(advance);
        self.total_advance += advance;
    }

    /// Start a new run at the given position with the given face parameters.
    fn start(
        &mut self,
        x: f32,
        y: f32,
        face_h: f32,
        face_ascent: f32,
        face_id: u32,
        is_overlay: bool,
    ) {
        self.clear();
        self.start_x = x;
        self.start_y = y;
        self.face_h = face_h;
        self.face_ascent = face_ascent;
        self.face_id = face_id;
        self.is_overlay = is_overlay;
    }
}

/// Check if a character is a ligature-eligible symbol/punctuation.
/// Programming font ligatures only form between these characters.
#[inline]
#[cfg(test)]
fn is_ligature_char(ch: char) -> bool {
    matches!(
        ch,
        '!' | '#'
            | '$'
            | '%'
            | '&'
            | '*'
            | '+'
            | '-'
            | '.'
            | '/'
            | ':'
            | ';'
            | '<'
            | '='
            | '>'
            | '?'
            | '@'
            | '\\'
            | '^'
            | '|'
            | '~'
    )
}

/// Check if a run consists entirely of ligature-eligible characters.
/// Mixed runs (e.g., "arrow:" or "Font:") should NOT be composed,
/// only pure symbol runs (e.g., "->", "!=", "===").
#[inline]
#[cfg(test)]
fn run_is_pure_ligature(run: &LigatureRunBuffer) -> bool {
    run.chars.iter().all(|&ch| is_ligature_char(ch))
}

/// Flush the accumulated ligature run as either individual chars or a composed glyph.
///
/// NOTE: Glyph output has been migrated to `GlyphMatrixBuilder`. This function is now
/// a no-op retained only to keep call-sites compiling during the migration.
fn flush_run(_run: &LigatureRunBuffer, _ligatures: bool) {}

#[inline]
fn skip_to_newline(text: &[u8], byte_idx: &mut usize, charpos: &mut i64) -> bool {
    while *byte_idx < text.len() {
        let (ch, ch_len) = decode_utf8(&text[*byte_idx..]);
        if ch_len == 0 {
            break;
        }
        *byte_idx += ch_len;
        *charpos += 1;
        if ch == '\n' {
            return true;
        }
    }
    false
}

#[inline]
fn skip_text_to_charpos(text: &[u8], byte_idx: &mut usize, charpos: &mut i64, target: i64) {
    while *charpos < target && *byte_idx < text.len() {
        let (_ch, ch_len) = decode_utf8(&text[*byte_idx..]);
        if ch_len == 0 {
            break;
        }
        *byte_idx += ch_len;
        *charpos += 1;
    }
}

fn row_metrics_for_cursor(
    row_metrics: &[RowMetricsSnapshot],
    cursor_row: usize,
    current_row: usize,
    current_row_y: f32,
    current_row_height: f32,
    current_row_ascent: f32,
) -> RowMetricsSnapshot {
    row_metrics
        .iter()
        .find(|metric| metric.row == cursor_row)
        .copied()
        .unwrap_or(RowMetricsSnapshot {
            row: current_row,
            pixel_y: current_row_y,
            height: current_row_height.max(1.0),
            ascent: current_row_ascent.max(0.0).min(current_row_height.max(1.0)),
        })
}

fn resolve_cursor_vertical_metrics(
    cursor_y: f32,
    face_h: f32,
    face_ascent: f32,
    row_height: f32,
    row_ascent: f32,
    default_line_height: f32,
    ends_at_visible_eob: bool,
) -> (f32, f32, f32) {
    let row_height = row_height.max(1.0);
    let glyph_ascent = face_ascent.max(0.0).min(face_h.max(1.0));
    let glyph_descent = (face_h - glyph_ascent).max(0.0);
    let mut y = cursor_y;
    let mut ascent = row_ascent.max(0.0).min(row_height);

    // GNU's physical cursor follows the row baseline, but if the glyph under
    // point rises above that baseline, the cursor origin shifts upward to keep
    // the box aligned with the displayed glyph. End-of-buffer rows are the
    // exception because point can sit on an empty visual slot there.
    if !ends_at_visible_eob && ascent < glyph_ascent {
        y -= glyph_ascent - ascent;
        ascent = glyph_ascent.min(row_height);
    }

    let minimum_height = default_line_height.max(1.0).min(row_height);
    let height = (ascent + glyph_descent).max(minimum_height).min(row_height);
    (y, height, ascent.min(height))
}

fn resolve_cursor_geometry(
    style: CursorStyle,
    source: CursorGeometrySource,
    x_stretch_cursor: bool,
    fallback_char_width: f32,
    color: Color,
) -> ResolvedCursorGeometry {
    let actual_slot_width = match style {
        CursorStyle::Bar(width) => width.max(1.0),
        CursorStyle::Hbar(_) | CursorStyle::FilledBox | CursorStyle::Hollow => {
            source.slot_width.max(1.0)
        }
    };
    let width = if source.stretch_like && !x_stretch_cursor && !matches!(style, CursorStyle::Bar(_))
    {
        fallback_char_width.max(1.0)
    } else {
        actual_slot_width
    };
    let (y, height, ascent) = resolve_cursor_vertical_metrics(
        source.y,
        source.face_height,
        source.face_ascent,
        source.row_height,
        source.row_ascent,
        source.default_line_height,
        source.ends_at_visible_eob,
    );

    ResolvedCursorGeometry {
        slot_id: source.slot_id,
        x: source.x,
        y,
        width,
        height,
        ascent,
        style,
        color,
        cursor_fg: source.cursor_fg,
    }
}

struct ReplacementStringItemMeasurer<'a> {
    font_metrics_svc: &'a mut Option<FontMetricsService>,
    active_face_state: DisplayRowActiveFaceState,
}

impl<'a> ReplacementStringItemMeasurer<'a> {
    fn from_active_face_state(
        font_metrics_svc: &'a mut Option<FontMetricsService>,
        active_face_state: &DisplayRowActiveFaceState,
    ) -> Self {
        Self {
            font_metrics_svc,
            active_face_state: active_face_state.clone(),
        }
    }
}

impl DisplayRowItemMeasurer for ReplacementStringItemMeasurer<'_> {
    fn measurement_for(
        &mut self,
        item: &crate::display_item::DisplayItem,
        _face_id: u32,
    ) -> DisplayRowItemMeasurement {
        let crate::display_item::DisplayItemKind::SourceMappedText(text) = &item.kind else {
            return DisplayRowItemMeasurement::Default;
        };
        DisplayRowItemMeasurement::TextRun(
            self.active_face_state
                .text_run_measurement(self.font_metrics_svc, text.text.as_ref()),
        )
    }
}

fn next_window_start_from_visible_rows(
    rows: &[DisplayRowSnapshot],
    current_start: i64,
) -> Option<i64> {
    if rows.is_empty() {
        return None;
    }

    rows.iter()
        .rev()
        .filter_map(row_next_window_start_charpos)
        .find(|&pos| pos > current_start)
}

#[inline]
fn row_start_charpos(row: &DisplayRowSnapshot) -> Option<i64> {
    row.start_buffer_pos.map(lisp_char_pos_to_layout_i64)
}

#[inline]
fn row_end_charpos(row: &DisplayRowSnapshot) -> Option<i64> {
    row.end_buffer_pos.map(lisp_char_pos_to_layout_i64)
}

#[inline]
fn row_next_window_start_charpos(row: &DisplayRowSnapshot) -> Option<i64> {
    row.end_buffer_pos
        .map(LispCharPos1::as_i64)
        .or_else(|| row_start_charpos(row))
}

fn next_window_start_for_partially_visible_point_row(
    rows: &[DisplayRowSnapshot],
    point: i64,
    text_area_top: i64,
    text_area_bottom: i64,
    current_start: i64,
) -> Option<i64> {
    let text_area_height = text_area_bottom.saturating_sub(text_area_top);
    let point_row_index = rows.iter().position(|row| {
        let start = row_start_charpos(row).unwrap_or(i64::MAX);
        let end = row_end_charpos(row).unwrap_or(i64::MIN);
        start <= point && point <= end
    })?;
    let point_row = &rows[point_row_index];
    if point_row.height > text_area_height {
        return None;
    }

    let row_top = point_row.y;
    let row_bottom = point_row.y.saturating_add(point_row.height);
    if row_top >= text_area_top && row_bottom <= text_area_bottom {
        return None;
    }

    if row_bottom > text_area_bottom {
        let overflow = row_bottom.saturating_sub(text_area_bottom);
        let mut lifted = 0i64;
        for row in rows.iter().take(point_row_index) {
            lifted = lifted.saturating_add(row.height.max(1));
            let candidate = row_next_window_start_charpos(row);
            if lifted >= overflow
                && let Some(pos) = candidate
                && pos > current_start
            {
                return Some(pos);
            }
        }
    }

    None
}

fn next_window_start_for_point_line_continuation<B: super::neovm_bridge::LayoutBufferView>(
    rows: &[DisplayRowSnapshot],
    point: i64,
    current_start: i64,
    buf_access: &super::neovm_bridge::RustBufferAccess<'_, B>,
    buffer_size: i64,
) -> Option<i64> {
    let point_row_index = rows.iter().position(|row| {
        let start = row_start_charpos(row).unwrap_or(i64::MAX);
        let end = row_end_charpos(row).unwrap_or(i64::MIN);
        start <= point && point <= end
    })?;
    let point_row = rows.get(point_row_index)?;
    let point_is_visible_row_start =
        row_start_charpos(point_row).is_some_and(|start| start == point);

    for row in rows.iter().skip(point_row_index) {
        let end_pos = row.end_buffer_pos?.as_i64();
        let end_byte = buf_access.lisp_charpos_to_bytepos(end_pos);
        if matches!(buf_access.byte_at(end_byte), Some(b'\n')) {
            return None;
        }
        let next_pos = end_pos.saturating_add(1);
        if next_pos > buffer_size {
            return None;
        }

        let next_byte = buf_access.lisp_charpos_to_bytepos(next_pos);
        match buf_access.byte_at(next_byte) {
            Some(b'\n') | None => return None,
            Some(_) if std::ptr::eq(row, rows.last()?) => {
                if point_is_visible_row_start {
                    return point
                        .checked_sub(1)
                        .filter(|&new_start| new_start > current_start);
                }
                break;
            }
            Some(_) => {}
        }
    }

    if point_row_index + 1 < rows.len() {
        return None;
    }

    rows.iter()
        .skip(1)
        .find_map(row_next_window_start_charpos)
        .filter(|&pos| pos > current_start)
}

// ---------------------------------------------------------------------------
// Display property helpers
// ---------------------------------------------------------------------------

/// Evaluate a `(space ...)` display spec into GNU-shaped stretch geometry.
///
/// Replaces the old `parse_display_space_width` helper. Delegates the
/// actual expression evaluation to
/// [`crate::display_pixel_calc::calc_pixel_width_or_height`], the
/// faithful port of GNU `xdisp.c:30102`. Supports the full GNU
/// expression grammar: fixnum/float, symbols (`right`, `text`,
/// `left-fringe`, etc.), arithmetic forms `(+ …)`/`(- …)`,
/// pixel-literal `(NUM)`, and unit-scaled `(NUM . UNIT)`.
///
/// GNU's xdisp.c uses canonical frame column width for these numeric
/// units, not the currently scaled face width of the covered buffer
/// position.
///
/// Returns canonical frame column/default face metrics when the spec is
/// invalid or the evaluator can't resolve it.
fn eval_display_space_geometry(
    spec: &neovm_core::emacs_core::Value,
    current_x: f32,
    content_x: f32,
    face_char_w: f32,
    display_char_width: f32,
    default_height: f32,
    default_ascent: f32,
    params: &WindowParams,
) -> DisplaySpaceGeometry {
    use crate::display_pixel_calc::{PixelCalcContext, calc_pixel_width_or_height};

    let default_width = params.char_width.max(1.0);
    let default_height = if params.window_system {
        default_height.max(1.0)
    } else {
        params.char_height.max(1.0)
    };
    let default_ascent = if params.window_system {
        default_ascent.max(0.0).min(default_height)
    } else {
        default_height
    };
    let Some(items) = neovm_core::emacs_core::value::list_to_vec(spec) else {
        return DisplaySpaceGeometry {
            width: default_width,
            height: default_height,
            ascent: default_ascent,
        };
    };

    let pctx = PixelCalcContext {
        frame_column_width: params.char_width.max(1.0) as f64,
        frame_line_height: params.char_height.max(1.0) as f64,
        frame_res_x: 96.0,
        frame_res_y: 96.0,
        face_font_height: default_height as f64,
        face_font_width: face_char_w.round().max(1.0) as f64,
        text_area_left: params.text_bounds.x as f64,
        text_area_right: (params.text_bounds.x + params.text_bounds.width) as f64,
        text_area_width: params.text_bounds.width as f64,
        left_margin_left: (params.text_bounds.x
            - params.left_fringe_width
            - params.left_margin_width) as f64,
        left_margin_width: params.left_margin_width as f64,
        right_margin_left: (params.text_bounds.x
            + params.text_bounds.width
            + params.right_fringe_width) as f64,
        right_margin_width: params.right_margin_width as f64,
        left_fringe_width: params.left_fringe_width as f64,
        right_fringe_width: params.right_fringe_width as f64,
        fringes_outside_margins: false,
        scroll_bar_width: 0.0,
        scroll_bar_on_left: false,
        line_number_pixel_width: 0.0,
        symbol_values: std::collections::HashMap::new(),
    };

    let plist_value = |wanted: DisplaySpaceKey| -> Option<Value> {
        let mut i = 1;
        while i + 1 < items.len() {
            if DisplaySpaceKey::from_lisp_value(items[i]) == Some(wanted) {
                return Some(items[i + 1]);
            }
            i += 2;
        }
        None
    };

    let mut width = if let Some(prop) = plist_value(DisplaySpaceKey::Width)
        && !prop.is_nil()
        && let Some(pixels) = calc_pixel_width_or_height(&pctx, &prop, true, None)
    {
        pixels as f32
    } else if let Some(prop) = plist_value(DisplaySpaceKey::RelativeWidth)
        && let Some(factor) = display_space_positive_number(prop)
    {
        factor * display_char_width.max(0.0)
    } else if let Some(prop) = plist_value(DisplaySpaceKey::AlignTo)
        && !prop.is_nil()
    {
        let mut align_to: i32 = -1;
        if let Some(pixels) = calc_pixel_width_or_height(&pctx, &prop, true, Some(&mut align_to)) {
            // If the expression contained a symbol like `right`, `align_to`
            // was updated to that position and `pixels` is the offset from it.
            // Otherwise, numeric-only `:align-to N` is column-relative from
            // `content_x`, matching GNU's text-area adjustment.
            let target_x = if align_to >= 0 {
                align_to as f32 + pixels as f32
            } else {
                content_x + pixels as f32
            };
            (target_x - current_x).max(0.0)
        } else {
            default_width
        }
    } else {
        default_width
    };
    let zero_width_ok = plist_value(DisplaySpaceKey::AlignTo).is_some_and(|prop| !prop.is_nil());
    if width <= 0.0 && (width < 0.0 || !zero_width_ok) {
        width = 1.0;
    }

    let (height, ascent) = if params.window_system {
        let mut height = if let Some(prop) = plist_value(DisplaySpaceKey::Height)
            && !prop.is_nil()
            && let Some(pixels) = calc_pixel_width_or_height(&pctx, &prop, false, None)
        {
            pixels as f32
        } else if let Some(prop) = plist_value(DisplaySpaceKey::RelativeHeight)
            && let Some(factor) = display_space_positive_number(prop)
        {
            default_height * factor
        } else {
            default_height
        };
        let zero_height_ok =
            plist_value(DisplaySpaceKey::Height).is_some_and(|prop| !prop.is_nil());
        if height <= 0.0 && (height < 0.0 || !zero_height_ok) {
            height = 1.0;
        }

        let ascent = if let Some(prop) = plist_value(DisplaySpaceKey::Ascent) {
            if let Some(percent) = display_space_positive_number(prop)
                && percent <= 100.0
            {
                height * percent / 100.0
            } else if !prop.is_nil()
                && let Some(pixels) = calc_pixel_width_or_height(&pctx, &prop, false, None)
            {
                (pixels as f32).max(0.0).min(height)
            } else {
                height * default_ascent / default_height
            }
        } else {
            height * default_ascent / default_height
        };
        (height, ascent)
    } else {
        // GNU `produce_stretch_glyph` does not append a pixel stretch glyph on
        // terminals; it appends ordinary TTY space glyphs and leaves the row
        // one terminal cell high, ignoring :height/:relative-height/:ascent.
        (1.0, 1.0)
    };

    DisplaySpaceGeometry {
        width,
        height,
        ascent: ascent.max(0.0).min(height),
    }
}

fn max_mini_window_lines(evaluator: &Context, frame_rows: f32) -> f32 {
    let raw = evaluator
        .obarray()
        .symbol_value("max-mini-window-height")
        .copied()
        .unwrap_or_else(|| Value::make_float(0.25));
    match raw.kind() {
        neovm_core::emacs_core::value::ValueKind::Float => {
            (frame_rows * raw.as_float().unwrap_or(0.25) as f32).max(1.0)
        }
        neovm_core::emacs_core::value::ValueKind::Fixnum(_) => raw.as_int().unwrap_or(1) as f32,
        _ => 1.0,
    }
}

fn message_truncate_lines(evaluator: &Context) -> bool {
    evaluator
        .obarray()
        .symbol_value("message-truncate-lines")
        .is_some_and(|value| !value.is_nil())
}

fn minibuffer_echo_message_for_window(
    is_minibuffer_window: bool,
    active_minibuffer_window: bool,
    current_message: Option<Value>,
) -> Option<Value> {
    if !is_minibuffer_window || active_minibuffer_window {
        return None;
    }
    current_message.filter(|message| {
        message
            .as_runtime_string_owned()
            .is_some_and(|text| !text.is_empty())
    })
}

#[inline]
fn next_tab_stop_col(current_col: usize, tab_width: i32, tab_stop_list: &[i32]) -> usize {
    if !tab_stop_list.is_empty() {
        if let Some(&stop) = tab_stop_list
            .iter()
            .find(|&&stop| (stop as usize) > current_col)
        {
            return stop as usize;
        }
        let last = *tab_stop_list.last().unwrap() as usize;
        let tab_w = tab_width.max(1) as usize;
        if current_col >= last {
            return last + ((current_col - last) / tab_w + 1) * tab_w;
        }
        return last;
    }

    let tab_w = tab_width.max(1) as usize;
    ((current_col / tab_w) + 1) * tab_w
}

#[inline]
fn is_word_wrap_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t')
}

#[inline]
fn char_can_wrap_before_basic(ch: char) -> bool {
    !matches!(ch, ' ' | '\t' | '\n' | '\r')
}

#[inline]
fn char_can_wrap_after_basic(ch: char) -> bool {
    is_word_wrap_whitespace(ch)
}

#[inline]
fn cursor_point_columns(text: &[u8], byte_idx: usize, col: i32, params: &WindowParams) -> usize {
    if byte_idx >= text.len() {
        return 1;
    }

    let (ch, _) = decode_utf8(&text[byte_idx..]);
    match ch {
        '\t' => {
            let col_usize = col.max(0) as usize;
            let next_tab = next_tab_stop_col(col_usize, params.tab_width, &params.tab_stop_list)
                .max(col_usize + 1);
            next_tab - col_usize
        }
        '\n' | '\r' => 1,
        _ if is_cluster_extender(ch) => 0,
        _ if is_wide_char(ch) => 2,
        _ => 1,
    }
}

#[inline]
fn cursor_width_for_style(
    style: CursorStyle,
    text: &[u8],
    byte_idx: usize,
    col: i32,
    params: &WindowParams,
    face_char_w: f32,
) -> f32 {
    match style {
        CursorStyle::Bar(w) => w,
        CursorStyle::Hbar(_) => {
            cursor_point_columns(text, byte_idx, col, params) as f32 * face_char_w
        }
        _ => {
            if !params.x_stretch_cursor && byte_idx < text.len() {
                let (ch, _) = decode_utf8(&text[byte_idx..]);
                if ch == '\t' {
                    return params.char_width.max(1.0);
                }
            }
            cursor_point_columns(text, byte_idx, col, params) as f32 * face_char_w
        }
    }
}

#[inline]
fn cursor_style_for_window(params: &WindowParams) -> Option<CursorStyle> {
    use neomacs_display_protocol::frame_glyphs::CursorKind;

    if params.cursor_kind == CursorKind::NoCursor {
        return None;
    }

    CursorStyle::from_kind(params.cursor_kind, params.cursor_bar_width)
}

fn cursor_style_for_visual(spec: &VisualCursorSpec) -> Option<CursorStyle> {
    use neomacs_display_protocol::frame_glyphs::CursorKind;

    if spec.cursor_kind == CursorKind::NoCursor {
        return None;
    }

    CursorStyle::from_kind(spec.cursor_kind, spec.cursor_bar_width)
}

fn visual_cursor_source_from_point(
    point: &DisplayPointSnapshot,
    window_id: i64,
    text_area_left: f32,
    window_top: f32,
) -> CursorGeometrySource {
    CursorGeometrySource::from_display_point(
        point,
        VisualCursorGeometryContext {
            window_id,
            text_area_left,
            window_top,
        },
    )
}

fn text_display_tab_policy(
    content_x: f32,
    params: &WindowParams,
) -> crate::display_row_builder::DisplayTabPolicy {
    crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(
        content_x,
        params.tab_width,
        &params.tab_stop_list,
    )
}

#[derive(Clone, Debug)]
struct DisplayStringBaseFace {
    face: super::neovm_bridge::ResolvedFace,
    face_id: u32,
}

fn display_string_base_face<B: super::neovm_bridge::LayoutBufferView>(
    buffer: &B,
    face_resolver: &super::neovm_bridge::FaceResolver,
    origin: DisplayOrigin,
    policy: BaseFacePolicy,
    current_face_id: &mut u32,
    builder: &mut crate::matrix_builder::GlyphMatrixBuilder,
) -> DisplayStringBaseFace {
    let mut next_check = buffer.layout_point_max_char_pos().get();
    let face = face_resolver.base_face_for_origin(Some(buffer), &origin, policy, &mut next_check);
    let face_id = if crate::display_source_resolver::same_resolved_face(
        &face,
        face_resolver.default_face(),
    ) {
        u32::from(neomacs_display_protocol::face::BasicFaceId::Default)
    } else {
        let face_id = *current_face_id;
        *current_face_id += 1;
        face_id
    };
    insert_resolved_display_row_face(builder, face_id, &face, None);
    DisplayStringBaseFace { face, face_id }
}

/// Render overlay string bytes into the layout.
///
/// On `\n`: ends the current glyph row, advances `row`/`y`, begins a new row,
/// and resets `x`/`col` — matching GNU `display_line()` behaviour for overlay
/// strings that contain newlines (e.g. fido-vertical-mode completions).
fn render_overlay_string<B: super::neovm_bridge::LayoutBufferView>(
    evaluator: &mut Context,
    output_emitter: &mut WindowOutputEmitter,
    buffer: &B,
    fragment: DisplayTextFragment,
    font_metrics: &mut Option<FontMetricsService>,
    face_resolver: &super::neovm_bridge::FaceResolver,
    x: &mut f32,
    col: &mut usize,
    geometry: &mut DisplayRowGeometryState,
    cursor_info: &mut Option<CapturedCursorInfo>,
    hit_rows: &mut Vec<HitRow>,
    hit_row_charpos_start: &mut i64,
    anchor_charpos: i64,
    row_y_positions: &mut DisplayRowYPositions,
    face_char_w: f32,
    char_h: f32,
    default_row_ascent: f32,
    max_x: f32,
    content_x: f32,
    text_y: f32,
    row_base: usize,
    max_rows: usize,
    current_face_id: &mut u32,
    builder: &mut crate::matrix_builder::GlyphMatrixBuilder,
    params: &WindowParams,
) {
    let DisplayTextStorage::LispString(text_value) = fragment.storage else {
        return;
    };
    if text_value.as_lisp_string().is_none() {
        return;
    }
    let text_props = get_string_text_properties_table_for_value(text_value);
    let base_face = display_string_base_face(
        buffer,
        face_resolver,
        fragment.origin,
        fragment.base_face_policy,
        current_face_id,
        builder,
    );
    let row_geometry_defaults = DisplayRowGeometryDefaults::new(text_y, char_h, default_row_ascent);

    macro_rules! finish_overlay_string_row {
        () => {{
            let geometry_transition = geometry.finish_boundary_and_record_hit(
                DisplayRowBoundaryTarget::line_break(
                    DisplayRowHitRange {
                        charpos_start: *hit_row_charpos_start,
                        charpos_end: anchor_charpos,
                    },
                    row_geometry_defaults,
                    row_base,
                    0,
                    content_x,
                    0.0,
                    DisplayRowYRecording::None,
                ),
                hit_rows,
            );
            *hit_row_charpos_start = anchor_charpos;
            if geometry.row >= max_rows {
                TextMatrixRowOutput::new(builder, output_emitter, evaluator)
                    .finish_and_end(geometry_transition.finished_row);
                false
            } else {
                geometry.record_current_row_y(row_y_positions);
                *x = content_x;
                *col = 0;
                TextMatrixRowOutput::new(builder, output_emitter, evaluator)
                    .emit(geometry_transition);
                true
            }
        }};
    }

    let Some(mut source) = crate::display_source::LispStringSourceCursor::new(
        1,
        text_value,
        crate::display_item::RenderFaceRef::FaceId(base_face.face_id),
    ) else {
        return;
    };
    let mut source_state = DisplayRowSourceState::default();

    while geometry.row < max_rows {
        if *x >= max_x {
            break;
        }

        let row_spec = DisplayRowSpec {
            geometry: DisplayRowGeometry {
                y: geometry.y,
                width: max_x - content_x,
                height: char_h,
                char_width: face_char_w,
                ascent: default_row_ascent,
                tab_policy: text_display_tab_policy(content_x, params),
            },
            render_bounds: DisplayRowRenderBounds {
                start: DisplayRowPosition {
                    x_px: *x,
                    col: *col,
                },
                max_x_px: max_x,
            },
            base_face_id: base_face.face_id,
            base_face: &base_face.face,
            role: GlyphRowRole::Text,
            symbol_values: std::collections::HashMap::new(),
        };
        let Some(outcome) = render_natural_display_item_source_into_current_text_row_and_emit(
            builder,
            output_emitter,
            evaluator,
            font_metrics,
            &mut source,
            &mut source_state,
            face_resolver,
            current_face_id,
            row_spec,
            geometry.text_row_output(char_h),
        ) else {
            break;
        };
        let stop = outcome.stop;
        geometry.include_glyph_vertical_metrics(outcome.row_height_px, outcome.row_ascent_px);
        let overlay_cursor_visual_state = CapturedCursorVisualState {
            face_width: face_char_w,
            face_height: char_h,
            face_ascent: default_row_ascent,
            background: Color::from_pixel(base_face.face.bg),
        };
        for slot in &outcome.source_slots {
            capture_overlay_string_cursor_at_slot(
                text_props.as_ref(),
                slot,
                cursor_info,
                geometry.y,
                geometry.row,
                overlay_cursor_visual_state,
            );
        }
        *x = outcome.end.x_px;
        *col = outcome.end.col;

        if stop == DisplayRowRenderStop::RowBreak {
            // End current row, start a new one — mirrors the main text loop.
            if !finish_overlay_string_row!() {
                break;
            }
            continue;
        }
        match stop {
            DisplayRowRenderStop::SourceExhausted => break,
            DisplayRowRenderStop::Clipped => {
                source_state.discard_pending_item();
                if source.discard_until_row_break() {
                    if !finish_overlay_string_row!() {
                        break;
                    }
                    continue;
                }
                break;
            }
            DisplayRowRenderStop::RowBreak => unreachable!("row break handled above"),
        }
    }
}

fn root_lisp_position_char(source: &crate::display_item::DisplaySourcePosition) -> Option<usize> {
    match source {
        crate::display_item::DisplaySourcePosition::LispString {
            source_id,
            char_index,
            ..
        } if source_id.get() == 1 => Some(*char_index),
        _ => None,
    }
}

fn capture_overlay_string_cursor_at_slot(
    text_props: Option<&neovm_core::buffer::text_props::TextPropertyTable>,
    slot: &crate::display_row_builder::DisplayRowGlyphSlot,
    cursor_info: &mut Option<CapturedCursorInfo>,
    y: f32,
    matrix_row: usize,
    visual_state: CapturedCursorVisualState,
) {
    let Some(char_idx) = root_lisp_position_char(&slot.source) else {
        return;
    };
    capture_overlay_string_cursor(
        text_props,
        char_idx,
        cursor_info,
        slot.x_px,
        y,
        slot.col,
        matrix_row,
        visual_state,
        CapturedCursorSlotWidth::Explicit(slot.width_px),
    );
}

fn capture_overlay_string_cursor(
    text_props: Option<&neovm_core::buffer::text_props::TextPropertyTable>,
    char_idx: usize,
    cursor_info: &mut Option<CapturedCursorInfo>,
    x: f32,
    y: f32,
    col: usize,
    matrix_row: usize,
    visual_state: CapturedCursorVisualState,
    slot_width: CapturedCursorSlotWidth,
) {
    if cursor_info.is_some() {
        return;
    }
    let Some(props) = text_props else {
        return;
    };
    let Some(cursor_prop) =
        props.get_property_at_char_pos(CharPos0::new(char_idx), Value::symbol("cursor"))
    else {
        return;
    };
    if cursor_prop.is_nil() {
        return;
    }

    capture_cursor_info(
        cursor_info,
        CapturedCursorInfo::from_visual_state(
            visual_state,
            CapturedCursorPlacement {
                x,
                y,
                byte_idx: 0,
                col,
                matrix_row,
                slot_width,
                stretch_like: false,
            },
        ),
    );
}

/// The main Rust layout engine.
///
/// Called on the Emacs thread during redisplay. Reads buffer/state from
/// neovm-core, resolves faces, computes layout, and publishes immutable
/// display snapshots for the render thread and TTY backend.
pub struct LayoutEngine {
    /// Reusable text buffer to avoid allocation per frame
    text_buf: Vec<u8>,
    /// Hit-test data being built for current frame
    hit_data: Vec<WindowHitData>,
    /// Authoritative visible glyph geometry published back into core state.
    display_snapshots: Vec<WindowDisplaySnapshot>,
    /// Reusable ligature run buffer
    run_buf: LigatureRunBuffer,
    /// Whether ligatures are enabled
    pub ligatures_enabled: bool,
    /// Cosmic-text font metrics service.
    ///
    /// Populated by `enable_cosmic_metrics()` at GUI startup. Left
    /// `None` for TTY mode, where all measurements go through the
    /// character-cell grid. Replaces the previous
    /// `use_cosmic_metrics: bool` runtime flag — the decision is
    /// now made once at startup by the binary that constructs the
    /// layout engine.
    pub font_metrics: Option<FontMetricsService>,
    /// Converts Emacs face height units into layout pixels for this display.
    font_sizing: FontSizing,
    /// Previous frame's per-window metadata for transition hint derivation.
    prev_window_infos: std::collections::HashMap<i64, WindowInfo>,
    /// Previous selected window id for switch-fade detection.
    prev_selected_window_id: i64,
    /// Previous frame background for theme-transition detection.
    prev_background: Option<(f32, f32, f32, f32)>,
    /// Authoritative glyph-matrix builder for the current frame layout pass.
    pub matrix_builder: crate::matrix_builder::GlyphMatrixBuilder,
    /// The last completed `FrameDisplayState`, produced by `layout_frame_rust()`.
    /// Used by the TTY redisplay path to drive `TtyRif` on the evaluator thread.
    pub last_frame_display_state: Option<neomacs_display_protocol::glyph_matrix::FrameDisplayState>,
    /// Monotonic face-id allocator, frame-scoped.
    ///
    /// Mirrors GNU's frame-wide `face_cache->used` counter in
    /// `src/xfaces.c::realize_face`, which grows within a frame and
    /// never resets per window: windows on the same frame share a
    /// single face cache so two windows referencing the same face
    /// end up with the same `face_id`, and two windows referencing
    /// DIFFERENT faces get different ids.
    ///
    /// Before this field existed, `layout_window_rust` used a
    /// function-local `let mut current_face_id: u32 = 1;` which
    /// reset to 1 for every window. That collided with the
    /// frame-wide `matrix_builder.faces` HashMap: the first window
    /// inserted `mode-line` at face_id=2, the second window then
    /// inserted `mode-line-inactive` ALSO at face_id=2 and
    /// overwrote the first entry, causing both mode lines to
    /// render with the inactive face after `C-x 2`.
    /// Frame-scoped face-ID counter.  Starts at
    /// [`BasicFaceId::SENTINEL`] so dynamic face IDs never collide
    /// with the fixed basic-face slots (0–19).
    pub(crate) frame_face_id_counter: u32,
    /// Frame-level chrome rows built before leaf-window layout.
    ///
    /// GNU treats the tab bar as frame-level redisplay, not as a row owned by
    /// the first leaf window. Neomacs stages those rows here and attaches them
    /// to the finished frame snapshot.
    pending_frame_chrome_rows: Vec<neomacs_display_protocol::glyph_matrix::FrameChromeRow>,
    /// Frame-level tab bar metadata for render-thread hit-testing.
    pending_tab_bar: Option<neomacs_display_protocol::frame_glyphs::FrameTabBarState>,
}

fn empty_minibuffer_echo_row(y: f32, ascent: f32, row_height: f32) -> Vec<RenderedDisplayRow> {
    let mut row = neomacs_display_protocol::glyph_matrix::GlyphRow::new(GlyphRowRole::Minibuffer);
    row.enabled = true;
    row.height_px = row_height.max(1.0);
    row.ascent_px = ascent.max(0.0).min(row.height_px);
    vec![RenderedDisplayRow {
        row,
        progress: DisplayRowOutputProgress {
            end_x: 0.0,
            end_col: 0,
            y,
            height: row_height.max(1.0),
        },
        source_slots: Vec::new(),
        faces: Vec::new(),
        media: Vec::new(),
    }]
}

impl LayoutEngine {
    /// Create a new layout engine with cosmic-text font metrics.
    ///
    /// Initializes the `FontMetricsService` eagerly (~500ms font
    /// database scan). Used by GUI mode and tests that need pixel-
    /// accurate font measurement. TTY binaries should use
    /// `new_without_font_metrics()` to skip the scan.
    pub fn new() -> Self {
        Self {
            text_buf: Vec::with_capacity(64 * 1024), // 64KB initial
            hit_data: Vec::new(),
            display_snapshots: Vec::new(),
            run_buf: LigatureRunBuffer::new(),
            ligatures_enabled: false,
            font_metrics: Some(FontMetricsService::new()),
            font_sizing: FontSizing::xft(),
            prev_window_infos: std::collections::HashMap::new(),
            prev_selected_window_id: 0,
            prev_background: None,
            matrix_builder: crate::matrix_builder::GlyphMatrixBuilder::new(),
            last_frame_display_state: None,
            frame_face_id_counter: BasicFaceId::SENTINEL,
            pending_frame_chrome_rows: Vec::new(),
            pending_tab_bar: None,
        }
    }

    /// Create a layout engine without font metrics (TTY mode).
    ///
    /// Skips the ~500ms cosmic-text font database scan. All
    /// measurements fall back to the character-cell grid (1x1 for
    /// TTY, matching GNU Emacs frame.c:1184-1185). GUI binaries
    /// should use `new()` instead.
    pub fn new_without_font_metrics() -> Self {
        Self {
            text_buf: Vec::with_capacity(64 * 1024),
            hit_data: Vec::new(),
            display_snapshots: Vec::new(),
            run_buf: LigatureRunBuffer::new(),
            ligatures_enabled: false,
            font_metrics: None,
            font_sizing: FontSizing::xft(),
            prev_window_infos: std::collections::HashMap::new(),
            prev_selected_window_id: 0,
            prev_background: None,
            matrix_builder: crate::matrix_builder::GlyphMatrixBuilder::new(),
            last_frame_display_state: None,
            frame_face_id_counter: BasicFaceId::SENTINEL,
            pending_frame_chrome_rows: Vec::new(),
            pending_tab_bar: None,
        }
    }

    /// Disable cosmic-text font measurement (TTY mode).
    ///
    /// Drops the `FontMetricsService` so all measurements fall back
    /// to the character-cell grid. Called once at TTY startup from
    /// the binary that constructs the layout engine.
    pub fn disable_cosmic_metrics(&mut self) {
        self.font_metrics = None;
    }

    /// Enable cosmic-text font measurement for GUI rendering.
    ///
    /// Constructs the `FontMetricsService` if it hasn't already been
    /// constructed. Called once at GUI startup from the binary that
    /// sets up the layout engine. TTY mode skips this call and
    /// leaves `font_metrics` as `None`, so all measurements fall
    /// back to the character-cell grid (GNU Emacs frame.c:1184-1185:
    /// TTY frames have column_width=1 and line_height=1).
    ///
    /// This replaces the previous `use_cosmic_metrics: bool` runtime
    /// flag. The decision of which measurement strategy to use is
    /// now made once at startup by which binary constructs the
    /// engine, matching GNU's per-frame redisplay_interface vtable
    /// dispatch.
    pub fn enable_cosmic_metrics(&mut self) {
        if self.font_metrics.is_none() {
            self.font_metrics = Some(FontMetricsService::new());
        }
    }

    pub fn set_font_sizing(&mut self, font_sizing: FontSizing) {
        self.font_sizing = font_sizing;
    }

    fn record_transition_hint_from_latest_window_info(
        &mut self,
        curr_window_infos: &mut std::collections::HashMap<i64, WindowInfo>,
    ) {
        if let Some(curr) = self.matrix_builder.window_infos().last().cloned() {
            if let Some(prev) = self.prev_window_infos.get(&curr.window_id) {
                if let Some(hint) = FrameGlyphBuffer::derive_transition_hint(prev, &curr) {
                    self.matrix_builder.push_transition_hint(hint);
                }
            }
            curr_window_infos.insert(curr.window_id, curr);
        }
    }

    fn record_effect_hints_from_latest_window_info(&mut self) {
        let Some(curr) = self.matrix_builder.window_infos().last().cloned() else {
            return;
        };
        if curr.is_minibuffer {
            return;
        }

        let Some(prev) = self.prev_window_infos.get(&curr.window_id) else {
            return;
        };
        if prev.buffer_id == 0 || curr.buffer_id == 0 {
            return;
        }

        if prev.buffer_id != curr.buffer_id {
            let hint = WindowEffectHint::TextFadeIn {
                window_id: curr.window_id,
                bounds: curr.bounds,
            };
            self.matrix_builder.push_effect_hint(hint);
            return;
        }

        if prev.window_start != curr.window_start {
            let direction = if curr.window_start > prev.window_start {
                1
            } else {
                -1
            };
            let delta = (curr.window_start - prev.window_start).unsigned_abs() as f32;
            let h1 = WindowEffectHint::TextFadeIn {
                window_id: curr.window_id,
                bounds: curr.bounds,
            };
            self.matrix_builder.push_effect_hint(h1);
            let h2 = WindowEffectHint::ScrollLineSpacing {
                window_id: curr.window_id,
                bounds: curr.bounds,
                direction,
            };
            self.matrix_builder.push_effect_hint(h2);
            let h3 = WindowEffectHint::ScrollMomentum {
                window_id: curr.window_id,
                bounds: curr.bounds,
                direction,
            };
            self.matrix_builder.push_effect_hint(h3);
            let h4 = WindowEffectHint::ScrollVelocityFade {
                window_id: curr.window_id,
                bounds: curr.bounds,
                delta,
            };
            self.matrix_builder.push_effect_hint(h4);
        }
    }

    /// Compute and emit scroll bar glyphs for a window.
    ///
    /// Mirrors GNU `set_vertical_scroll_bar` (xdisp.c:20109) and the
    /// GTK/wgpu scroll bar rendering path.  The thumb position and size
    /// are proportional to the visible region within the accessible buffer.
    fn emit_window_scroll_bars(&mut self, params: &WindowParams) {
        let Some(info) = self
            .matrix_builder
            .window_infos()
            .iter()
            .rev()
            .find(|info| info.window_id == params.window_id)
        else {
            return;
        };
        let track_color = Color::new(0.7, 0.7, 0.7, 1.0);
        let thumb_color = Color::new(0.5, 0.5, 0.5, 1.0);
        let chrome_top = params.header_line_height + params.tab_line_height;
        let chrome_bottom = params.mode_line_height + params.scroll_bar_pixel_height;

        // --- Vertical scroll bar ---
        if let Some(ref side) = params.vertical_scroll_bar_side {
            let track_height = (params.bounds.height - chrome_top - chrome_bottom).max(0.0);
            if track_height <= 0.0 {
                return;
            }
            let track_width = params.scroll_bar_pixel_width;

            let x = if side == "left" {
                params.bounds.x
            } else {
                params.bounds.x + params.bounds.width - track_width
            };
            let y = params.bounds.y + chrome_top;

            let accessible_start = params.accessible_start_charpos().get();
            let accessible_end = params.accessible_end_charpos().get();
            let metrics = Self::compute_vertical_scroll_bar_metrics(
                info.window_start,
                info.window_end,
                accessible_start,
                accessible_end,
                track_height,
            );

            self.matrix_builder.push_scroll_bar(ScrollBarItem {
                window_id: params.window_id,
                row_role: GlyphRowRole::Text,
                clip_rect: Some(params.bounds),
                horizontal: false,
                x,
                y,
                width: track_width,
                height: track_height,
                position: metrics.position,
                portion: metrics.portion,
                whole: metrics.whole,
                thumb_start: metrics.thumb_start,
                thumb_size: metrics.thumb_size,
                track_color,
                thumb_color,
            });
        }

        // --- Horizontal scroll bar ---
        if params.horizontal_scroll_bar {
            let track_width = params.bounds.width;
            let track_height = params.scroll_bar_pixel_height;
            let x = params.bounds.x;
            let y = params.bounds.y + params.bounds.height
                - params.mode_line_height
                - params.scroll_bar_pixel_height;

            let hscroll_px = params.hscroll as f32 * params.char_width;
            let visible_px = params.text_bounds.width.max(1.0);
            let thumb_size = if track_width > 0.0 {
                (visible_px / (visible_px + hscroll_px + track_width)) * track_width
            } else {
                track_width
            }
            .clamp(8.0, track_width);
            let thumb_start = if track_width > 0.0 && hscroll_px + visible_px > 0.0 {
                (hscroll_px / (hscroll_px + visible_px)) * (track_width - thumb_size)
            } else {
                0.0
            };

            self.matrix_builder.push_scroll_bar(ScrollBarItem {
                window_id: params.window_id,
                row_role: GlyphRowRole::Text,
                clip_rect: Some(params.bounds),
                horizontal: true,
                x,
                y,
                width: track_width,
                height: track_height,
                position: params.hscroll as i64,
                portion: visible_px.round().max(1.0) as i64,
                whole: (visible_px + hscroll_px).round().max(1.0) as i64,
                thumb_start,
                thumb_size,
                track_color,
                thumb_color,
            });
        }
    }

    /// Compute vertical scroll bar thumb position and size.
    ///
    /// Mirrors GNU `set_vertical_scroll_bar` (xdisp.c:20109-20161):
    ///   whole = ZV - BEGV
    ///   start = window_start - BEGV
    ///   end   = Z - window_end_pos - BEGV
    ///   portion = end - start
    fn compute_vertical_scroll_bar_metrics(
        window_start: i64,
        window_end: i64,
        buffer_begv: i64,
        buffer_size: i64,
        track_height: f32,
    ) -> ScrollBarMetrics {
        let whole = (buffer_size - buffer_begv).max(1);
        let position = (window_start - 1 - buffer_begv).max(0);
        let end = if window_end > 0 {
            (window_end - 1 - buffer_begv).max(position)
        } else {
            position
        };
        let portion = (end - position).max(1);
        let effective_whole = whole.max(portion);

        let thumb_start = (position as f32 / effective_whole as f32) * track_height;
        let thumb_size = (portion as f32 / effective_whole as f32) * track_height;
        // Minimum thumb height: 20px or 20% of track, whichever is smaller.
        let min_thumb = 20.0f32.min(track_height * 0.2);
        let thumb_size = thumb_size.max(min_thumb).min(track_height);
        let thumb_start = thumb_start
            .max(0.0)
            .min((track_height - thumb_size).max(0.0));

        ScrollBarMetrics {
            position,
            portion,
            whole: effective_whole,
            thumb_start,
            thumb_size,
        }
    }

    fn push_window_divider_rects(
        &mut self,
        window_id: i64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        vertical: bool,
        frame_params: &FrameParams,
    ) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let inner = Color::from_pixel(frame_params.divider_fg);
        if (if vertical { width } else { height }) < 3.0 {
            self.matrix_builder
                .push_border(window_id, x, y, width, height, inner);
            return;
        }

        let first = Color::from_pixel(frame_params.divider_first_fg);
        let last = Color::from_pixel(frame_params.divider_last_fg);
        if vertical {
            self.matrix_builder
                .push_border(window_id, x, y, 1.0, height, first);
            self.matrix_builder.push_border(
                window_id,
                x + 1.0,
                y,
                (width - 2.0).max(0.0),
                height,
                inner,
            );
            self.matrix_builder
                .push_border(window_id, x + width - 1.0, y, 1.0, height, last);
        } else {
            self.matrix_builder
                .push_border(window_id, x, y, width, 1.0, first);
            self.matrix_builder.push_border(
                window_id,
                x,
                y + 1.0,
                width,
                (height - 2.0).max(0.0),
                inner,
            );
            self.matrix_builder
                .push_border(window_id, x, y + height - 1.0, width, 1.0, last);
        }
    }

    fn find_window_cursor_y_in_builder(
        builder: &crate::matrix_builder::GlyphMatrixBuilder,
        info: &WindowInfo,
    ) -> Option<f32> {
        let in_window = |x: f32, y: f32, hollow: bool| -> bool {
            !hollow
                && x >= info.bounds.x
                && x < info.bounds.x + info.bounds.width
                && y >= info.bounds.y
                && y < info.bounds.y + info.bounds.height
        };
        // The selected window's cursor lives in the phys cursor, not the
        // per-window CursorItem list (which now holds only non-selected windows).
        if let Some(phys) = builder.phys_cursor()
            && in_window(phys.x, phys.y, phys.style.is_hollow())
        {
            return Some(phys.y);
        }
        for cursor in builder.cursors() {
            if in_window(cursor.x, cursor.y, cursor.style.is_hollow()) {
                return Some(cursor.y);
            }
        }
        None
    }

    fn add_line_animation_hints(
        &mut self,
        curr_window_infos: &std::collections::HashMap<i64, WindowInfo>,
    ) {
        for (window_id, curr) in curr_window_infos {
            if curr.is_minibuffer {
                continue;
            }
            let Some(prev) = self.prev_window_infos.get(window_id) else {
                continue;
            };
            if prev.buffer_id == 0 || curr.buffer_id == 0 {
                continue;
            }
            if prev.buffer_id == curr.buffer_id
                && prev.window_start == curr.window_start
                && prev.buffer_size != curr.buffer_size
            {
                if let Some(edit_y) =
                    Self::find_window_cursor_y_in_builder(&self.matrix_builder, curr)
                {
                    let offset = if curr.buffer_size > prev.buffer_size {
                        -curr.char_height
                    } else {
                        curr.char_height
                    };
                    let hint = WindowEffectHint::LineAnimation {
                        window_id: curr.window_id,
                        bounds: curr.bounds,
                        edit_y: edit_y + curr.char_height,
                        offset,
                    };
                    self.matrix_builder.push_effect_hint(hint);
                }
            }
        }
    }

    fn update_window_switch_hint(&mut self) {
        let new_selected = self
            .matrix_builder
            .window_infos()
            .iter()
            .find(|info| info.selected && !info.is_minibuffer)
            .map(|info| (info.window_id, info.bounds));
        if let Some((window_id, bounds)) = new_selected {
            if self.prev_selected_window_id != 0 && self.prev_selected_window_id != window_id {
                let hint = WindowEffectHint::WindowSwitchFade { window_id, bounds };
                self.matrix_builder.push_effect_hint(hint);
            }
            self.prev_selected_window_id = window_id;
        }
    }

    fn update_theme_transition_hint(&mut self, frame_width: f32, frame_height: f32) {
        let bg = self.matrix_builder.background_color();
        let new_bg = (bg.r, bg.g, bg.b, bg.a);
        if let Some(old_bg) = self.prev_background {
            let dr = (new_bg.0 - old_bg.0).abs();
            let dg = (new_bg.1 - old_bg.1).abs();
            let db = (new_bg.2 - old_bg.2).abs();
            if dr > 0.02 || dg > 0.02 || db > 0.02 {
                let full_h = self
                    .matrix_builder
                    .window_infos()
                    .iter()
                    .find(|w| w.is_minibuffer)
                    .map_or(frame_height, |w| w.bounds.y);
                let hint = WindowEffectHint::ThemeTransition {
                    bounds: Rect::new(0.0, 0.0, frame_width, full_h),
                };
                self.matrix_builder.push_effect_hint(hint);
            }
        }
        self.prev_background = Some(new_bg);
    }

    fn maybe_add_topology_transition_hint(
        &mut self,
        frame_width: f32,
        frame_height: f32,
        curr_window_infos: &std::collections::HashMap<i64, WindowInfo>,
    ) {
        if self.prev_window_infos.is_empty() {
            return;
        }

        let prev_non_mini: std::collections::HashSet<i64> = self
            .prev_window_infos
            .iter()
            .filter(|(_, info)| !info.is_minibuffer)
            .map(|(window_id, _)| *window_id)
            .collect();
        let curr_non_mini: std::collections::HashSet<i64> = curr_window_infos
            .iter()
            .filter(|(_, info)| !info.is_minibuffer)
            .map(|(window_id, _)| *window_id)
            .collect();

        if prev_non_mini.is_empty() || curr_non_mini.is_empty() || prev_non_mini == curr_non_mini {
            return;
        }

        if self
            .matrix_builder
            .transition_hints()
            .iter()
            .any(|hint| hint.window_id == 0 && matches!(hint.kind, WindowTransitionKind::Crossfade))
        {
            return;
        }

        let full_h = self
            .matrix_builder
            .window_infos()
            .iter()
            .find(|w| w.is_minibuffer)
            .map_or(frame_height, |w| w.bounds.y);

        let hint = WindowTransitionHint {
            window_id: 0,
            bounds: Rect::new(0.0, 0.0, frame_width, full_h),
            kind: WindowTransitionKind::Crossfade,
            effect: None,
            easing: None,
        };
        self.matrix_builder.push_transition_hint(hint);
    }

    // char_advance is a standalone function (below) to avoid borrow conflicts
    // with self.text_buf

    /// Perform layout for a frame using neovm-core data (Rust-authoritative path).
    ///
    /// This is the Rust-native alternative to `layout_frame()` which reads from
    /// C struct pointers. It reads buffer text, window geometry, and buffer-local
    /// variables directly from the Context's state.
    pub fn layout_frame_rust(
        &mut self,
        evaluator: &mut neovm_core::emacs_core::Context,
        frame_id: neovm_core::window::FrameId,
    ) {
        // The font service can exist on the engine even while laying out a
        // terminal frame in tests. Match GNU's redisplay split: window-system
        // frames use realized font pixels, terminal frames stay on cell
        // metrics.

        evaluator.sync_runtime_faces_for_frame(frame_id);

        let (bootstrap_bg, bootstrap_font_size, window_system) = {
            let Some(frame) = evaluator.frame_manager().get(frame_id) else {
                tracing::error!("layout_frame_rust: frame {:?} not found", frame_id);
                return;
            };
            let bootstrap =
                super::neovm_bridge::frame_params_from_neovm(frame, evaluator.face_table());
            let ws = frame
                .effective_window_system()
                .and_then(|v| v.as_symbol_name().map(|s| s.to_string()));
            (bootstrap.background, frame.font_pixel_size, ws)
        };

        // Realize the default face before collecting window params so frame and
        // window geometry use the same default metrics GNU Emacs redisplay does.
        let face_resolver = super::neovm_bridge::FaceResolver::new_with_font_sizing(
            evaluator.face_table(),
            0x00FFFFFF,
            bootstrap_bg,
            bootstrap_font_size,
            window_system.clone(),
            self.font_sizing,
        );
        let default_resolved = face_resolver.default_face();
        let default_metrics = if window_system.is_some() {
            self.font_metrics.as_mut().map(|svc| {
                svc.font_metrics(
                    &default_resolved.font_family,
                    default_resolved.font_weight,
                    default_resolved.italic,
                    default_resolved.font_size,
                )
            })
        } else {
            None
        };

        if let Some(metrics) = default_metrics {
            if let Some(frame) = evaluator.frame_manager_mut().get_mut(frame_id) {
                frame.char_width = metrics.char_width.max(1.0);
                frame.char_height = metrics.line_height.max(1.0);
                frame.font_pixel_size = default_resolved.font_size;
            }
        } else {
            // GNU Emacs TTY frames use 1x1 character cell metrics
            // (frame.c:1184-1185: column_width=1, line_height=1).
            // Ensure char_height is never zero to prevent cosmic-text
            // assertion "line height cannot be 0".
            if let Some(frame) = evaluator.frame_manager_mut().get_mut(frame_id) {
                if frame.char_height < 1.0 {
                    frame.char_height = 1.0;
                }
                if frame.char_width < 1.0 {
                    frame.char_width = 1.0;
                }
            }
        }

        // --- Minibuffer auto-resize retry loop (GNU xdisp.c:13161-13301) ---
        //
        // After laying out all windows we check whether the minibuffer
        // used more (or fewer) display rows than its allocated height.
        // If so we call grow_mini_window / shrink_mini_window and
        // re-layout the entire frame.  The `mini_resize_attempted` flag
        // limits this to a single retry to prevent infinite loops.
        let mut mini_resize_attempted = false;
        let mut tab_bar_resize_attempted = false;

        let (frame_params, curr_window_infos) = loop {
            // Collect window and frame params from neovm-core
            let (frame_params, window_params_list) =
                match super::neovm_bridge::collect_layout_params_with_font_sizing(
                    evaluator,
                    frame_id,
                    default_metrics.map(|metrics| metrics.ascent),
                    self.font_sizing,
                ) {
                    Some(data) => data,
                    None => {
                        tracing::error!("layout_frame_rust: frame {:?} not found", frame_id);
                        return;
                    }
                };

            // --- Fontification pass ---
            // Run fontification for each window's visible region BEFORE the
            // read-only layout pass.  This triggers jit-lock / font-lock to set
            // font-lock-face text properties that the FaceResolver later reads.
            evaluator.setup_thread_locals();
            for params in &window_params_list {
                let buf_id = neovm_core::buffer::BufferId(params.buffer_id);
                let accessible_start = params.accessible_start_charpos().get();
                let accessible_end = params.accessible_end_charpos().get();
                let window_start = params.window_start_charpos().get().max(accessible_start);
                let text_height = params.bounds.height - params.mode_line_height;
                let max_rows = if params.char_height > 0.0 {
                    (text_height / params.char_height).ceil() as i64
                } else {
                    50 // fallback
                };
                // Estimate the end of the visible region (generous: 200 chars/line).
                let fontify_end = (window_start + max_rows * 200).min(accessible_end);
                Self::ensure_fontified_rust(evaluator, buf_id, window_start, fontify_end);
            }

            // Reset builder for new frame
            self.matrix_builder.reset();
            self.frame_face_id_counter = BasicFaceId::SENTINEL;
            self.pending_frame_chrome_rows.clear();
            self.pending_tab_bar = None;
            let mut curr_window_infos: std::collections::HashMap<i64, WindowInfo> =
                std::collections::HashMap::new();

            // Set up frame dimensions in the builder
            if let Some(frame) = evaluator.frame_manager().get(frame_id) {
                let (origin_x, origin_y) = evaluator
                    .frame_manager()
                    .frame_origin_in_root(frame_id)
                    .unwrap_or((frame.left_pos as f32, frame.top_pos as f32));
                self.matrix_builder.set_frame_identity(
                    frame.id.0,
                    frame.parent_frame.as_frame_id().unwrap_or(0),
                    origin_x,
                    origin_y,
                    frame.z_order,
                    frame.undecorated,
                    frame.internal_border_width() as f32,
                    Color::BLACK,
                    1.0,
                    frame.no_accept_focus,
                );
            }
            self.matrix_builder
                .set_background_color(Color::from_pixel(frame_params.background));
            self.matrix_builder
                .set_font_pixel_size(frame_params.font_pixel_size);

            // Clear hit-test data for new frame
            self.hit_data.clear();
            self.display_snapshots.clear();
            let default_resolved = face_resolver.default_face();

            insert_resolved_display_row_face(
                &mut self.matrix_builder,
                0,
                default_resolved,
                default_metrics,
            );

            let tab_bar_height = frame_params.tab_bar_height;
            if tab_bar_height > 0.0 {
                if let Some(actual_tab_bar_height) = self.render_frame_tab_bar_rust(
                    evaluator,
                    frame_id.0 as i64,
                    &face_resolver,
                    &frame_params,
                    tab_bar_height,
                ) && (actual_tab_bar_height - tab_bar_height).abs() > 0.5
                    && !tab_bar_resize_attempted
                {
                    if let Some(frame) = evaluator.frame_manager_mut().get_mut(frame_id) {
                        frame.tab_bar_height = actual_tab_bar_height.round().max(1.0) as u32;
                        frame.sync_window_area_bounds();
                    }
                    tab_bar_resize_attempted = true;
                    continue;
                }
            }

            tracing::debug!(
                "layout_frame_rust: {}x{} char={}x{} windows={}",
                frame_params.width,
                frame_params.height,
                frame_params.char_width,
                frame_params.char_height,
                window_params_list.len()
            );

            if let Some(frame) = evaluator.frame_manager_mut().get_mut(frame_id) {
                frame.begin_display_output_pass();
            }
            let main_area_bottom = window_params_list
                .iter()
                .filter(|params| !params.is_minibuffer)
                .map(|params| params.bounds.y + params.bounds.height)
                .fold(0.0_f32, f32::max);

            for params in &window_params_list {
                tracing::debug!(
                    "layout window: id={} buf={} bounds=({:.0},{:.0},{:.0},{:.0}) mini={} selected={} mode_line_h={:.0}",
                    params.window_id,
                    params.buffer_id,
                    params.bounds.x,
                    params.bounds.y,
                    params.bounds.width,
                    params.bounds.height,
                    params.is_minibuffer,
                    params.selected,
                    params.mode_line_height,
                );
                // Add window background
                self.matrix_builder
                    .push_background(params.bounds, Color::from_pixel(params.default_bg));

                // Add window info for animation detection
                let buffer_file_name = {
                    let buf_id = neovm_core::buffer::BufferId(params.buffer_id);
                    evaluator
                        .buffer_manager()
                        .get(buf_id)
                        .and_then(|b| b.file_name_runtime_string_owned())
                        .unwrap_or_default()
                };
                let modified = {
                    let buf_id = neovm_core::buffer::BufferId(params.buffer_id);
                    evaluator
                        .buffer_manager()
                        .get(buf_id)
                        .map(|b| b.is_modified())
                        .unwrap_or(false)
                };
                let window_info = neomacs_display_protocol::frame_glyphs::WindowInfo {
                    window_id: params.window_id,
                    buffer_id: params.buffer_id,
                    window_start: params.window_start,
                    window_end: 0, // filled after layout
                    buffer_size: params.buffer_size,
                    bounds: Rect::new(
                        params.bounds.x,
                        params.bounds.y,
                        params.bounds.width,
                        params.bounds.height,
                    ),
                    mode_line_height: params.mode_line_height,
                    header_line_height: params.header_line_height,
                    tab_line_height: params.tab_line_height,
                    selected: params.selected,
                    is_minibuffer: params.is_minibuffer,
                    char_height: params.char_height,
                    buffer_file_name,
                    modified,
                };
                self.matrix_builder.push_window_info(window_info);
                self.record_transition_hint_from_latest_window_info(&mut curr_window_infos);
                self.record_effect_hints_from_latest_window_info();

                let right_edge = params.bounds.x + params.bounds.width;
                let bottom_edge = params.bounds.y + params.bounds.height;
                let is_rightmost = right_edge >= frame_params.width - 1.0;
                let is_bottommost = params.is_minibuffer || bottom_edge >= main_area_bottom - 1.0;
                let reserve_right_border_col = !frame_params.window_system
                    && frame_params.right_divider_width == 0
                    && !is_rightmost
                    && !params.is_minibuffer;

                // Simplified layout for this window (no face resolution, no overlays)
                self.layout_window_rust(
                    evaluator,
                    frame_id,
                    params,
                    &frame_params,
                    &face_resolver,
                    reserve_right_border_col,
                    MAX_WINDOW_VISIBILITY_RETRIES,
                );

                // Emit scroll bar glyphs for this window.
                self.emit_window_scroll_bars(params);

                // Draw window dividers
                if !params.is_minibuffer && frame_params.right_divider_width > 0 && !is_rightmost {
                    let dw = frame_params.right_divider_width as f32;
                    let x0 = right_edge - dw;
                    let y0 = params.bounds.y;
                    let h = params.bounds.height
                        - if frame_params.bottom_divider_width > 0 && !is_bottommost {
                            frame_params.bottom_divider_width as f32
                        } else {
                            0.0
                        };
                    self.push_window_divider_rects(
                        params.window_id,
                        x0,
                        y0,
                        dw,
                        h.max(0.0),
                        true,
                        &frame_params,
                    );
                } else if !params.is_minibuffer && !is_rightmost {
                    if frame_params.window_system {
                        // GNU GUI draws a one-pixel vertical border when
                        // `right-divider-width' is zero.  The literal `|'
                        // replacement belongs to terminal frame matrices.
                        self.matrix_builder.push_border(
                            params.window_id,
                            right_edge - 1.0,
                            params.bounds.y,
                            1.0,
                            params.bounds.height.max(0.0),
                            Color::from_pixel(frame_params.vertical_border_fg),
                        );
                    } else {
                        // Mirrors GNU `src/dispnew.c::build_frame_matrix_from_leaf_window`.
                        let border_face = face_resolver.resolve_named_face("vertical-border");
                        let border_face_id = border_face.face_id;
                        let realized_face =
                            crate::display_status_line::DisplayRowFace::from_resolved(
                                border_face_id,
                                &border_face,
                            );
                        self.matrix_builder
                            .insert_face(border_face_id, realized_face.render_face());
                        self.matrix_builder
                            .overwrite_last_window_right_border('|', border_face_id);
                    }
                }

                if !params.is_minibuffer && frame_params.bottom_divider_width > 0 && !is_bottommost
                {
                    let dw = frame_params.bottom_divider_width as f32;
                    let x0 = params.bounds.x;
                    let y0 = bottom_edge - dw;
                    let w = params.bounds.width
                        - if frame_params.right_divider_width > 0 && !is_rightmost {
                            frame_params.right_divider_width as f32
                        } else {
                            0.0
                        };
                    self.push_window_divider_rects(
                        params.window_id,
                        x0,
                        y0,
                        w.max(0.0),
                        dw,
                        false,
                        &frame_params,
                    );
                }
            }

            // --- Minibuffer auto-resize check (GNU xdisp.c:13161-13301) ---
            //
            // After laying out all windows, check if the minibuffer used
            // more display rows than its allocated height. If so, grow
            // the minibuffer and re-layout the entire frame (one retry).
            // Also shrink back when the minibuffer content fits in fewer
            // rows than currently allocated.
            if !mini_resize_attempted {
                if let Some(mini_entry) = self.matrix_builder.windows().last() {
                    if let Some(mini_params) = window_params_list.last() {
                        if mini_params.is_minibuffer {
                            let mini_rows_used =
                                mini_entry.matrix.rows.iter().filter(|r| r.enabled).count();
                            let char_h = frame_params.char_height.max(1.0);
                            let allocated_rows =
                                (mini_params.bounds.height / char_h).floor().max(1.0) as usize;
                            let frame_rows = frame_params.height / char_h;
                            let max_mini_lines = max_mini_window_lines(evaluator, frame_rows);
                            let resize_policy = evaluator
                                .obarray()
                                .symbol_value("resize-mini-windows")
                                .copied();
                            let resize_mode =
                                ResizeMiniWindowsMode::from_lisp_value(resize_policy.as_ref());

                            if mini_rows_used > allocated_rows {
                                // --- Grow ---
                                let delta = (mini_rows_used as i32) - (allocated_rows as i32);

                                if resize_mode.should_grow() {
                                    tracing::debug!(
                                        "minibuffer auto-resize: grow by {} rows \
                                         (used={}, allocated={})",
                                        delta,
                                        mini_rows_used,
                                        allocated_rows,
                                    );
                                    if let Some(frame) =
                                        evaluator.frame_manager_mut().get_mut(frame_id)
                                    {
                                        frame
                                            .grow_mini_window_with_max_lines(delta, max_mini_lines);
                                    }
                                    mini_resize_attempted = true;
                                    continue; // restart the layout loop
                                }
                            } else if mini_rows_used < allocated_rows && allocated_rows > 1 {
                                // --- Shrink ---
                                // GNU `resize_mini_window` shrinks a grow-only
                                // mini-window when its buffer is empty
                                // (`BEGV == ZV`). neomacs renders echo-area
                                // messages from the evaluator's `current_message`
                                // overlay rather than from the minibuffer buffer,
                                // and leaves the idle ` *Minibuf-0*` buffer holding
                                // a blank placeholder. Treat an empty OR
                                // whitespace-only minibuffer buffer as empty so an
                                // over-allocated idle echo area shrinks back to one
                                // line; `mini_rows_used` already reflects any real
                                // (multi-line) message, so a genuine tall message
                                // is preserved by the `used < allocated` guard.
                                let buf_id = neovm_core::buffer::BufferId(mini_params.buffer_id);
                                let visible_region_empty = evaluator
                                    .buffer_manager()
                                    .get(buf_id)
                                    .map(|b| {
                                        b.buffer_substring_bytes_range(
                                            b.accessible_emacs_byte_range(),
                                        )
                                        .iter()
                                        .all(|byte| byte.is_ascii_whitespace())
                                    })
                                    .unwrap_or(true);
                                let should_shrink = resize_mode.should_shrink(visible_region_empty);

                                if should_shrink {
                                    tracing::debug!(
                                        "minibuffer auto-resize: shrink \
                                         (used={}, allocated={})",
                                        mini_rows_used,
                                        allocated_rows,
                                    );
                                    if let Some(frame) =
                                        evaluator.frame_manager_mut().get_mut(frame_id)
                                    {
                                        frame.shrink_mini_window();
                                    }
                                    mini_resize_attempted = true;
                                    continue; // restart the layout loop
                                }
                            }
                        }
                    }
                }
            }

            self.add_line_animation_hints(&curr_window_infos);
            self.update_window_switch_hint();
            self.update_theme_transition_hint(frame_params.width, frame_params.height);
            self.maybe_add_topology_transition_hint(
                frame_params.width,
                frame_params.height,
                &curr_window_infos,
            );

            break (frame_params, curr_window_infos);
        };

        // Build parallel GlyphMatrix output for validation
        let frame_cols = (frame_params.width / frame_params.char_width.max(1.0)) as usize;
        let frame_rows = (frame_params.height / frame_params.char_height.max(1.0)) as usize;
        let matrix_builder = std::mem::replace(
            &mut self.matrix_builder,
            crate::matrix_builder::GlyphMatrixBuilder::new(),
        );
        let mut frame_display_state = matrix_builder.finish_with_pixel_size(
            frame_cols,
            frame_rows,
            frame_params.char_width,
            frame_params.char_height,
            frame_params.width,
            frame_params.height,
        );
        frame_display_state
            .frame_chrome_rows
            .extend(std::mem::take(&mut self.pending_frame_chrome_rows));
        frame_display_state.tab_bar = self.pending_tab_bar.take();

        // NOTE: GlyphMatrix vs FrameGlyphBuffer character count validation removed.
        // FrameGlyphBuffer no longer receives glyph output; the GlyphMatrixBuilder
        // is now the sole output path.

        // Populate the frame-level TTY menu bar.  Mirrors GNU
        // `xdisp.c:prepare_menu_bars` -> `update_menu_bar` -> walking
        // the active maps' `[menu-bar]` prefix and stashing the result
        // in `f->menu_bar_items`.  We do the same walk via
        // `tty_menu_bar::collect_tty_menu_bar_items` and stash the
        // resulting items on the FrameDisplayState so the TTY rasterizer
        // (`tty_rif.rs`) can paint them at row 0.
        //
        // The GUI render runtime has its own menu-bar pipeline (see
        // `neomacs-display-runtime::render_thread`) and ignores this
        // field; we still populate it unconditionally because the
        // collection cost is small and any future TTY-via-display-state
        // path benefits.
        let menu_bar_lines_px = frame_params.menu_bar_height;
        let char_h = frame_params.char_height.max(1.0);
        let menu_bar_lines = (menu_bar_lines_px / char_h).round() as u16;
        if menu_bar_lines > 0 {
            let items =
                crate::tty_menu_bar::collect_tty_menu_bar_items_for_frame(evaluator, frame_id);
            // Resolve the GNU `menu` face once and pass its attributes
            // through to the TTY rasterizer.  Mirrors how
            // `display_menu_bar` (`xdisp.c:27444`) initialises its
            // iterator with `MENU_FACE_ID`: the per-cell face is the
            // `menu` face for every glyph in the menu-bar row.
            //
            // We resolve through `FaceResolver::resolve_named_face`
            // (the same path mode-line / header-line use), so any user
            // customisation of the `menu` face via `face-spec-set` is
            // honoured. The default `menu` face inherits :inverse-video
            // on TTYs, which gives the highlighted bar visible in GNU
            // Emacs `-nw`.
            let menu_face_resolver = crate::neovm_bridge::FaceResolver::new_with_font_sizing(
                evaluator.face_table(),
                0x00FFFFFF,
                0x00000000,
                frame_params.font_pixel_size,
                window_system.clone(),
                self.font_sizing,
            );
            let menu_face = menu_face_resolver.resolve_named_face("menu");
            frame_display_state.menu_bar =
                Some(neomacs_display_protocol::glyph_matrix::TtyMenuBarState {
                    items,
                    lines: menu_bar_lines,
                    fg: menu_face.fg,
                    bg: menu_face.bg,
                    use_default_foreground: menu_face.use_default_foreground,
                    use_default_background: menu_face.use_default_background,
                    bold: menu_face.font_weight >= 600,
                    inverse: menu_face.terminal_inverse_video,
                });
        }
        if frame_display_state.parent_id == 0 {
            let menu_face_resolver = crate::neovm_bridge::FaceResolver::new_with_font_sizing(
                evaluator.face_table(),
                0x00FFFFFF,
                0x00000000,
                frame_params.font_pixel_size,
                window_system.clone(),
                self.font_sizing,
            );
            let pixel_to_tuple = |pixel: u32| -> (f32, f32, f32) {
                (
                    ((pixel >> 16) & 0xFF) as f32 / 255.0,
                    ((pixel >> 8) & 0xFF) as f32 / 255.0,
                    (pixel & 0xFF) as f32 / 255.0,
                )
            };

            if frame_params.menu_bar_height > 0.0 {
                let menu_face = menu_face_resolver.resolve_named_face_without_inverse_video("menu");
                frame_display_state.gui_menu_bar =
                    Some(neomacs_display_protocol::glyph_matrix::GuiMenuBarState {
                        items: collect_gui_menu_bar_items_for_frame(evaluator, frame_id),
                        height: frame_params.menu_bar_height,
                        fg: pixel_to_tuple(menu_face.fg),
                        bg: pixel_to_tuple(menu_face.bg),
                    });
            }

            if frame_params.tool_bar_height > 0.0 {
                let tool_bar_face = menu_face_resolver.resolve_named_face("tool-bar");
                frame_display_state.gui_tool_bar =
                    Some(neomacs_display_protocol::glyph_matrix::GuiToolBarState {
                        items: collect_gui_tool_bar_items(evaluator),
                        height: frame_params.tool_bar_height,
                        fg: pixel_to_tuple(tool_bar_face.fg),
                        bg: pixel_to_tuple(tool_bar_face.bg),
                    });
            }

            if frame_params.compact_bar_height > 0.0 {
                let menu_face = menu_face_resolver.resolve_named_face_without_inverse_video("menu");
                let tool_bar_face = menu_face_resolver.resolve_named_face("tool-bar");
                frame_display_state.gui_compact_bar =
                    Some(neomacs_display_protocol::glyph_matrix::GuiCompactBarState {
                        menu_items: collect_gui_menu_bar_items_for_frame(evaluator, frame_id),
                        tool_items: collect_gui_tool_bar_items(evaluator),
                        height: frame_params.compact_bar_height,
                        menu_fg: pixel_to_tuple(menu_face.fg),
                        menu_bg: pixel_to_tuple(menu_face.bg),
                        tool_fg: pixel_to_tuple(tool_bar_face.fg),
                        tool_bg: pixel_to_tuple(tool_bar_face.bg),
                    });
            }
        }

        self.last_frame_display_state = Some(frame_display_state);
        self.prev_window_infos = curr_window_infos;

        let snapshots = std::mem::take(&mut self.display_snapshots);
        if let Some(frame) = evaluator.frame_manager_mut().get_mut(frame_id) {
            frame.set_display_snapshots(snapshots);
        }
        unsafe {
            *std::ptr::addr_of_mut!(FRAME_HIT_DATA) = Some(std::mem::take(&mut self.hit_data));
        }
    }

    /// Simplified window layout using neovm-core data.
    ///
    /// Renders buffer text as a monospace grid with face resolution.
    /// Queries FontMetricsService for per-face character metrics when available.
    /// Note: fontification (jit-lock / font-lock) is triggered by
    /// `layout_frame_rust()` before this function is called, so text
    /// properties are already up-to-date when we read them here.
    fn layout_window_rust(
        &mut self,
        evaluator: &mut neovm_core::emacs_core::Context,
        frame_id: neovm_core::window::FrameId,
        params: &WindowParams,
        frame_params: &FrameParams,
        face_resolver: &super::neovm_bridge::FaceResolver,
        reserve_right_border_col: bool,
        remaining_visibility_retries: usize,
    ) {
        let buf_id = neovm_core::buffer::BufferId(params.buffer_id);
        let window_id = neovm_core::window::WindowId(params.window_id as u64);
        let layout_buffer = match evaluator.buffer_manager().get(buf_id) {
            Some(buffer) => super::neovm_bridge::LayoutBufferSnapshot::from_buffer_with_obarray(
                buffer,
                evaluator.obarray(),
            ),
            None => {
                tracing::debug!("layout_window_rust: buffer {} not found", params.buffer_id);
                return;
            }
        };
        let buffer = &layout_buffer;

        // Capture buffer name as owned String for use in mode-line fallback.
        // This avoids holding a borrow on `evaluator` through eval calls.
        let buffer_name = buffer.name().to_owned();
        let accessible_end_lisp_char = buffer.accessible_end_char_pos().get().saturating_add(1);
        let accessible_end_emacs_byte = buffer.accessible_end_emacs_byte_pos().get();

        let buf_access = super::neovm_bridge::RustBufferAccess::new(buffer);
        if let Some(effects) = params.cursor_effects.clone() {
            self.matrix_builder
                .set_window_cursor_effects(params.window_id, effects);
        }

        let char_w = params.char_width;
        let char_h = params.char_height;
        let font_ascent = params.font_ascent;
        let active_minibuffer_window =
            evaluator.minibuffer_window_is_active(WindowId(params.window_id as u64));
        let echo_message = minibuffer_echo_message_for_window(
            params.is_minibuffer,
            active_minibuffer_window,
            evaluator.current_message_value(),
        );

        // Line number configuration from buffer-local variables
        let lnum_mode = super::neovm_bridge::buffer_display_line_numbers_mode(buffer).engine_code();
        let lnum_enabled = lnum_mode > 0;
        let lnum_offset =
            super::neovm_bridge::buffer_local_int(buffer, "display-line-numbers-offset", 0);
        let lnum_major_tick =
            super::neovm_bridge::buffer_local_int(buffer, "display-line-numbers-major-tick", 0)
                as i32;
        let _lnum_minor_tick =
            super::neovm_bridge::buffer_local_int(buffer, "display-line-numbers-minor-tick", 0)
                as i32;
        let lnum_current_absolute =
            super::neovm_bridge::buffer_local_bool(buffer, "display-line-numbers-current-absolute");
        let lnum_widen =
            super::neovm_bridge::buffer_local_bool(buffer, "display-line-numbers-widen");
        let lnum_min_width =
            super::neovm_bridge::buffer_local_int(buffer, "display-line-numbers-width", 0) as i32;

        // Selective display: integer N = hide lines with > N indent + CR hides rest of line;
        // t (True) = only CR hides rest of line (mapped to i32::MAX so indent check never triggers)
        let selective_display = super::neovm_bridge::buffer_selective_display(buffer);

        // Line/wrap prefix: keep Lisp string values so display-time prefixes
        // retain text properties while moving through the shared row builder.
        let line_prefix_value = super::neovm_bridge::buffer_local_value(buffer, "line-prefix")
            .filter(|value| value.as_lisp_string().is_some());
        let wrap_prefix_value = super::neovm_bridge::buffer_local_value(buffer, "wrap-prefix")
            .filter(|value| value.as_lisp_string().is_some());
        let has_prefix = line_prefix_value.is_some() || wrap_prefix_value.is_some();

        // Use face_resolver's default face for this window.
        // Chrome row reservation must use the same realized face metrics as
        // the final status-line renderer, otherwise rows drift from GNU
        // redisplay when faces override font size, ascent, or box widths.
        let default_resolved = face_resolver.default_face();
        let default_fg = Color::from_pixel(default_resolved.fg);

        let (default_face_char_w, default_face_h, default_face_ascent) = if frame_params
            .window_system
            && let Some(ref mut svc) = self.font_metrics
        {
            let m = svc.font_metrics(
                &default_resolved.font_family,
                default_resolved.font_weight,
                default_resolved.italic,
                default_resolved.font_size,
            );
            (m.char_width, m.line_height, m.ascent)
        } else {
            (char_w, char_h, font_ascent)
        };

        tracing::debug!(
            "layout font metrics: family={:?} weight={} italic={} size={} char_w={:.2} char_h={:.2} ascent={:.2} (window char_w={:.2} char_h={:.2})",
            default_resolved.font_family,
            default_resolved.font_weight,
            default_resolved.italic,
            default_resolved.font_size,
            default_face_char_w,
            default_face_h,
            default_face_ascent,
            char_w,
            char_h,
        );

        let mode_line_face = if params.mode_line_height > 0.0 {
            Some(face_resolver.resolve_named_face(if params.selected {
                "mode-line-active"
            } else {
                "mode-line-inactive"
            }))
        } else {
            None
        };
        let header_line_face = if params.header_line_height > 0.0 {
            Some(face_resolver.resolve_named_face(if params.selected {
                "header-line-active"
            } else {
                "header-line-inactive"
            }))
        } else {
            None
        };
        let tab_line_face = if params.tab_line_height > 0.0 {
            Some(face_resolver.resolve_named_face("tab-line"))
        } else {
            None
        };

        let mode_line_height = mode_line_face.as_ref().map_or(0.0, |face| {
            self.display_row_height_for_face(face, char_w, default_face_ascent, default_face_h)
        });
        let header_line_height = header_line_face.as_ref().map_or(0.0, |face| {
            self.display_row_height_for_face(face, char_w, default_face_ascent, default_face_h)
        });
        let tab_line_height = tab_line_face.as_ref().map_or(0.0, |face| {
            self.display_row_height_for_face(face, char_w, default_face_ascent, default_face_h)
        });
        let top_chrome_rows =
            usize::from(tab_line_height > 0.0) + usize::from(header_line_height > 0.0);

        let text_x = params.text_bounds.x;
        let text_y = params.text_bounds.y + header_line_height + tab_line_height;
        let text_width = params.text_bounds.width;
        let text_height =
            params.bounds.height - mode_line_height - header_line_height - tab_line_height;

        // In Emacs, w->vscroll is negative when content is shifted up.
        let vscroll = (-params.vscroll).max(0) as f32;
        let text_height = (text_height - vscroll).max(0.0);

        let max_rows = (text_height / char_h).floor() as usize;

        // Compute line number column width.  GNU's
        // `maybe_produce_line_number' reserves `lnum_width + 2` columns: the
        // right-aligned number plus one blank on each side.  `lnum_width` is
        // wide enough for the largest line number that can appear in the
        // current window, so a tiny buffer in a tall window still gets the
        // same two-digit gutter GNU displays for visible rows 1..N.
        let lnum_cols = if lnum_enabled {
            let total_lines = buf_access.count_lines(0, buf_access.zv()) + 1;
            let visible_lines = max_rows.max(1) as i64;
            let digit_count = total_lines.max(visible_lines).max(1).to_string().len() as i32;
            let min = lnum_min_width.max(1);
            digit_count.max(min) + 2
        } else {
            0
        };
        let lnum_pixel_width = lnum_cols as f32 * char_w;

        // The minibuffer must always render at least 1 row.  Its pixel
        // height may be fractionally smaller than char_h (e.g. 24px vs
        // 24.15 with line-spacing) causing floor() to yield 0.
        // Exception: when vscroll is active, don't force 1 row -- vscroll
        // is used (e.g. by vertico-posframe) to intentionally hide content.
        let max_rows =
            if params.is_minibuffer && max_rows == 0 && text_height > 0.0 && vscroll == 0.0 {
                1
            } else {
                max_rows
            };
        // GNU `resize_mini_window` (`xdisp.c:13161-13301`) pre-
        // grows the minibuffer BEFORE layout by running
        // `move_it_to` to walk ALL content (buffer text + overlay
        // strings) and measuring the resulting pixel height.
        //
        // neomacs approximation: count `\n` in the buffer text plus
        // resize-relevant overlay strings to estimate the display line
        // count.  GNU redisplay can render zero-length EOB overlay
        // strings (see `overlay_strings' in buffer.c and
        // `load_overlay_strings' in xdisp.c), but `resize_mini_window'
        // does not grow the parent minibuffer for a zero-length EOB
        // `before-string'.  Pre-expand max_rows to the matching count
        // (clamped to max-mini-window-height = 25% of frame). This avoids
        // the boot-time "tall echo area" bug (single-line content stays
        // at 1 row) while allowing fido/vertico multi-line overlays that
        // GNU counts during mini-window resize to render.
        let max_rows = if params.is_minibuffer {
            let buf_id = neovm_core::buffer::BufferId(params.buffer_id);
            let content_lines = evaluator
                .buffer_manager()
                .get(buf_id)
                .map(|buffer| minibuffer_resize_line_count(buffer, params.window_id as u64))
                .unwrap_or(1);
            let frame_rows = frame_params.height / char_h;
            let max_mini = max_mini_window_lines(evaluator, frame_rows).ceil() as usize;
            content_lines.clamp(1, max_mini)
        } else {
            max_rows
        };
        let text_matrix_row_base = top_chrome_rows;
        let text_matrix_rows = max_rows.max(1);
        let bottom_chrome_rows = usize::from(mode_line_height > 0.0);
        let mode_line_matrix_row = text_matrix_row_base + text_matrix_rows;
        let cols = ((text_width - lnum_pixel_width) / char_w).floor() as usize;
        let content_x = text_x + lnum_pixel_width;

        let requested_window_start = params.window_start_charpos().get();
        let previous_window_end = params.previous_window_end_charpos().map(|pos| pos.get());
        let point_charpos = params.point_charpos().get();
        let accessible_start = params.accessible_start_charpos().get();
        let accessible_end = params.accessible_end_charpos().get();

        // Read buffer text starting from window_start.
        // Auto-adjust window_start when point is above the visible region.
        let window_start = {
            let mut ws = requested_window_start.max(accessible_start);
            // GNU Emacs xdisp.c: if window-start is beyond the buffer content
            // that can fill the window, scroll back to show meaningful content.
            // This happens after buffer deletions that shrink the buffer below
            // the previous window-start.
            if ws > accessible_start {
                let remaining_chars = accessible_end - ws;
                if remaining_chars < max_rows as i64 && accessible_end > max_rows as i64 {
                    // Not enough content after ws to fill the window.
                    // Recenter around point.
                    let target_rows_above = (max_rows / 2).max(1) as i64;
                    let mut lines_back: i64 = 0;
                    let mut scan_pos = point_charpos.max(accessible_start);
                    while scan_pos > accessible_start && lines_back < target_rows_above {
                        scan_pos -= 1;
                        let bp = buf_access.charpos_to_bytepos(scan_pos);
                        if buf_access.byte_at(bp) == Some(b'\n') {
                            lines_back += 1;
                        }
                    }
                    ws = scan_pos.max(accessible_start);
                }
            }
            if point_charpos >= accessible_start && point_charpos < ws {
                // Point is above the visible region: scroll backward.
                // Target: show point about 25% of the way down from the top.
                let target_rows_above = (max_rows / 4).max(1) as i64;
                let mut lines_back: i64 = 0;
                let mut scan_pos = point_charpos;
                // Scan backward through buffer text counting newlines
                while scan_pos > accessible_start && lines_back < target_rows_above {
                    scan_pos -= 1;
                    let bp = buf_access.charpos_to_bytepos(scan_pos);
                    if buf_access.byte_at(bp) == Some(b'\n') {
                        lines_back += 1;
                    }
                }
                ws = scan_pos.max(accessible_start);
                tracing::debug!(
                    "layout_window_rust: adjusted window_start {} -> {} (point={})",
                    requested_window_start,
                    ws,
                    point_charpos
                );
            } else if point_charpos > 0 && !params.is_minibuffer && {
                // Forward-scroll trigger: either
                //   (a) we have a previous window_end and
                //       point is past it (standard
                //       scroll-below-previous case), or
                //   (b) we have no previous window_end (first
                //       layout after construction) and point
                //       is far enough past window_start that
                //       a first-pass layout starting from ws
                //       could not plausibly reach it.
                //
                // Case (b) handles the
                // `converges_visibility_for_wrapped_rows` and
                // `retries_window_when_point_starts_below_visible_span`
                // tests, which construct a fresh window with
                // window_start=1 and point far below, and
                // expect layout_frame_rust to publish geometry
                // that includes point without a second
                // redisplay pass.
                let has_prev_end = previous_window_end.is_some_and(|end| point_charpos > end);
                let max_visible_chars =
                    (max_rows.max(1) as i64) * (params.bounds.width.max(1.0) as i64);
                let far_below_without_prev_end =
                    previous_window_end.is_none() && point_charpos - ws > max_visible_chars;
                has_prev_end || far_below_without_prev_end
            } {
                // Mirror GNU/legacy forward scroll: when point moved below the
                // previous visible end, choose a new start before layout so the
                // current redisplay already includes point.
                let target_rows_above = ((max_rows * 3) / 4).max(1) as i64;
                let mut lines_back: i64 = 0;
                let mut scan_pos = point_charpos;
                while scan_pos > accessible_start && lines_back < target_rows_above {
                    scan_pos -= 1;
                    let bp = buf_access.charpos_to_bytepos(scan_pos);
                    if buf_access.byte_at(bp) == Some(b'\n') {
                        lines_back += 1;
                    }
                }
                ws = scan_pos.max(accessible_start);
                tracing::debug!(
                    "layout_window_rust: forward-adjusted window_start {} -> {} (point={}, prev_end={})",
                    requested_window_start,
                    ws,
                    point_charpos,
                    previous_window_end.unwrap_or(0)
                );
            }
            ws
        };
        // GNU Emacs redisplay advances iterators until the visible window is
        // fully resolved; it does not stop at an arbitrary "rows * cols"
        // character budget.  Capping the text slice here truncates long
        // wrapped or truncated lines before they are actually offscreen, which
        // breaks both redisplay and geometry queries.
        let read_chars = accessible_end - window_start + 1;

        let text_start_byte = buf_access.charpos_to_bytepos(window_start) as usize;
        let bytes_read = if read_chars <= 0 {
            0i64
        } else {
            let text_end = (window_start + read_chars).min(accessible_end);
            let byte_to = buf_access.charpos_to_bytepos(text_end);
            buf_access.copy_text(text_start_byte as i64, byte_to, &mut self.text_buf);
            self.text_buf.len() as i64
        };

        let text = if bytes_read > 0 {
            &self.text_buf[..bytes_read as usize]
        } else {
            &[]
        };
        let transition_hints_len_before = self.matrix_builder.transition_hints().len();
        let effect_hints_len_before = self.matrix_builder.effect_hints().len();

        tracing::debug!(
            "  layout_window_rust id={}: text_y={:.1} text_h={:.1} max_rows={} bytes_read={}",
            params.window_id,
            text_y,
            text_height,
            max_rows,
            bytes_read
        );

        if text_height <= 0.0 || text_width <= 0.0 {
            return;
        }

        let default_fallback_metrics = DisplayRowFallbackMetrics::from_default_face_extents(
            default_face_char_w,
            default_face_h,
            default_face_ascent,
        );
        // Face resolution state
        let mut face_next_check: usize = 0;
        // Load the frame-wide face-id counter so this window's
        // glyph/mode-line/header-line faces get IDs that do NOT
        // collide with earlier siblings' faces in the frame-scoped
        // `matrix_builder.faces` HashMap. Write back below before
        // returning. Mirrors GNU's single `face_cache->used`
        // counter per frame at `src/xfaces.c::lookup_face` /
        // `init_frame_faces`.
        let mut current_face_id: u32 = self.frame_face_id_counter.max(BasicFaceId::SENTINEL);
        let measurement_policy = DisplayRowMeasurementPolicy::for_frame(frame_params.window_system);

        let default_measured_face = measurement_policy.measured_face(
            BasicFaceId::Default.into(),
            default_resolved,
            None,
            char_w,
            default_fallback_metrics,
            &mut self.font_metrics,
        );
        let mut active_face_state =
            DisplayRowActiveFaceState::new(default_resolved.clone(), default_measured_face);
        // Per-face metrics — start with defaults, updated on face change.
        let mut face_metrics = active_face_state.metrics();
        let default_face_state = active_face_state.clone();

        if let Some(echo_message) = echo_message {
            // GNU `display_echo_area_1` displays the current message by
            // temporarily making the echo-area buffer current, calling
            // `resize_mini_window`, then redisplaying the minibuffer window.
            // GNU measures the displayed height, not just literal newlines:
            // a long one-line message grows the echo area when
            // `message-truncate-lines' is nil.
            let reserve_right_special_col =
                !frame_params.window_system && params.right_fringe_width == 0.0;
            let truncate_echo_lines = message_truncate_lines(evaluator);
            let frame_rows = frame_params.height / char_h;
            let max_mini = max_mini_window_lines(evaluator, frame_rows).ceil().max(1.0) as usize;
            let rows = self.render_minibuffer_echo_rows(
                params.bounds.y,
                text_width,
                char_w,
                default_face_ascent,
                char_h,
                default_resolved,
                face_resolver,
                evaluator.display_host.as_deref(),
                echo_message,
                max_mini,
                truncate_echo_lines,
                reserve_right_special_col,
                &mut current_face_id,
            );
            let max_rows_echo = rows.len().clamp(1, max_mini);
            let cols_echo = (text_width / char_w).ceil().max(1.0) as usize;
            self.matrix_builder.begin_window_with_text_bounds(
                params.window_id as u64,
                max_rows_echo,
                cols_echo,
                params.bounds,
                params.text_bounds,
                params.selected,
            );
            for (row_index, rendered) in rows.iter().enumerate() {
                install_rendered_display_row(&mut self.matrix_builder, rendered, row_index);
            }
            self.matrix_builder.end_window();
            return;
        }

        if params.is_minibuffer && !active_minibuffer_window {
            // GNU `display_echo_area` temporarily displays an echo-area
            // buffer in the minibuffer window.  With no current message that
            // buffer is empty; the inactive minibuffer must not redisplay the
            // ordinary buffer attached to the window record.
            let cols = (text_width / char_w).ceil().max(1.0) as usize;
            self.matrix_builder.begin_window_with_text_bounds(
                params.window_id as u64,
                1,
                cols,
                params.bounds,
                params.text_bounds,
                params.selected,
            );
            let row_spec = DisplayRowSpec::from_base_face(
                DisplayRowGeometry {
                    y: params.bounds.y,
                    width: text_width,
                    height: char_h,
                    char_width: char_w,
                    ascent: default_face_ascent,
                    tab_policy: DisplayTabPolicy::every(8),
                },
                &mut current_face_id,
                default_resolved,
                GlyphRowRole::Minibuffer,
                std::collections::HashMap::new(),
            );
            let rendered = self
                .render_lisp_string_row_with_display_host(
                    row_spec,
                    Value::string(""),
                    face_resolver,
                    evaluator.display_host.as_deref(),
                    &mut current_face_id,
                )
                .expect("empty Lisp string should render an inactive minibuffer row");
            install_rendered_display_row(&mut self.matrix_builder, &rendered, 0);
            self.matrix_builder.end_window();
            return;
        }

        // Line number state
        let window_start_byte = buf_access.charpos_to_bytepos(window_start);
        let begin_byte = if lnum_widen { 0 } else { buf_access.begv() };
        let mut current_line: i64 = if lnum_enabled {
            buf_access.count_lines(begin_byte, window_start_byte) + 1
        } else {
            1
        };
        let point_line: i64 = if lnum_enabled && lnum_mode >= 2 {
            let pt_byte = buf_access.charpos_to_bytepos(point_charpos);
            buf_access.count_lines(begin_byte, pt_byte) + 1
        } else {
            0
        };
        let mut need_line_number = lnum_enabled;

        // Simple monospace text layout
        let mut x = content_x;
        let mut y = text_y;
        let mut row = 0usize;
        let mut col = 0usize;
        let mut byte_idx = 0usize;
        let mut charpos = window_start;
        let mut invis_next_check: i64 = window_start; // Next position where visibility might change
        let mut display_next_check: i64 = window_start; // Next position where display props might change

        // Display :raise property: vertical Y offset for glyphs
        let mut raise_y_offset: f32 = 0.0;
        let mut raise_end: i64 = window_start;

        // Display :height property: font scale factor applied as a real face
        // transformation, matching GNU `face_with_height`.
        let mut height_factor: Option<f32> = None;
        let mut height_end: i64 = window_start;

        // Fringe state tracking
        let left_fringe_x = params.text_bounds.x - params.left_fringe_width;
        let right_fringe_x = params.text_bounds.x + params.text_bounds.width;
        let mut row_continued = vec![false; max_rows];
        let mut row_truncated = vec![false; max_rows];
        let mut row_continuation = vec![false; max_rows];

        // Horizontal scroll: skip first hscroll columns on each line
        let hscroll = if params.truncate_lines {
            params.hscroll.max(0) as i32
        } else {
            0
        };
        let show_left_trunc = hscroll > 0;
        let mut hscroll_remaining = hscroll;

        // Word-wrap break tracking
        let mut wrap_break_byte_idx = 0usize;
        let mut wrap_break_charpos = window_start;
        let mut _wrap_break_x: f32 = 0.0;
        let mut _wrap_break_col = 0usize;
        let mut wrap_break_display_point_count = 0usize;
        let mut wrap_break_row_first_display_pos: Option<LispCharPos1> = None;
        let mut wrap_break_row_last_display_pos: Option<LispCharPos1> = None;
        let mut wrap_has_break = false;
        let mut word_wrap_may_wrap = false;

        // Line/wrap prefix tracking: 0=none, 1=line-prefix, 2=wrap-prefix
        let mut need_prefix: u8 = if has_prefix && line_prefix_value.is_some() {
            1
        } else {
            0
        };

        let reserve_right_border_width = if reserve_right_border_col {
            char_w
        } else {
            0.0
        };
        let reserve_right_special_col =
            !frame_params.window_system && params.right_fringe_width == 0.0;
        let reserve_right_special_width = if reserve_right_special_col {
            char_w
        } else {
            0.0
        };
        let avail_width = (text_width
            - lnum_pixel_width
            - reserve_right_border_width
            - reserve_right_special_width)
            .max(char_w);
        let text_append_surface = DisplayRowAppendSurface::new(
            DisplayRowAppendArea {
                content_x,
                width: avail_width,
                text_width,
                line_number_width: lnum_pixel_width,
            },
            text_display_tab_policy(content_x, params),
        );

        // Variable-height row tracking
        let mut row_max_height: f32 = char_h; // max glyph height on current row
        let mut row_max_ascent: f32 = default_face_ascent; // max ascent on current row
        let mut row_extra_y: f32 = 0.0; // cumulative extra height from previous rows
        let mut row_y_positions =
            DisplayRowYPositions::with_capacity_and_first_row(max_rows, text_y);
        macro_rules! current_row_geometry {
            () => {
                DisplayRowGeometryState {
                    row,
                    y,
                    row_extra_y,
                    height: row_max_height,
                    ascent: row_max_ascent,
                }
            };
        }
        macro_rules! current_row_geometry_vars {
            () => {
                LegacyDisplayRowGeometryVars::new(
                    &mut row,
                    &mut y,
                    &mut row_extra_y,
                    &mut row_max_height,
                    &mut row_max_ascent,
                )
            };
        }
        // Trailing whitespace tracking
        let trailing_ws_bg = if params.show_trailing_whitespace {
            Some(Color::from_pixel(params.trailing_ws_bg))
        } else {
            None
        };
        let mut trailing_ws_start_col: i32 = -1; // -1 = no trailing ws
        let mut trailing_ws_start_x: f32 = 0.0;
        let mut trailing_ws_row: usize = 0;
        // Exact joined-form advances for the current contextual-shaping run,
        // shaped once via shape_run and keyed by absolute byte offset (robust
        // to wrap re-processing). Empty/unused for non-complex text.
        let mut complex_run_adv: Vec<DisplayTextRunByteAdvance> = Vec::new();
        let mut complex_run_start: usize = usize::MAX;
        let mut complex_run_end: usize = 0;

        // Check if the buffer has any overlays (optimization: skip per-char overlay checks if empty)
        let has_overlays = !buffer.overlays().is_empty();

        // Face :extend tracking — extends face background to end of line
        let mut row_extend_bg: Option<(Color, u32)> = None; // (bg_color, face_id)
        let mut row_extend_row: i32 = -1;

        // Box face tracking: track active :box face regions
        let mut box_active = false;
        let mut box_start_x: f32 = 0.0;
        let mut box_row: usize = 0;

        // Cursor metrics captured during the main layout loop.
        let mut cursor_info: Option<CapturedCursorInfo> = None;

        // Hit-test data for this window
        let mut hit_rows: Vec<HitRow> = Vec::new();
        let mut hit_row_charpos_start: i64 = window_start;
        let text_area_left = text_x;
        let window_top = params.bounds.y;
        let mut output_emitter = WindowOutputEmitter::new(
            frame_id,
            window_id,
            text_matrix_row_base,
            text_area_left,
            window_top,
        );
        output_emitter.begin_update(evaluator);
        let sync_charpos_from_byte_idx = |byte_idx: usize| {
            buf_access.bytepos_to_charpos(text_start_byte as i64 + byte_idx as i64)
        };

        let ligatures = self.ligatures_enabled;
        self.run_buf.clear();

        // Margin state tracking
        let has_margins = params.left_margin_width > 0.0 || params.right_margin_width > 0.0;

        // Clear margin backgrounds with default face background so they don't
        // show visual artifacts.  Default Emacs layout (fringes-outside-margins
        // nil): | LEFT_MARGIN | LEFT_FRINGE | TEXT_AREA | RIGHT_FRINGE | RIGHT_MARGIN |
        // So left margin is outermost (before fringe), right margin is outermost
        // (after fringe).
        if has_margins {
            if params.left_margin_width > 0.0 {
                let _margin_x = text_x - params.left_fringe_width - params.left_margin_width;
            }
            if params.right_margin_width > 0.0 {
                let _margin_x = text_x + text_width + params.right_fringe_width;
            }
        }

        macro_rules! resolve_current_face_state {
            () => {
                if (charpos as usize) >= face_next_check {
                    flush_run(&self.run_buf, ligatures);
                    self.run_buf.clear();
                    let mut resolved =
                        face_resolver.face_at_pos(buffer, charpos as usize, &mut face_next_check);
                    if let Some(factor) = height_factor
                        && let Some(adjusted) = height_adjusted_face(
                            &resolved,
                            DisplayHeightFaceBasis {
                                canonical_face: default_resolved,
                                base_face: default_resolved,
                                fallback_char_width: default_face_char_w,
                                fallback_ascent: default_face_ascent,
                                fallback_row_height: default_face_h,
                            },
                            factor,
                        )
                    {
                        resolved = adjusted;
                    }
                    let face_id = current_face_id;

                    let metrics = if frame_params.window_system {
                        self.font_metrics.as_mut().map(|svc| {
                            svc.font_metrics(
                                &resolved.font_family,
                                resolved.font_weight,
                                resolved.italic,
                                resolved.font_size,
                            )
                        })
                    } else {
                        None
                    };
                    let resolved_measured_face = measurement_policy.resolved_measured_face(
                        face_id,
                        resolved.clone(),
                        metrics,
                        char_w,
                        DisplayRowFallbackMetrics::from_default_face_extents(
                            char_w,
                            char_h,
                            font_ascent,
                        ),
                        &mut self.font_metrics,
                    );
                    resolved_measured_face.install_into(&mut self.matrix_builder);
                    active_face_state = resolved_measured_face.into_active_face_state();
                    face_metrics = active_face_state.metrics();
                    current_row_geometry_vars!()
                        .include_row_extents(face_metrics.row_height, face_metrics.ascent);

                    current_face_id += 1;

                    if resolved.extend {
                        let ext_bg = Color::from_pixel(resolved.bg);
                        row_extend_bg = Some((ext_bg, face_id));
                        row_extend_row = row as i32;
                    }

                    if box_active && resolved.box_type == 0 {
                        box_active = false;
                    }
                    if resolved.box_type > 0 {
                        box_active = true;
                        box_start_x = x;
                        box_row = row;
                    }
                }
            };
        }

        macro_rules! save_word_wrap_candidate {
            ($ch:expr, $break_byte_idx:expr) => {
                if params.word_wrap && word_wrap_may_wrap && char_can_wrap_before_basic($ch) {
                    flush_run(&self.run_buf, ligatures);
                    self.run_buf.clear();
                    wrap_break_byte_idx = $break_byte_idx;
                    wrap_break_charpos = charpos;
                    wrap_break_display_point_count = output_emitter.display_point_len();
                    (
                        wrap_break_row_first_display_pos,
                        wrap_break_row_last_display_pos,
                    ) = output_emitter.current_row_display_positions();
                    wrap_has_break = true;
                }
            };
        }

        // --- GlyphMatrix builder: begin window and first row ---
        let matrix_rows = text_matrix_row_base + text_matrix_rows + bottom_chrome_rows;
        let matrix_cols = cols.max(1);
        self.matrix_builder.begin_window_with_text_bounds(
            params.window_id as u64,
            matrix_rows,
            matrix_cols,
            params.bounds,
            params.text_bounds,
            params.selected,
        );
        TextMatrixRowOutput::new(&mut self.matrix_builder, &mut output_emitter, evaluator)
            .begin(current_row_geometry!().text_matrix_row_begin(text_matrix_row_base, col, x));

        let row_visibility_limit = DisplayRowVisibilityLimit {
            max_rows,
            bottom_y: text_y + text_height,
        };
        let row_geometry_defaults =
            DisplayRowGeometryDefaults::new(text_y, char_h, default_face_ascent);

        while byte_idx < text.len()
            && current_row_geometry_vars!().current_row_is_visible(row_visibility_limit)
        {
            // Render line number at start of each visual line
            if need_line_number && lnum_enabled {
                let display_num = match lnum_mode {
                    2 | 3 => {
                        // Relative/visual mode
                        if lnum_current_absolute && current_line == point_line {
                            (current_line + lnum_offset).abs()
                        } else {
                            (current_line - point_line).abs()
                        }
                    }
                    _ => {
                        // Absolute mode
                        (current_line + lnum_offset).abs()
                    }
                };

                // Resolve line number face
                let is_current = current_line == point_line;
                let lnum_face = if is_current {
                    face_resolver.resolve_named_face("line-number-current-line")
                } else if lnum_major_tick > 0 && current_line % lnum_major_tick as i64 == 0 {
                    face_resolver.resolve_named_face("line-number-major-tick")
                } else {
                    face_resolver.resolve_named_face("line-number")
                };
                let _lnum_bg = Color::from_pixel(lnum_face.bg);
                // Realize and register the line-number face so the renderer
                // uses the same family/weight/slant the layout chose.
                insert_resolved_display_row_face(
                    &mut self.matrix_builder,
                    current_face_id,
                    &lnum_face,
                    None,
                );
                let lnum_face_id = current_face_id;
                current_face_id += 1;

                // Format number right-aligned
                let num_str = format!("{}", display_num);
                let num_chars = num_str.len() as i32;
                let padding = (lnum_cols - 1) - num_chars; // -1 for trailing space

                let _gy = y;

                // Leading padding (stretch)
                if padding > 0 {
                    self.matrix_builder
                        .push_left_margin_stretch(padding as u16, lnum_face_id);
                }

                // Number digits
                for (i, ch) in num_str.chars().enumerate() {
                    let _dx = text_x + (padding.max(0) + i as i32) as f32 * char_w;
                    self.matrix_builder.push_left_margin_char(ch, lnum_face_id);
                }

                // Trailing space separator
                let _space_x = text_x + (lnum_cols - 1) as f32 * char_w;
                self.matrix_builder
                    .push_left_margin_stretch(1, lnum_face_id);

                // Force face resolution to re-apply text face after line number face
                face_next_check = 0;

                need_line_number = false;
            }

            // --- Line/wrap prefix rendering ---
            if need_prefix > 0 {
                // Check text property prefix first (overrides buffer-local)
                let text_props = super::neovm_bridge::RustTextPropAccess::new(buffer);
                let prefix = if need_prefix == 2 {
                    text_props
                        .get_property(charpos, Value::symbol("wrap-prefix"))
                        .filter(|value| value.as_lisp_string().is_some())
                        .or(wrap_prefix_value)
                } else {
                    text_props
                        .get_property(charpos, Value::symbol("line-prefix"))
                        .filter(|value| value.as_lisp_string().is_some())
                        .or(line_prefix_value)
                };

                if let Some(prefix_value) = prefix {
                    // Flush ligature run before prefix
                    flush_run(&self.run_buf, ligatures);
                    self.run_buf.clear();
                    let prefix_fragment = if need_prefix == 2 {
                        DisplayTextFragment::wrap_prefix(
                            prefix_value,
                            CharPos0::new(charpos as usize),
                        )
                    } else {
                        DisplayTextFragment::line_prefix(
                            prefix_value,
                            CharPos0::new(charpos as usize),
                        )
                    };
                    let prefix_base_face = display_string_base_face(
                        buffer,
                        face_resolver,
                        prefix_fragment.origin,
                        prefix_fragment.base_face_policy,
                        &mut current_face_id,
                        &mut self.matrix_builder,
                    );

                    let append_frame = text_append_surface.frame_for_active_face(
                        current_row_geometry!().append_placement(raise_y_offset),
                        &active_face_state,
                        char_h,
                    );
                    let position = append_lisp_string_fragment_to_text_row_and_emit(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                        &mut self.font_metrics,
                        prefix_fragment,
                        2,
                        face_resolver,
                        &prefix_base_face.face,
                        prefix_base_face.face_id,
                        &mut current_face_id,
                        append_frame,
                        DisplayRowPosition { x_px: x, col },
                    );
                    x = position.x_px;
                    col = position.col;
                }
                need_prefix = 0;
            }

            // --- Invisible text check ---
            // Only call check_invisible at property change boundaries for efficiency
            if charpos >= invis_next_check {
                let text_props = super::neovm_bridge::RustTextPropAccess::new(buffer);
                let (invisible, next_visible) = text_props.check_invisible(charpos);
                if invisible.hidden {
                    let skip_to = next_visible.min(accessible_end);
                    let point_in_hidden_region = cursor_info.is_none()
                        && point_charpos >= charpos
                        && point_charpos < skip_to;
                    if point_in_hidden_region {
                        capture_cursor_info(
                            &mut cursor_info,
                            CapturedCursorInfo::from_active_face_state(
                                &active_face_state,
                                CapturedCursorPlacement {
                                    x,
                                    y,
                                    byte_idx,
                                    col,
                                    matrix_row: row,
                                    slot_width: CapturedCursorSlotWidth::FaceChar,
                                    stretch_like: false,
                                },
                            ),
                        );
                    }

                    skip_text_to_charpos(text, &mut byte_idx, &mut charpos, skip_to);
                    invis_next_check = next_visible;

                    // GNU displays ellipsis only when the matching
                    // `buffer-invisibility-spec' entry requests it.
                    if invisible.ellipsis {
                        flush_run(&self.run_buf, ligatures);
                        self.run_buf.clear();

                        let ellipsis_frame = text_append_surface.frame_for_active_face(
                            current_row_geometry!().append_placement(raise_y_offset),
                            &active_face_state,
                            char_h,
                        );
                        let measurement =
                            active_face_state.text_run_measurement(&mut self.font_metrics, "...");
                        if let Some((_progress, position)) = append_synthetic_text_to_display_row(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            evaluator,
                            face_resolver,
                            active_face_state.resolved_face(),
                            ellipsis_frame,
                            DisplayRowPosition { x_px: x, col },
                            SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS,
                            "...",
                            active_face_state.face_id(),
                            Some(measurement),
                        ) {
                            x = position.x_px;
                            col = position.col;
                        }
                    }

                    // Check for overlay strings at invisible region boundary.
                    // Packages like org-mode use overlay after-strings at invisible
                    // boundaries to show fold indicators (e.g. "[N lines]").
                    if has_overlays {
                        let invis_text_props =
                            super::neovm_bridge::RustTextPropAccess::new_for_window(
                                buffer,
                                params.window_id as u64,
                            );
                        let (_before_strings, after_strings) =
                            invis_text_props.overlay_strings_at(charpos);
                        if !after_strings.is_empty() {
                            flush_run(&self.run_buf, ligatures);
                            self.run_buf.clear();
                            let right_limit = content_x + avail_width;
                            for overlay_string in &after_strings {
                                current_row_geometry_vars!().with_display_row_geometry_state(
                                    |geometry| {
                                        render_overlay_string(
                                            evaluator,
                                            &mut output_emitter,
                                            buffer,
                                            DisplayTextFragment::overlay_string(
                                                overlay_string.string,
                                                overlay_string.overlay_id,
                                                CharPos0::new(charpos as usize),
                                                OverlayStringKind::After,
                                            ),
                                            &mut self.font_metrics,
                                            face_resolver,
                                            &mut x,
                                            &mut col,
                                            geometry,
                                            &mut cursor_info,
                                            &mut hit_rows,
                                            &mut hit_row_charpos_start,
                                            charpos,
                                            &mut row_y_positions,
                                            face_metrics.char_width,
                                            char_h,
                                            default_face_ascent,
                                            right_limit,
                                            content_x,
                                            text_y,
                                            text_matrix_row_base,
                                            max_rows,
                                            &mut current_face_id,
                                            &mut self.matrix_builder,
                                            params,
                                        );
                                    },
                                );
                            }
                        }
                    }

                    flush_run(&self.run_buf, ligatures);
                    self.run_buf.clear();
                    continue;
                }
                invis_next_check = next_visible;
            }

            // Handle hscroll: skip columns consumed by horizontal scroll
            if hscroll_remaining > 0 {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
                let ch_start_byte_idx = byte_idx;
                let (ch, ch_len) = decode_utf8(&text[byte_idx..]);
                byte_idx += ch_len;
                charpos += 1;

                if ch == '\n' {
                    x = content_x;
                    // Record newline position on the row (see main \n handler).
                    output_emitter.note_display_buffer_pos(LispCharPos1::new(charpos));
                    row_extend_bg = None;
                    row_extend_row = -1;

                    let geometry_transition = current_row_geometry_vars!()
                        .finish_boundary_and_record_hit(
                            DisplayRowBoundaryTarget::line_break(
                                DisplayRowHitRange {
                                    charpos_start: hit_row_charpos_start,
                                    charpos_end: charpos,
                                },
                                row_geometry_defaults,
                                text_matrix_row_base,
                                col,
                                x,
                                0.0,
                                row_y_positions.recording(),
                            ),
                            &mut hit_rows,
                        );
                    // Record hit-test row (hscroll newline)
                    hit_row_charpos_start = charpos;
                    let row_transition = TextMatrixRowOutput::new(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_with_row_limit(geometry_transition, max_rows);
                    if row_transition.is_exhausted() {
                        break;
                    }
                    col = 0;
                    current_line += 1;
                    need_line_number = lnum_enabled;
                    hscroll_remaining = hscroll; // reset for next line
                    trailing_ws_start_col = -1;
                    if has_prefix {
                        need_prefix = 1;
                    }
                    if cursor_info.is_none() && point_charpos == charpos {
                        capture_cursor_info(
                            &mut cursor_info,
                            CapturedCursorInfo::line_break_from_active_face_state(
                                &active_face_state,
                                CapturedCursorPlacement {
                                    x,
                                    y,
                                    byte_idx: ch_start_byte_idx,
                                    col,
                                    matrix_row: row,
                                    slot_width: CapturedCursorSlotWidth::FaceChar,
                                    stretch_like: false,
                                },
                                char_h,
                            ),
                        );
                    }
                } else {
                    let ch_cols: i32 = if ch == '\t' {
                        let tab_w = params.tab_width.max(1) as i32;
                        let consumed = hscroll - hscroll_remaining;
                        ((consumed / tab_w + 1) * tab_w) - consumed
                    } else if is_wide_char(ch) {
                        2
                    } else {
                        1
                    };
                    hscroll_remaining -= ch_cols.min(hscroll_remaining);

                    // When hscroll is exhausted, show $ indicator at left edge
                    if hscroll_remaining <= 0 && show_left_trunc {
                        let trunc_face_id: u32 = BasicFaceId::Default.into();
                        let trunc_frame = text_append_surface.frame(
                            current_row_geometry!().append_placement(0.0),
                            DisplayRowAppendMetrics {
                                height: char_h,
                                ascent: default_face_ascent,
                                char_width: char_w,
                                space_width: char_w,
                                default_row_height: char_h,
                            },
                        );
                        let measurement =
                            default_face_state.text_run_measurement(&mut self.font_metrics, "$");
                        if let Some((_progress, position)) = append_synthetic_text_to_display_row(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            evaluator,
                            face_resolver,
                            face_resolver.default_face(),
                            trunc_frame,
                            DisplayRowPosition {
                                x_px: content_x,
                                col: 0,
                            },
                            SYNTHETIC_SOURCE_HSCROLL_TRUNCATION,
                            "$",
                            trunc_face_id,
                            Some(measurement),
                        ) {
                            x = position.x_px;
                            col = position.col;
                        }
                        self.matrix_builder.with_current_row_mut(|glyph_row| {
                            glyph_row.truncated_left = true;
                        });
                    }
                    if cursor_info.is_none() && point_charpos == charpos {
                        capture_cursor_info(
                            &mut cursor_info,
                            CapturedCursorInfo::from_active_face_state(
                                &active_face_state,
                                CapturedCursorPlacement {
                                    x,
                                    y,
                                    byte_idx: ch_start_byte_idx,
                                    col,
                                    matrix_row: row,
                                    slot_width: CapturedCursorSlotWidth::FaceChar,
                                    stretch_like: false,
                                },
                            ),
                        );
                    }
                }
                continue;
            }

            // --- Display property check ---
            // Only call check_display_prop at property change boundaries for efficiency
            if height_end > window_start && charpos >= height_end {
                height_factor = None;
                height_end = window_start;
                face_next_check = 0;
            }
            resolve_current_face_state!();
            if charpos >= display_next_check {
                let display_prop_val: Option<neovm_core::emacs_core::Value> = {
                    let text_props = super::neovm_bridge::RustTextPropAccess::new(buffer);
                    let (dp, next_change) = text_props.check_display_prop(charpos);
                    display_next_check = next_change;
                    dp
                };

                if let Some(prop_val) = display_prop_val {
                    flush_run(&self.run_buf, ligatures);
                    self.run_buf.clear();
                    let skip_to = display_next_check.min(accessible_end);
                    let point_in_display_replacement = cursor_info.is_none()
                        && point_charpos >= charpos
                        && point_charpos < skip_to;
                    let display_property = classify_display_property(prop_val);
                    // Case 1: String replacement — render the string instead of buffer text
                    if matches!(
                        display_property.replacement,
                        Some(DisplayReplacementProperty::String)
                    ) && let Some(replacement) = prop_val.as_utf8_str()
                    {
                        if point_in_display_replacement {
                            let slot_width = replacement
                                .chars()
                                .next()
                                .map(|rch| {
                                    active_face_state.advance_for_char(
                                        &mut self.font_metrics,
                                        rch,
                                        face_metrics.char_width,
                                    )
                                })
                                .unwrap_or_else(|| char_w.max(1.0));
                            capture_cursor_info(
                                &mut cursor_info,
                                CapturedCursorInfo::from_active_face_state(
                                    &active_face_state,
                                    CapturedCursorPlacement {
                                        x,
                                        y,
                                        byte_idx,
                                        col,
                                        matrix_row: row,
                                        slot_width: CapturedCursorSlotWidth::Explicit(slot_width),
                                        stretch_like: false,
                                    },
                                ),
                            );
                        }
                        if !replacement.is_empty() {
                            let right_limit = content_x + (text_width - lnum_pixel_width);
                            let replacement_string_surface = DisplayRowAppendSurface::new(
                                DisplayRowAppendArea {
                                    content_x,
                                    width: right_limit - content_x,
                                    text_width,
                                    line_number_width: lnum_pixel_width,
                                },
                                text_display_tab_policy(content_x, params),
                            );
                            let replacement_source = BufferDisplayReplacementSource::new(
                                buf_id,
                                charpos,
                                text_start_byte + byte_idx,
                            );
                            let replacement_fragment = DisplayTextFragment::display_property_string(
                                prop_val,
                                CharPos0::new(charpos as usize),
                                DisplayPropertySource::TextProperty,
                            );
                            let mut replacement_next_check =
                                buffer.layout_point_max_char_pos().get();
                            let replacement_base_face = face_resolver.base_face_for_origin(
                                Some(buffer),
                                &replacement_fragment.origin,
                                replacement_fragment.base_face_policy,
                                &mut replacement_next_check,
                            );
                            let replacement_base_face_id =
                                if crate::display_source_resolver::same_resolved_face(
                                    &replacement_base_face,
                                    active_face_state.resolved_face(),
                                ) {
                                    active_face_state.face_id()
                                } else if crate::display_source_resolver::same_resolved_face(
                                    &replacement_base_face,
                                    face_resolver.default_face(),
                                ) {
                                    u32::from(neomacs_display_protocol::face::BasicFaceId::Default)
                                } else {
                                    let face_id = current_face_id;
                                    current_face_id += 1;
                                    insert_resolved_display_row_face(
                                        &mut self.matrix_builder,
                                        face_id,
                                        &replacement_base_face,
                                        None,
                                    );
                                    face_id
                                };
                            let DisplayTextStorage::LispString(replacement_value) =
                                replacement_fragment.storage
                            else {
                                continue;
                            };
                            if let Some(source) = crate::display_source::LispStringSourceCursor::new(
                                1,
                                replacement_value,
                                crate::display_item::RenderFaceRef::FaceId(
                                    replacement_base_face_id,
                                ),
                            ) {
                                let source = BufferDisplayReplacementStringSource::new(
                                    replacement_source,
                                    source,
                                );
                                let append_frame = replacement_string_surface
                                    .frame_for_active_face(
                                        current_row_geometry!().append_placement(raise_y_offset),
                                        &active_face_state,
                                        char_h,
                                    );
                                let mut item_measurer =
                                    ReplacementStringItemMeasurer::from_active_face_state(
                                        &mut self.font_metrics,
                                        &active_face_state,
                                    );
                                let position = append_display_replacement_string_source_to_text_row(
                                    &mut self.matrix_builder,
                                    &mut output_emitter,
                                    evaluator,
                                    source,
                                    face_resolver,
                                    &replacement_base_face,
                                    replacement_base_face_id,
                                    &mut current_face_id,
                                    append_frame,
                                    DisplayRowPosition { x_px: x, col },
                                    &mut item_measurer,
                                );
                                x = position.x_px;
                                col = position.col;
                            }
                        }

                        // Skip the buffer text that this display property covers
                        skip_text_to_charpos(text, &mut byte_idx, &mut charpos, skip_to);
                        continue;
                    }

                    // Case 2: Space spec — (space :width …) or (space :align-to …)
                    if matches!(
                        display_property.replacement,
                        Some(DisplayReplacementProperty::Space(_))
                    ) {
                        let (display_ch, _) = decode_utf8(&text[byte_idx..]);
                        let display_char_width = active_face_state.advance_for_char(
                            &mut self.font_metrics,
                            display_ch,
                            face_metrics.char_width,
                        );
                        let space_geometry = eval_display_space_geometry(
                            &prop_val,
                            x,
                            content_x,
                            face_metrics.char_width,
                            display_char_width,
                            face_metrics.row_height,
                            face_metrics.ascent,
                            params,
                        );
                        let space_width = space_geometry.width;
                        if point_in_display_replacement {
                            capture_cursor_info(
                                &mut cursor_info,
                                CapturedCursorInfo::from_active_face_state(
                                    &active_face_state,
                                    CapturedCursorPlacement {
                                        x,
                                        y,
                                        byte_idx,
                                        col,
                                        matrix_row: row,
                                        slot_width: CapturedCursorSlotWidth::Explicit(
                                            space_width.max(face_metrics.char_width),
                                        ),
                                        stretch_like: true,
                                    },
                                ),
                            );
                        }
                        if space_width > 0.0 {
                            let _bg = Color::from_pixel(default_resolved.bg);
                            current_row_geometry_vars!().include_glyph_vertical_metrics(
                                space_geometry.height,
                                space_geometry.ascent,
                            );
                            let replacement_source = BufferDisplayReplacementSource::new(
                                buf_id,
                                charpos,
                                text_start_byte + byte_idx,
                            );
                            let item = replacement_source.stretch_item(
                                active_face_state.face_id(),
                                DisplayReplacementBox::new(
                                    space_width,
                                    space_geometry.height,
                                    space_geometry.ascent,
                                ),
                            );
                            let replacement_frame = text_append_surface.frame_for_active_face(
                                current_row_geometry!().append_placement(raise_y_offset),
                                &active_face_state,
                                char_h,
                            );
                            if let Some((_progress, position)) =
                                append_display_replacement_item_to_text_row_and_emit(
                                    &mut self.matrix_builder,
                                    &mut output_emitter,
                                    evaluator,
                                    item,
                                    face_resolver,
                                    active_face_state.resolved_face(),
                                    active_face_state.face_id(),
                                    replacement_frame,
                                    DisplayRowPosition { x_px: x, col },
                                )
                            {
                                x = position.x_px;
                                col = position.col;
                            }
                        }

                        // Skip covered buffer text
                        skip_text_to_charpos(text, &mut byte_idx, &mut charpos, skip_to);
                        continue;
                    }

                    // Case 3: media specs — direct xwidget specs already carry a
                    // media item; image/video/webkit resolve through the display
                    // host and keep TTY placeholders when unresolved.
                    if let Some(replacement) = display_property.replacement.as_ref()
                        && replacement.is_media_replacement()
                    {
                        let replacement_source = BufferDisplayReplacementSource::new(
                            buf_id,
                            charpos,
                            text_start_byte + byte_idx,
                        );
                        let maybe_media_item = replacement.direct_media_item_kind().or_else(|| {
                            resolve_display_property_media(
                                &prop_val,
                                evaluator.display_host.as_deref(),
                                active_face_state.resolved_face(),
                                face_metrics.char_width,
                                face_metrics.row_height,
                            )
                            .filter(|kind| replacement.accepts_resolved_media_item(kind))
                        });

                        if let Some(media_item) = maybe_media_item {
                            let media =
                                crate::display_item::DisplayMediaReplacement::from_item_kind(
                                    &media_item,
                                )
                                .expect("resolved media item should have media geometry");
                            let display_width = media.width;
                            let display_height = media.height;
                            let (cursor_face_h, cursor_face_ascent) =
                                if matches!(replacement, DisplayReplacementProperty::Xwidget(_)) {
                                    (
                                        display_height.max(face_metrics.row_height),
                                        display_height.max(face_metrics.ascent),
                                    )
                                } else {
                                    (display_height, display_height)
                                };

                            if point_in_display_replacement {
                                capture_cursor_info(
                                    &mut cursor_info,
                                    CapturedCursorInfo::display_box_from_active_face_state(
                                        &active_face_state,
                                        CapturedCursorPlacement {
                                            x,
                                            y,
                                            byte_idx,
                                            col,
                                            matrix_row: row,
                                            slot_width: CapturedCursorSlotWidth::Explicit(
                                                display_width,
                                            ),
                                            stretch_like: false,
                                        },
                                        cursor_face_h,
                                        cursor_face_ascent,
                                    ),
                                );
                            }

                            let replacement_frame = text_append_surface.frame(
                                current_row_geometry!().append_placement(raise_y_offset),
                                DisplayRowAppendMetrics::display_box_from_active_face_state(
                                    &active_face_state,
                                    display_height,
                                    display_height,
                                    char_h,
                                ),
                            );
                            let item =
                                replacement_source.item(active_face_state.face_id(), media_item);
                            if let Some((progress, position)) =
                                append_display_replacement_item_to_text_row_and_emit(
                                    &mut self.matrix_builder,
                                    &mut output_emitter,
                                    evaluator,
                                    item,
                                    face_resolver,
                                    active_face_state.resolved_face(),
                                    active_face_state.face_id(),
                                    replacement_frame,
                                    DisplayRowPosition { x_px: x, col },
                                )
                                && progress.status
                                    == crate::display_row_builder::DisplayRowAppendStatus::Complete
                                && progress.metrics.width_px > 0.0
                            {
                                current_row_geometry_vars!()
                                    .include_row_extents(display_height, display_height);
                                x = position.x_px;
                                col = position.col;
                            }
                        } else if let Some(placeholder) = replacement.media_fallback_placeholder() {
                            if point_in_display_replacement {
                                capture_cursor_info(
                                    &mut cursor_info,
                                    CapturedCursorInfo::from_active_face_state(
                                        &active_face_state,
                                        CapturedCursorPlacement {
                                            x,
                                            y,
                                            byte_idx,
                                            col,
                                            matrix_row: row,
                                            slot_width: CapturedCursorSlotWidth::FaceChar,
                                            stretch_like: false,
                                        },
                                    ),
                                );
                            }
                            let replacement_frame = text_append_surface.frame_for_active_face(
                                current_row_geometry!().append_placement(raise_y_offset),
                                &active_face_state,
                                char_h,
                            );
                            let item = replacement_source
                                .source_mapped_text_item(active_face_state.face_id(), placeholder);
                            if let Some((_progress, position)) =
                                append_display_replacement_item_to_text_row_and_emit(
                                    &mut self.matrix_builder,
                                    &mut output_emitter,
                                    evaluator,
                                    item,
                                    face_resolver,
                                    active_face_state.resolved_face(),
                                    active_face_state.face_id(),
                                    replacement_frame,
                                    DisplayRowPosition { x_px: x, col },
                                )
                            {
                                x = position.x_px;
                                col = position.col;
                            }
                        }

                        // Skip covered buffer text
                        skip_text_to_charpos(text, &mut byte_idx, &mut charpos, skip_to);
                        continue;
                    }

                    // Case 4: Raise — (raise FACTOR) or plist with :raise
                    if let Some(factor) = display_property.modifiers.raise {
                        raise_y_offset = -(factor * char_h);
                        raise_end = display_next_check;
                    }

                    // Case 5: Height — (height FACTOR) or plist with :height
                    if let Some(factor) = display_property.modifiers.height {
                        if factor.is_finite() && factor > 0.0 {
                            height_factor = Some(factor);
                            height_end = display_next_check;
                            face_next_check = 0;
                            resolve_current_face_state!();
                        }
                    }
                    // Other display property types: fall through to normal rendering
                }
            }

            // Decode UTF-8 character. Keep the original byte/char position so
            // character-wrap can resume from the same buffer position on the
            // next visual row, like GNU Emacs restoring its iterator state.
            let ch_start_byte_idx = byte_idx;
            let _ch_start_charpos = charpos;
            let ch = match std::str::from_utf8(&text[byte_idx..]) {
                Ok(s) => {
                    let ch = s.chars().next().unwrap_or('\u{FFFD}');
                    byte_idx += ch.len_utf8();
                    ch
                }
                Err(e) => {
                    // Partial valid UTF-8: try decoding from the valid prefix
                    let valid_up_to = e.valid_up_to();
                    if valid_up_to > 0 {
                        if let Ok(s) = std::str::from_utf8(&text[byte_idx..byte_idx + valid_up_to])
                        {
                            let ch = s.chars().next().unwrap_or('\u{FFFD}');
                            byte_idx += ch.len_utf8();
                            ch
                        } else {
                            byte_idx += 1;
                            '\u{FFFD}'
                        }
                    } else {
                        byte_idx += 1;
                        '\u{FFFD}'
                    }
                }
            };

            // Selective display: \r hides rest of line until \n
            if selective_display > 0 && ch == '\r' {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
                let ellipsis_frame = text_append_surface.frame_for_active_face(
                    current_row_geometry!().append_placement(raise_y_offset),
                    &active_face_state,
                    char_h,
                );
                let measurement =
                    active_face_state.text_run_measurement(&mut self.font_metrics, "...");
                if let Some((_progress, position)) = append_synthetic_text_to_display_row(
                    &mut self.matrix_builder,
                    &mut output_emitter,
                    evaluator,
                    face_resolver,
                    active_face_state.resolved_face(),
                    ellipsis_frame,
                    DisplayRowPosition { x_px: x, col },
                    SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS,
                    "...",
                    active_face_state.face_id(),
                    Some(measurement),
                ) {
                    x = position.x_px;
                    col = position.col;
                }
                // Skip remaining chars until newline
                charpos += 1;
                while byte_idx < text.len() {
                    let (skip_ch, skip_len) = decode_utf8(&text[byte_idx..]);
                    byte_idx += skip_len;
                    charpos += 1;
                    if skip_ch == '\n' {
                        // Advance to next row (same as newline handler)
                        x = content_x;
                        row_extend_bg = None;
                        row_extend_row = -1;
                        if box_active {
                            box_start_x = content_x;
                            box_row = row + 1;
                        }
                        let geometry_transition = current_row_geometry_vars!()
                            .finish_boundary_and_record_hit(
                                DisplayRowBoundaryTarget::line_break(
                                    DisplayRowHitRange {
                                        charpos_start: hit_row_charpos_start,
                                        charpos_end: charpos,
                                    },
                                    row_geometry_defaults,
                                    text_matrix_row_base,
                                    col,
                                    x,
                                    0.0,
                                    row_y_positions.recording(),
                                ),
                                &mut hit_rows,
                            );
                        let row_transition = TextMatrixRowOutput::new(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            evaluator,
                        )
                        .emit_with_row_limit(geometry_transition, max_rows);
                        if row_transition.is_exhausted() {
                            break;
                        }
                        charpos = sync_charpos_from_byte_idx(byte_idx);
                        hit_row_charpos_start = charpos;
                        col = 0;
                        current_line += 1;
                        need_line_number = lnum_enabled;
                        hscroll_remaining = hscroll;
                        word_wrap_may_wrap = false;
                        wrap_has_break = false;
                        trailing_ws_start_col = -1;
                        if has_prefix {
                            need_prefix = 1;
                        }
                        break;
                    }
                }
                continue;
            }

            save_word_wrap_candidate!(ch, ch_start_byte_idx);

            if ch == '\n' {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
                if cursor_info.is_none() && point_charpos == charpos {
                    // GNU `set_cursor_from_row` treats the terminating
                    // newline as an exact match for point on this row.  The
                    // newline itself has no rendered text glyph, so the
                    // physical cursor uses the row-end cell width instead of
                    // waiting for the next row.
                    capture_cursor_info(
                        &mut cursor_info,
                        CapturedCursorInfo::from_active_face_state(
                            &active_face_state,
                            CapturedCursorPlacement {
                                x,
                                y,
                                byte_idx: ch_start_byte_idx,
                                col,
                                matrix_row: row,
                                slot_width: CapturedCursorSlotWidth::FaceChar,
                                stretch_like: false,
                            },
                        ),
                    );
                }
                // Highlight trailing whitespace before advancing to next row
                if let Some(_tw_bg) = trailing_ws_bg {
                    if trailing_ws_start_col >= 0 && trailing_ws_row == row {
                        let tw_x = trailing_ws_start_x;
                        let tw_w = x - tw_x;
                        if tw_w > 0.0 {}
                    }
                }
                trailing_ws_start_col = -1;

                // Face :extend: fill rest of row with extending face background
                if let Some((_ext_bg, _ext_face_id)) = row_extend_bg {
                    if row_extend_row == row as i32 {
                        let right_edge = content_x + avail_width;
                        if x < right_edge {}
                    }
                }
                row_extend_bg = None;
                row_extend_row = -1;

                // Box face tracking: box stays active across line breaks
                if box_active {
                    box_start_x = content_x;
                }

                charpos += 1;

                // Check line-spacing text property on the newline we just consumed.
                // Text property overrides buffer-local line-spacing for that line.
                let text_prop_spacing = {
                    let nl_pos = charpos - 1; // the newline char
                    let text_props = super::neovm_bridge::RustTextPropAccess::new(buffer);
                    text_props.check_line_spacing(nl_pos, char_h)
                };
                let line_spacing = if text_prop_spacing > 0.0 {
                    text_prop_spacing
                } else if params.extra_line_spacing > 0.0 {
                    params.extra_line_spacing
                } else {
                    0.0
                };
                x = content_x;
                // Record the newline position so the row's
                // end_buffer_pos includes it. GNU's redisplay engine
                // counts newlines as part of the row they terminate,
                // so window-end reflects the position AFTER the last
                // newline. Without this, trailing empty rows have
                // end_buffer_pos=None and window-end falls short of
                // point-max, causing %p to show "Top" instead of "All".
                output_emitter.note_display_buffer_pos(LispCharPos1::new(charpos));
                // Record hit-test row (newline ends the row)
                let geometry_transition = current_row_geometry_vars!()
                    .finish_boundary_and_record_hit(
                        DisplayRowBoundaryTarget::line_break(
                            DisplayRowHitRange {
                                charpos_start: hit_row_charpos_start,
                                charpos_end: charpos,
                            },
                            row_geometry_defaults,
                            text_matrix_row_base,
                            col,
                            x,
                            line_spacing,
                            row_y_positions.recording(),
                        ),
                        &mut hit_rows,
                    );
                let row_transition = TextMatrixRowOutput::new(
                    &mut self.matrix_builder,
                    &mut output_emitter,
                    evaluator,
                )
                .emit_with_row_limit(geometry_transition, max_rows);
                if row_transition.is_exhausted() {
                    break;
                }
                charpos = sync_charpos_from_byte_idx(byte_idx);
                hit_row_charpos_start = charpos;
                if box_active {
                    box_row = row;
                }
                col = 0;
                current_line += 1;
                need_line_number = lnum_enabled;
                hscroll_remaining = hscroll;
                word_wrap_may_wrap = false;
                wrap_has_break = false;
                if has_prefix {
                    need_prefix = 1;
                }
                // Selective display: skip lines indented beyond threshold
                if selective_display > 0 && selective_display < i32::MAX && byte_idx < text.len() {
                    let mut shown_ellipsis = false;
                    loop {
                        if byte_idx >= text.len() {
                            break;
                        }
                        // Peek at indentation of next line
                        let mut indent = 0i32;
                        let mut peek = byte_idx;
                        while peek < text.len() {
                            let b = text[peek];
                            if b == b' ' {
                                indent += 1;
                                peek += 1;
                            } else if b == b'\t' {
                                let tab_w = params.tab_width.max(1) as i32;
                                indent = ((indent / tab_w) + 1) * tab_w;
                                peek += 1;
                            } else {
                                break;
                            }
                        }
                        if indent > selective_display {
                            // Show ... ellipsis once for the hidden block
                            if !shown_ellipsis && row > 0 {
                                let _prev_row_y = row_y_positions.y_for_row(
                                    row - 1,
                                    DisplayRowYFallback {
                                        text_y,
                                        default_height: char_h,
                                        row_extra_y: 0.0,
                                    },
                                );
                                for dot_i in 0..3 {
                                    let dot_x = content_x + dot_i as f32 * face_metrics.char_width;
                                    if dot_x + face_metrics.char_width <= content_x + avail_width {}
                                }
                                shown_ellipsis = true;
                            }
                            // Skip this hidden line
                            while byte_idx < text.len() {
                                let (skip_ch, skip_len) = decode_utf8(&text[byte_idx..]);
                                byte_idx += skip_len;
                                charpos += 1;
                                if skip_ch == '\n' {
                                    current_line += 1;
                                    break;
                                }
                            }
                        } else {
                            break; // Next line is visible
                        }
                    }
                }
                continue;
            }

            // Control characters: render as ^X notation
            if (ch < ' ' && ch != '\t') || ch == '\x7F' {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
                let needed_width = 2.0 * face_metrics.char_width;

                // Check if we have room for ^X (2 columns)
                if x + needed_width > content_x + (text_width - lnum_pixel_width) {
                    // Doesn't fit — wrap or truncate
                    if params.truncate_lines {
                        if row < max_rows {
                            row_truncated[row] = true;
                        }
                        // Same byte_idx/charpos desync as the main-char
                        // truncation path: byte_idx is past the overflowing
                        // control char, but charpos hasn't been incremented
                        // for it yet. Compensate before skipping.
                        charpos += 1;
                        if skip_to_newline(text, &mut byte_idx, &mut charpos) {
                            current_line += 1;
                            need_line_number = lnum_enabled;
                        }
                        x = content_x;
                        row_extend_bg = None;
                        row_extend_row = -1;
                        let geometry_transition = current_row_geometry_vars!()
                            .finish_boundary_and_record_hit(
                                DisplayRowBoundaryTarget::truncation(
                                    DisplayRowHitRange {
                                        charpos_start: hit_row_charpos_start,
                                        charpos_end: charpos,
                                    },
                                    row_geometry_defaults,
                                    text_matrix_row_base,
                                    col,
                                    x,
                                    row_y_positions.recording(),
                                ),
                                &mut hit_rows,
                            );
                        // Record hit-test row (wrap/truncation break)
                        let row_transition = TextMatrixRowOutput::new(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            evaluator,
                        )
                        .emit_with_row_limit(geometry_transition, max_rows);
                        if row_transition.is_exhausted() {
                            break;
                        }
                        charpos = sync_charpos_from_byte_idx(byte_idx);
                        hit_row_charpos_start = charpos;
                        col = 0;
                        word_wrap_may_wrap = false;
                        trailing_ws_start_col = -1;
                        if has_prefix {
                            need_prefix = 1;
                        }
                        continue;
                    } else {
                        if row < max_rows {
                            row_continued[row] = true;
                        }
                        x = content_x;
                        row_extend_bg = None;
                        row_extend_row = -1;
                        let geometry_transition = current_row_geometry_vars!()
                            .finish_boundary_and_record_hit(
                                DisplayRowBoundaryTarget::visual_wrap(
                                    DisplayRowHitRange {
                                        charpos_start: hit_row_charpos_start,
                                        charpos_end: charpos,
                                    },
                                    row_geometry_defaults,
                                    text_matrix_row_base,
                                    col,
                                    x,
                                    row_y_positions.recording(),
                                ),
                                &mut hit_rows,
                            );
                        // Record hit-test row (wrap/truncation break)
                        hit_row_charpos_start = charpos;
                        let row_transition = TextMatrixRowOutput::new(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            evaluator,
                        )
                        .emit_with_row_limit(geometry_transition, max_rows);
                        if row_transition.is_exhausted() {
                            break;
                        }
                        col = 0;
                        trailing_ws_start_col = -1;
                        if row < max_rows {
                            row_continuation[row] = true;
                        }
                        if has_prefix {
                            need_prefix = 2;
                        }
                        if !current_row_geometry_vars!()
                            .current_row_is_visible(row_visibility_limit)
                        {
                            break;
                        }
                    }
                }

                // Render ^X with escape-glyph face color
                if params.escape_glyph_fg != 0 {
                    current_face_id += 1;
                }
                let buffer_text_fragment = DisplayTextFragment::buffer_text(
                    CharPos0::new(charpos as usize),
                    CharPos0::new((charpos + 1) as usize),
                );
                let text_item_frame = text_append_surface.frame_for_active_face(
                    current_row_geometry!().append_placement(raise_y_offset),
                    &active_face_state,
                    char_h,
                );
                if let Some((_progress, position)) =
                    append_buffer_text_item_fragment_to_text_row_and_emit(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                        buffer_text_fragment,
                        buffer,
                        buf_id,
                        face_resolver,
                        active_face_state.resolved_face(),
                        active_face_state.face_id(),
                        crate::display_item::DisplayItemKind::ControlChar { ch },
                        text_item_frame,
                        DisplayRowPosition { x_px: x, col },
                    )
                {
                    x = position.x_px;
                    col = position.col;
                }
                charpos += 1;
                word_wrap_may_wrap = false;
                face_next_check = 0; // force face re-check to restore text face
                continue;
            }

            // Nobreak character display (U+00A0 non-breaking space, U+00AD soft hyphen)
            if params.nobreak_char_display > 0 && (ch == '\u{00A0}' || ch == '\u{00AD}') {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
                let mapped_text = match params.nobreak_char_display {
                    1 => Some(if ch == '\u{00A0}' { " " } else { "-" }),
                    2 => Some(if ch == '\u{00A0}' { "\\ " } else { "\\-" }),
                    _ => None,
                };
                if let Some(mapped_text) = mapped_text {
                    if params.nobreak_char_fg != 0 {
                        let _nb_fg = Color::from_pixel(params.nobreak_char_fg);
                        current_face_id += 1;
                    }
                    let buffer_text_fragment = DisplayTextFragment::buffer_text(
                        CharPos0::new(charpos as usize),
                        CharPos0::new((charpos + 1) as usize),
                    );
                    let text_item_frame = text_append_surface.frame_for_active_face(
                        current_row_geometry!().append_placement(raise_y_offset),
                        &active_face_state,
                        char_h,
                    );
                    if let Some((_progress, position)) =
                        append_buffer_text_item_fragment_to_text_row_and_emit(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            evaluator,
                            buffer_text_fragment,
                            buffer,
                            buf_id,
                            face_resolver,
                            active_face_state.resolved_face(),
                            active_face_state.face_id(),
                            crate::display_item::DisplayItemKind::SourceMappedText(
                                crate::display_item::DisplaySourceMappedText::new(mapped_text),
                            ),
                            text_item_frame,
                            DisplayRowPosition { x_px: x, col },
                        )
                    {
                        x = position.x_px;
                        col = position.col;
                    }
                    charpos += 1;
                    word_wrap_may_wrap = false;
                    face_next_check = 0;
                    continue;
                }
            }
            // Grapheme-cluster continuation is decided BEFORE glyphless
            // handling: a zero-width joiner / non-joiner / variation selector
            // that continues an emoji composition (the ZWJs in 👨‍👩‍👧, VS-16 in
            // ❤️ or keycaps) is a format char that glyphless classification would
            // otherwise SKIP, splitting the composition. GNU consumes such
            // characters into the active composition instead of drawing them
            // glyphless. Only suppress glyphless handling when there is a
            // preceding glyph to merge into — a standalone joiner still renders
            // glyphless.
            let cluster_tail = self.matrix_builder.last_text_cluster_tail();
            let is_cluster_continuation = crate::composition::continues_cluster(ch, cluster_tail);
            // Only emoji/text composition joiners (ZWJ, variation selectors,
            // tag chars) are absorbed — not C1 controls, bidi marks, or
            // separators, which must still render as their glyphless glyph.
            let absorbed_into_cluster =
                cluster_tail.is_some() && crate::composition::is_composition_joiner(ch);

            // Glyphless character detection (C1 controls, format chars, etc.)
            if let Some(method) = crate::display_item::glyphless_method_for_char(
                ch,
                crate::display_item::GlyphlessJoinerPolicy::ClassifyAsGlyphless,
            )
            .filter(|_| !absorbed_into_cluster)
            {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();

                let buffer_text_fragment = DisplayTextFragment::buffer_text(
                    CharPos0::new(charpos as usize),
                    CharPos0::new((charpos + 1) as usize),
                );
                let text_item_frame = text_append_surface.frame_for_active_face(
                    current_row_geometry!().append_placement(raise_y_offset),
                    &active_face_state,
                    char_h,
                );
                if let Some((_progress, position)) =
                    append_buffer_text_item_fragment_to_text_row_and_emit(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                        buffer_text_fragment,
                        buffer,
                        buf_id,
                        face_resolver,
                        active_face_state.resolved_face(),
                        active_face_state.face_id(),
                        crate::display_item::DisplayItemKind::Glyphless(
                            crate::display_item::DisplayGlyphless { ch, method },
                        ),
                        text_item_frame,
                        DisplayRowPosition { x_px: x, col },
                    )
                {
                    x = position.x_px;
                    col = position.col;
                }
                charpos += 1;
                word_wrap_may_wrap = false;
                continue;
            }

            // Check for line wrap / truncation using per-face char width

            let tab_advance = (ch == '\t').then(|| {
                text_display_tab_policy(content_x, params).advance_from(
                    crate::display_row_builder::DisplayRowPosition { x_px: x, col },
                    face_metrics.space_width,
                )
            });

            // Grapheme-cluster extenders (combining marks, ZWJ, variation
            // selectors) share the preceding base char's cell — zero columns,
            // zero advance — grouping clusters identically across every layout
            // walk (neomacs's stand-in for GNU's shared `composition_it`,
            // src/composite.c). `cluster_tail` / `is_cluster_continuation` were
            // computed above (before glyphless handling) and are reused here.
            let char_cols = if let Some(tab_advance) = tab_advance {
                tab_advance.width_cols
            } else if is_cluster_continuation {
                0
            } else {
                crate::composition::base_width_cols(ch) as usize
            };
            let advance = if let Some(tab_advance) = tab_advance {
                tab_advance.pixel_width
            } else if is_cluster_continuation {
                0.0
            } else if crate::composition::needs_complex_shaping(ch) {
                // Use the joined-form advance from shaping the whole run, so
                // composed Arabic/Indic text is tight and cursor columns line
                // up with the rendered letters (isolated-form widths over-
                // reserve). Shape the run once and cache advances by absolute
                // byte offset.
                if !(complex_run_start <= ch_start_byte_idx && ch_start_byte_idx < complex_run_end)
                {
                    let script = crate::composition::complex_script(ch);
                    let mut end = ch_start_byte_idx;
                    let mut run_text = String::new();
                    while end < text.len() {
                        let (c, clen) = decode_utf8(&text[end..]);
                        if crate::composition::complex_script(c) == script
                            || (end > ch_start_byte_idx && is_cluster_extender(c))
                        {
                            run_text.push(c);
                            end += clen;
                        } else {
                            break;
                        }
                    }
                    let measurement =
                        active_face_state.text_run_measurement(&mut self.font_metrics, &run_text);
                    complex_run_adv.clear();
                    // Leave the cache empty when shaping yields nothing (no
                    // font / unavailable) so each char falls back to its
                    // isolated width rather than collapsing to zero.
                    complex_run_adv =
                        measurement.base_char_byte_advances(&run_text, ch_start_byte_idx);
                    complex_run_start = ch_start_byte_idx;
                    complex_run_end = end;
                }
                match complex_run_adv
                    .iter()
                    .find(|advance| advance.byte_offset == ch_start_byte_idx)
                    .map(|advance| advance.advance_px)
                {
                    // In the shaped run: use it, including 0 for a character
                    // covered by a preceding ligature glyph (no double-count).
                    Some(a) => a,
                    // Not cached (shaping unavailable / no font): fall back to
                    // the isolated-form width.
                    None => {
                        active_face_state.advance_for_columns(&mut self.font_metrics, ch, char_cols)
                    }
                }
            } else {
                active_face_state.advance_for_columns(&mut self.font_metrics, ch, char_cols)
            };
            update_cursor_info_for_main_char(&mut cursor_info, ch_start_byte_idx, advance);
            if ch != '\t' && x + advance > content_x + avail_width {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
                if params.truncate_lines {
                    if row < max_rows {
                        row_truncated[row] = true;
                    }
                    // The current char has been decoded and `byte_idx` is
                    // already past it, but `charpos` is not yet incremented
                    // (that happens after the would-be push below). Account
                    // for the consumed-but-uncounted char here so
                    // `skip_to_newline` starts from the right offset.
                    charpos += 1;
                    // Skip remaining chars until newline
                    if skip_to_newline(text, &mut byte_idx, &mut charpos) {
                        current_line += 1;
                        need_line_number = lnum_enabled;
                    }
                    x = content_x;
                    row_extend_bg = None;
                    row_extend_row = -1;
                    let geometry_transition = current_row_geometry_vars!()
                        .finish_boundary_and_record_hit(
                            DisplayRowBoundaryTarget::truncation(
                                DisplayRowHitRange {
                                    charpos_start: hit_row_charpos_start,
                                    charpos_end: charpos,
                                },
                                row_geometry_defaults,
                                text_matrix_row_base,
                                col,
                                x,
                                row_y_positions.recording(),
                            ),
                            &mut hit_rows,
                        );
                    // Record hit-test row (wrap/truncation break)
                    let row_transition = TextMatrixRowOutput::new(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_with_row_limit(geometry_transition, max_rows);
                    if row_transition.is_exhausted() {
                        break;
                    }
                    col = 0;
                    word_wrap_may_wrap = false;
                    wrap_has_break = false;
                    trailing_ws_start_col = -1;
                    if has_prefix {
                        need_prefix = 1;
                    }
                    continue;
                } else if params.word_wrap && wrap_has_break {
                    // Word-wrap: rewind to last break point
                    output_emitter.truncate_display_points(wrap_break_display_point_count);
                    output_emitter.restore_current_row_display_positions(
                        wrap_break_row_first_display_pos,
                        wrap_break_row_last_display_pos,
                    );
                    byte_idx = wrap_break_byte_idx;
                    charpos = wrap_break_charpos;
                    col = 0;

                    if row < max_rows {
                        row_continued[row] = true;
                    }
                    x = content_x;
                    row_extend_bg = None;
                    row_extend_row = -1;
                    let geometry_transition = current_row_geometry_vars!()
                        .finish_boundary_and_record_hit(
                            DisplayRowBoundaryTarget::visual_wrap(
                                DisplayRowHitRange {
                                    charpos_start: hit_row_charpos_start,
                                    charpos_end: charpos,
                                },
                                row_geometry_defaults,
                                text_matrix_row_base,
                                col,
                                x,
                                row_y_positions.recording(),
                            ),
                            &mut hit_rows,
                        );
                    // Record hit-test row (wrap/truncation break)
                    let row_transition = TextMatrixRowOutput::new(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_with_row_limit(geometry_transition, max_rows);
                    if row_transition.is_exhausted() {
                        break;
                    }
                    charpos = sync_charpos_from_byte_idx(byte_idx);
                    hit_row_charpos_start = charpos;
                    if row < max_rows {
                        row_continuation[row] = true;
                    }
                    word_wrap_may_wrap = false;
                    wrap_has_break = false;
                    trailing_ws_start_col = -1;
                    if has_prefix {
                        need_prefix = 2;
                    }

                    // Force face re-check since we rewound
                    face_next_check = 0;

                    if !current_row_geometry_vars!().current_row_is_visible(row_visibility_limit) {
                        break;
                    }
                    continue;
                } else {
                    // Character wrap (no break point available)
                    if row < max_rows {
                        row_continued[row] = true;
                    }
                    x = content_x;
                    row_extend_bg = None;
                    row_extend_row = -1;
                    let geometry_transition = current_row_geometry_vars!()
                        .finish_boundary_and_record_hit(
                            DisplayRowBoundaryTarget::visual_wrap(
                                DisplayRowHitRange {
                                    charpos_start: hit_row_charpos_start,
                                    charpos_end: charpos,
                                },
                                row_geometry_defaults,
                                text_matrix_row_base,
                                col,
                                x,
                                row_y_positions.recording(),
                            ),
                            &mut hit_rows,
                        );
                    // Record hit-test row (wrap/truncation break)
                    let row_transition = TextMatrixRowOutput::new(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_with_row_limit(geometry_transition, max_rows);
                    if row_transition.is_exhausted() {
                        break;
                    }
                    col = 0;
                    trailing_ws_start_col = -1;
                    if row < max_rows {
                        row_continuation[row] = true;
                    }
                    byte_idx = ch_start_byte_idx;
                    charpos = sync_charpos_from_byte_idx(byte_idx);
                    hit_row_charpos_start = charpos;
                    word_wrap_may_wrap = false;
                    face_next_check = 0;
                    if has_prefix {
                        need_prefix = 2;
                    }
                    if !current_row_geometry_vars!().current_row_is_visible(row_visibility_limit) {
                        break;
                    }
                    continue;
                }
            }

            // Reset raise offset when past the raise region
            if raise_end > window_start && charpos >= raise_end {
                raise_y_offset = 0.0;
                raise_end = window_start;
            }

            // Capture cursor metrics at point position during the main layout
            // so cursor emission uses the correct per-face height/width.
            if cursor_info.is_none() && charpos == point_charpos {
                capture_cursor_info(
                    &mut cursor_info,
                    CapturedCursorInfo::from_active_face_state(
                        &active_face_state,
                        CapturedCursorPlacement {
                            x,
                            y,
                            byte_idx: ch_start_byte_idx,
                            col,
                            matrix_row: row,
                            slot_width: CapturedCursorSlotWidth::Explicit(advance),
                            stretch_like: ch == '\t',
                        },
                    ),
                );
            }

            // --- Overlay before-strings ---
            if has_overlays {
                let text_props = super::neovm_bridge::RustTextPropAccess::new_for_window(
                    buffer,
                    params.window_id as u64,
                );
                let (before_strings, _) = text_props.overlay_strings_at(charpos);
                if !before_strings.is_empty() {
                    // Flush run buffer before emitting overlay chars
                    flush_run(&self.run_buf, ligatures);
                    self.run_buf.clear();
                    let right_limit = content_x + avail_width;
                    for overlay_string in &before_strings {
                        current_row_geometry_vars!().with_display_row_geometry_state(|geometry| {
                            render_overlay_string(
                                evaluator,
                                &mut output_emitter,
                                buffer,
                                DisplayTextFragment::overlay_string(
                                    overlay_string.string,
                                    overlay_string.overlay_id,
                                    CharPos0::new(charpos as usize),
                                    OverlayStringKind::Before,
                                ),
                                &mut self.font_metrics,
                                face_resolver,
                                &mut x,
                                &mut col,
                                geometry,
                                &mut cursor_info,
                                &mut hit_rows,
                                &mut hit_row_charpos_start,
                                charpos,
                                &mut row_y_positions,
                                face_metrics.char_width,
                                char_h,
                                default_face_ascent,
                                right_limit,
                                content_x,
                                text_y,
                                text_matrix_row_base,
                                max_rows,
                                &mut current_face_id,
                                &mut self.matrix_builder,
                                params,
                            );
                        });
                    }
                }
            }

            // Accumulate drawable text into the ligature run buffer. Tabs are
            // emitted as stretch glyphs through the display-row builder.
            if ch == '\t' {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
            } else if self.run_buf.is_empty() {
                let gy = y + raise_y_offset;
                self.run_buf.start(
                    x,
                    gy,
                    face_metrics.row_height,
                    face_metrics.ascent,
                    active_face_state.face_id(),
                    false,
                );
            }
            if ch != '\t' {
                self.run_buf.push(ch, advance);
            }
            let frame = text_append_surface.frame_for_active_face(
                current_row_geometry!().append_placement(raise_y_offset),
                &active_face_state,
                char_h,
            );
            let buffer_text_fragment = DisplayTextFragment::buffer_text(
                CharPos0::new(charpos as usize),
                CharPos0::new((charpos + 1) as usize),
            );
            let mut ch_text = [0; 4];
            let measurement = active_face_state
                .resolved_fragment_measurement(ch.encode_utf8(&mut ch_text), advance);
            let Some((_progress, position)) = append_buffer_text_fragment_to_text_row(
                &mut self.matrix_builder,
                &mut output_emitter,
                evaluator,
                &mut self.font_metrics,
                buffer_text_fragment,
                face_resolver,
                active_face_state.resolved_face(),
                buf_id,
                buffer,
                active_face_state.face_id(),
                measurement,
                frame,
                DisplayRowPosition { x_px: x, col },
            ) else {
                break;
            };

            // Flush if run is too long
            if self.run_buf.len() >= MAX_LIGATURE_RUN_LEN {
                flush_run(&self.run_buf, ligatures);
                self.run_buf.clear();
            }

            x = position.x_px;
            col = position.col;
            charpos += 1;
            word_wrap_may_wrap = char_can_wrap_after_basic(ch);

            // --- Overlay after-strings ---
            if has_overlays {
                let text_props = super::neovm_bridge::RustTextPropAccess::new_for_window(
                    buffer,
                    params.window_id as u64,
                );
                let (_, after_strings) = text_props.overlay_strings_at(charpos);
                if !after_strings.is_empty() {
                    // Flush run buffer before emitting overlay chars
                    flush_run(&self.run_buf, ligatures);
                    self.run_buf.clear();
                    let right_limit = content_x + avail_width;
                    for overlay_string in &after_strings {
                        current_row_geometry_vars!().with_display_row_geometry_state(|geometry| {
                            render_overlay_string(
                                evaluator,
                                &mut output_emitter,
                                buffer,
                                DisplayTextFragment::overlay_string(
                                    overlay_string.string,
                                    overlay_string.overlay_id,
                                    CharPos0::new(charpos as usize),
                                    OverlayStringKind::After,
                                ),
                                &mut self.font_metrics,
                                face_resolver,
                                &mut x,
                                &mut col,
                                geometry,
                                &mut cursor_info,
                                &mut hit_rows,
                                &mut hit_row_charpos_start,
                                charpos,
                                &mut row_y_positions,
                                face_metrics.char_width,
                                char_h,
                                default_face_ascent,
                                right_limit,
                                content_x,
                                text_y,
                                text_matrix_row_base,
                                max_rows,
                                &mut current_face_id,
                                &mut self.matrix_builder,
                                params,
                            );
                        });
                    }
                }
            }

            // Track trailing whitespace
            if trailing_ws_bg.is_some() {
                if ch == ' ' || ch == '\t' {
                    if trailing_ws_start_col < 0 {
                        trailing_ws_start_col = if ch == '\t' {
                            col as i32
                        } else {
                            (col as i32) - 1
                        };
                        trailing_ws_start_x = x - advance;
                        trailing_ws_row = row;
                    }
                } else {
                    trailing_ws_start_col = -1;
                }
            }
        }

        flush_run(&self.run_buf, ligatures);
        self.run_buf.clear();

        let point_is_visible_eob = point_charpos == accessible_end && charpos == accessible_end;

        // Capture cursor at end-of-buffer position.
        // GNU Emacs shows point at point-max+1 as a real cursor location.
        // In the layout engine's internal 0-based space, that is `accessible_end`.
        if cursor_info.is_none() && (charpos == point_charpos || point_is_visible_eob) {
            if point_is_visible_eob {
                tracing::debug!(
                    "layout_window_rust: capturing EOB cursor at x={:.1} y={:.1} point={} point-max={}",
                    x,
                    y,
                    point_charpos,
                    accessible_end
                );
            }
            capture_cursor_info(
                &mut cursor_info,
                CapturedCursorInfo::from_active_face_state(
                    &active_face_state,
                    CapturedCursorPlacement {
                        x,
                        y,
                        byte_idx,
                        col,
                        matrix_row: row,
                        slot_width: CapturedCursorSlotWidth::FaceChar,
                        stretch_like: false,
                    },
                ),
            );
        }

        // Close any remaining box face region at end of text
        if box_active {
            let _ = (box_start_x, box_row); // suppress unused warnings
        }

        // EOB overlay strings: check for overlay strings at the end-of-buffer position
        if has_overlays && row < max_rows {
            let text_props = super::neovm_bridge::RustTextPropAccess::new_for_window(
                buffer,
                params.window_id as u64,
            );
            let (before_strings, after_strings) = text_props.overlay_strings_at(charpos);
            let right_limit = content_x + avail_width;
            for overlay_string in &before_strings {
                current_row_geometry_vars!().with_display_row_geometry_state(|geometry| {
                    render_overlay_string(
                        evaluator,
                        &mut output_emitter,
                        buffer,
                        DisplayTextFragment::overlay_string(
                            overlay_string.string,
                            overlay_string.overlay_id,
                            CharPos0::new(charpos as usize),
                            OverlayStringKind::Before,
                        ),
                        &mut self.font_metrics,
                        face_resolver,
                        &mut x,
                        &mut col,
                        geometry,
                        &mut cursor_info,
                        &mut hit_rows,
                        &mut hit_row_charpos_start,
                        charpos,
                        &mut row_y_positions,
                        face_metrics.char_width,
                        char_h,
                        default_face_ascent,
                        right_limit,
                        content_x,
                        text_y,
                        text_matrix_row_base,
                        max_rows,
                        &mut current_face_id,
                        &mut self.matrix_builder,
                        params,
                    );
                });
            }
            for overlay_string in &after_strings {
                current_row_geometry_vars!().with_display_row_geometry_state(|geometry| {
                    render_overlay_string(
                        evaluator,
                        &mut output_emitter,
                        buffer,
                        DisplayTextFragment::overlay_string(
                            overlay_string.string,
                            overlay_string.overlay_id,
                            CharPos0::new(charpos as usize),
                            OverlayStringKind::After,
                        ),
                        &mut self.font_metrics,
                        face_resolver,
                        &mut x,
                        &mut col,
                        geometry,
                        &mut cursor_info,
                        &mut hit_rows,
                        &mut hit_row_charpos_start,
                        charpos,
                        &mut row_y_positions,
                        face_metrics.char_width,
                        char_h,
                        default_face_ascent,
                        right_limit,
                        content_x,
                        text_y,
                        text_matrix_row_base,
                        max_rows,
                        &mut current_face_id,
                        &mut self.matrix_builder,
                        params,
                    );
                });
            }
        }

        // Face :extend at end-of-buffer: fill remaining empty rows
        // with the last :extend face's background color
        if let Some((_ext_bg, _ext_face_id)) = row_extend_bg {
            let right_edge = content_x + avail_width;
            // First, extend the current (partially filled) row if text didn't fill it
            if x < right_edge && row < max_rows {
                let _ry = row_y_positions
                    .y_for_row(row, current_row_geometry!().row_y_fallback(text_y, char_h));
            }
            // Then fill completely empty rows below
            let start_row = (row + 1).min(max_rows);
            for r in start_row..max_rows {
                let ry = row_y_positions
                    .y_for_row(r, current_row_geometry!().row_y_fallback(text_y, char_h));
                if ry + char_h > text_y + text_height {
                    break;
                } // Don't extend past text area
            }
        }

        // Render fringe indicators
        if params.left_fringe_width > 0.0 || params.right_fringe_width > 0.0 {
            let _fringe_char_w = params.left_fringe_width.min(char_w).max(char_w * 0.5);

            for r in 0..row.min(max_rows) {
                let _gy = row_y_positions.y_for_row(
                    r,
                    DisplayRowYFallback {
                        text_y,
                        default_height: char_h,
                        row_extra_y: 0.0,
                    },
                );

                // Right fringe: continuation arrow for wrapped lines
                if params.right_fringe_width > 0.0 && row_continued.get(r).copied().unwrap_or(false)
                {
                }

                // Right fringe: truncation indicator
                if params.right_fringe_width > 0.0 && row_truncated.get(r).copied().unwrap_or(false)
                {
                }

                // Left fringe: continuation from previous line
                if params.left_fringe_width > 0.0
                    && row_continuation.get(r).copied().unwrap_or(false)
                {}
            }

            // Empty line indicators (after buffer text ends)
            if params.indicate_empty_lines > 0 {
                let eob_start = row.min(max_rows);
                for r in eob_start..max_rows {
                    let _gy = row_y_positions
                        .y_for_row(r, current_row_geometry!().row_y_fallback(text_y, char_h));
                    let _fringe_x = if params.indicate_empty_lines == 2 {
                        right_fringe_x
                    } else {
                        left_fringe_x
                    };
                    let fringe_w = if params.indicate_empty_lines == 2 {
                        params.right_fringe_width
                    } else {
                        params.left_fringe_width
                    };
                    if fringe_w > 0.0 {}
                }
            }
        }

        // Render fill-column indicator
        if params.fill_column_indicator >= 0 {
            let fci_col = params.fill_column_indicator;
            let _fci_char = params.fill_column_indicator_char;
            let _fci_fg = if params.fill_column_indicator_fg != 0 {
                Color::from_pixel(params.fill_column_indicator_fg)
            } else {
                default_fg
            };

            // Draw indicator character at the fill column on each row
            if (fci_col as usize) < cols {
                let indicator_x = content_x + fci_col as f32 * char_w;
                let total_rows = row.min(max_rows);
                for r in 0..total_rows {
                    let _gy = row_y_positions.y_for_row(
                        r,
                        DisplayRowYFallback {
                            text_y,
                            default_height: char_h,
                            row_extra_y: 0.0,
                        },
                    );
                    if indicator_x < content_x + avail_width {}
                }
            }
        }

        if point_charpos >= window_start && (point_charpos <= charpos || point_is_visible_eob) {
            if let Some(cursor) = cursor_info {
                let row_metric = row_metrics_for_cursor(
                    output_emitter.row_metrics(),
                    text_matrix_row_base + cursor.matrix_row,
                    text_matrix_row_base + row,
                    y,
                    row_max_height,
                    row_max_ascent,
                );
                output_emitter.set_logical_cursor(cursor.logical_cursor_position(
                    row_metric,
                    text_matrix_row_base,
                    text_area_left,
                    window_top,
                ));
                if let Some(style) = cursor_style_for_window(params) {
                    let source = CursorGeometrySource::from_captured_cursor(
                        &cursor,
                        row_metric,
                        CursorGeometryContext {
                            window_id: params.window_id,
                            slot_width: cursor.resolved_slot_width(style, text, params),
                            default_line_height: char_h,
                            ends_at_visible_eob: point_is_visible_eob,
                        },
                    );
                    let resolved_cursor = resolve_cursor_geometry(
                        style,
                        source,
                        params.x_stretch_cursor,
                        char_w,
                        Color::from_pixel(params.cursor_color),
                    );
                    if resolved_cursor.y >= text_y
                        && resolved_cursor.y + resolved_cursor.height <= text_y + text_height
                    {
                        // The selected window's cursor is published as the phys
                        // cursor below; every backend draws it from there and
                        // dedups the matching per-window CursorItem. Pushing one
                        // for the selected window is pure redundancy (and was the
                        // source of the "two cursors" drift), so only non-selected
                        // windows get a per-window cursor here.
                        if !params.selected {
                            self.matrix_builder.push_cursor(
                                resolved_cursor.window_id(),
                                resolved_cursor.slot_id,
                                resolved_cursor.x,
                                resolved_cursor.y,
                                resolved_cursor.width,
                                resolved_cursor.height,
                                resolved_cursor.style,
                                resolved_cursor.color,
                            );
                        }
                        self.matrix_builder.set_cursor_at_row(
                            resolved_cursor.row(),
                            resolved_cursor.col(),
                            resolved_cursor.style,
                        );
                        output_emitter.set_phys_cursor(WindowCursorSnapshot {
                            kind: window_cursor_kind(resolved_cursor.style),
                            x: (resolved_cursor.x - text_area_left).round() as i64,
                            y: (resolved_cursor.y - window_top).round() as i64,
                            width: resolved_cursor.width.round() as i64,
                            height: resolved_cursor.height.round() as i64,
                            ascent: resolved_cursor.ascent.round() as i64,
                            row: resolved_cursor.row() as i64,
                            col: i64::from(resolved_cursor.col()),
                        });
                        if params.selected {
                            self.matrix_builder.set_phys_cursor(PhysCursor {
                                window_id: resolved_cursor.window_id(),
                                charpos: point_charpos.max(0) as usize,
                                row: resolved_cursor.row(),
                                col: resolved_cursor.col(),
                                slot_id: resolved_cursor.slot_id,
                                x: resolved_cursor.x,
                                y: resolved_cursor.y,
                                width: resolved_cursor.width,
                                height: resolved_cursor.height,
                                ascent: resolved_cursor.ascent,
                                style: resolved_cursor.style,
                                color: resolved_cursor.color,
                                cursor_fg: resolved_cursor.cursor_fg,
                            });
                        }

                        if point_is_visible_eob {
                            tracing::debug!(
                                "layout_window_rust: emitting EOB cursor at x={:.1} y={:.1} w={:.1} h={:.1}",
                                resolved_cursor.x,
                                resolved_cursor.y,
                                resolved_cursor.width,
                                resolved_cursor.height
                            );
                        }
                    }
                }
            } else {
                tracing::debug!(
                    "layout_window_rust: no explicit cursor capture for point={} window_start={} charpos_end={}",
                    point_charpos,
                    window_start,
                    charpos
                );
            }
        }

        let has_pending_row_output = output_emitter.current_row_has_output();
        if row < max_rows && (charpos > hit_row_charpos_start || has_pending_row_output) {
            let row_y_start = row_y_positions
                .y_for_row(row, current_row_geometry!().row_y_fallback(text_y, char_h));
            let row_cursor = current_row_geometry!().with_row_y(row_y_start).cursor();
            hit_rows.push(row_cursor.hit_row(hit_row_charpos_start, charpos));
            TextMatrixRowOutput::new(&mut self.matrix_builder, &mut output_emitter, evaluator)
                .finish(row_cursor.finish_current_row());
        }

        for spec in &params.visual_cursors {
            let Some(style) = cursor_style_for_visual(spec) else {
                continue;
            };
            let Some(point) = output_emitter
                .point_for_lisp_buffer_pos(layout_i64_char_pos_to_lisp_char_pos(spec.charpos))
            else {
                continue;
            };
            let source =
                visual_cursor_source_from_point(point, spec.id as i64, text_area_left, window_top);
            let resolved_cursor = resolve_cursor_geometry(
                style,
                source,
                params.x_stretch_cursor,
                char_w,
                Color::from_pixel(spec.color),
            );
            if resolved_cursor.y < text_y
                || resolved_cursor.y + resolved_cursor.height > text_y + text_height
            {
                continue;
            }
            if let Some(effects) = spec.effects.clone() {
                self.matrix_builder
                    .set_window_cursor_effects(spec.id as i64, effects);
            }
            self.matrix_builder.push_cursor(
                resolved_cursor.window_id(),
                resolved_cursor.slot_id,
                resolved_cursor.x,
                resolved_cursor.y,
                resolved_cursor.width,
                resolved_cursor.height,
                resolved_cursor.style,
                resolved_cursor.color,
            );
        }

        // GNU redisplay keeps iterating until point visibility converges or no
        // further progress can be made.  Advance by actual rendered row spans
        // from this pass rather than rescanning by logical newlines, since
        // wrapped and variable-height lines are exactly where newline-based
        // retry selection goes wrong.
        let visible_end_lisp = output_emitter
            .rows()
            .iter()
            .rev()
            .find_map(|row| row.end_buffer_pos);
        let point_lisp = layout_i64_char_pos_to_lisp_char_pos(point_charpos);
        let visible_end_lisp = if point_is_visible_eob {
            Some(visible_end_lisp.unwrap_or(point_lisp).max(point_lisp))
        } else {
            visible_end_lisp
        };
        let visible_progress = visible_end_lisp
            .map(LispCharPos1::as_i64)
            .unwrap_or(charpos);
        let point_beyond_visible_span = visible_end_lisp
            .map(|end_lisp| point_lisp > end_lisp)
            .unwrap_or(point_charpos > charpos);

        let scroll_down_ws = if point_beyond_visible_span
            && visible_progress > window_start
            && !params.is_minibuffer
        {
            let new_ws = next_window_start_from_visible_rows(output_emitter.rows(), window_start)
                .map(|new_ws| new_ws.min(point_charpos.max(accessible_start)));
            tracing::debug!(
                "layout_window_rust: point={} beyond visible_end={:?} (charpos_end={}), visible_rows={}, new_window_start={:?}",
                point_lisp.as_i64(),
                visible_end_lisp,
                charpos,
                output_emitter.rows().len(),
                new_ws
            );
            new_ws
        } else {
            None
        };
        let text_area_top = (text_y - window_top).round() as i64;
        let text_area_bottom = (text_y + text_height - window_top).round() as i64;
        let point_row_ws = next_window_start_for_partially_visible_point_row(
            output_emitter.rows(),
            point_charpos,
            text_area_top,
            text_area_bottom,
            window_start,
        );
        if point_row_ws.is_some() {
            tracing::debug!(
                "layout_window_rust: point={} row partially visible within {}..{}, new_window_start={:?}",
                point_charpos,
                text_area_top,
                text_area_bottom,
                point_row_ws
            );
        }
        let point_line_ws = next_window_start_for_point_line_continuation(
            output_emitter.rows(),
            point_charpos,
            window_start,
            &buf_access,
            accessible_end,
        );
        if point_line_ws.is_some() {
            tracing::debug!(
                "layout_window_rust: point={} line continues below final visible row, new_window_start={:?}",
                point_charpos,
                point_line_ws
            );
        }
        let retry_window_start = scroll_down_ws.or(point_row_ws).or(point_line_ws);

        if let Some(new_window_start) = retry_window_start
            && remaining_visibility_retries > 0
            && new_window_start > window_start
        {
            tracing::debug!(
                "layout_window_rust: retrying window {} with adjusted window_start {} -> {} (remaining={})",
                params.window_id,
                window_start,
                new_window_start,
                remaining_visibility_retries
            );
            self.matrix_builder
                .truncate_transition_hints(transition_hints_len_before);
            self.matrix_builder
                .truncate_effect_hints(effect_hints_len_before);

            let mut retry_params = params.clone();
            retry_params.window_start = new_window_start;
            retry_params.window_end = 0;
            // Persist the counter BEFORE recursing so the retry
            // call loads the parent's bumped value as its base.
            // The retry will write back its final counter; the
            // unconditional `return` below skips the bottom-of-
            // function writeback path.
            self.frame_face_id_counter = current_face_id;
            self.layout_window_rust(
                evaluator,
                frame_id,
                &retry_params,
                frame_params,
                face_resolver,
                reserve_right_border_col,
                remaining_visibility_retries.saturating_sub(1),
            );
            return;
        }

        let window_start_lisp = layout_i64_char_pos_to_lisp_char_pos(window_start);
        // Use the last row that actually has a buffer position, not
        // just the last row.  Empty trailing rows (e.g. the blank
        // line after a buffer ending with `\n`) have
        // end_buffer_pos = None.  Using `.last()` hit that None and
        // fell back to 1, making the %p mode-line construct show
        // "Top" instead of "All" for short buffers.
        let window_end_lisp = output_emitter
            .rows()
            .iter()
            .rev()
            .find_map(|row| row.end_buffer_pos)
            .map(|pos| layout_i64_char_pos_to_lisp_char_pos(pos.as_i64()))
            .unwrap_or_else(|| LispCharPos1::from_one_based_usize(1));
        let window_end_byte = EmacsBytePos::new(text_start_byte.saturating_add(byte_idx));
        let window_end_vpos = output_emitter
            .rows()
            .last()
            .map(|row| row.row.max(0) as usize)
            .unwrap_or(0);

        if let Some(info) = self.matrix_builder.window_infos_last_mut()
            && info.window_id == params.window_id
        {
            info.window_start = window_start_lisp.as_i64();
            info.window_end = window_end_lisp.as_i64();
        }

        tracing::debug!(
            "  layout_window_rust: window_start={} window_end={}",
            window_start_lisp.as_i64(),
            window_end_lisp.as_i64()
        );

        // GNU status-line percent specs read the live window state from the
        // just-produced redisplay. Publish the authoritative window geometry
        // before evaluating mode-line/header-line/tab-line forms so `%p/%P/%o`
        // reflect the frame we are about to render, not stale state from the
        // previous redisplay.
        evaluator.publish_redisplay_window_positions(
            frame_id,
            neovm_core::window::WindowId(params.window_id as u64),
            window_start_lisp,
            LispCharPos1::from_one_based_usize(accessible_end_lisp_char),
            EmacsBytePos::new(accessible_end_emacs_byte),
            window_end_lisp,
            window_end_byte,
            window_end_vpos,
        );

        // --- GlyphMatrix builder: finalize text rows, then emit chrome rows
        // into their real glyph-matrix slots before closing the window. ---
        for metric in output_emitter.row_metrics() {
            self.matrix_builder.set_row_metrics(
                metric.row,
                metric.pixel_y,
                metric.height,
                metric.ascent,
            );
        }
        self.matrix_builder.end_row();
        if reserve_right_special_col {
            let target_col = if reserve_right_border_col {
                matrix_cols.saturating_sub(2)
            } else {
                matrix_cols.saturating_sub(1)
            };
            for row_idx in 0..row_truncated.len() {
                let matrix_row = text_matrix_row_base + row_idx;
                if row_truncated[row_idx] {
                    self.matrix_builder
                        .overwrite_current_window_row_glyph_at_col(matrix_row, target_col, '$', 0);
                } else if row_continued[row_idx] {
                    self.matrix_builder
                        .overwrite_current_window_row_glyph_at_col(matrix_row, target_col, '\\', 0);
                }
            }
        }

        let mut status_line_symbol_values = std::collections::HashMap::new();
        if let Some(buffer) = evaluator
            .buffer_manager()
            .get(neovm_core::buffer::BufferId(params.buffer_id))
        {
            if let Some(value) = buffer.buffer_local_value("header-line-indent-width") {
                status_line_symbol_values.insert("header-line-indent-width".to_string(), value);
            }
        }

        // Tab-line: evaluate format-mode-line with tab-line-format
        if params.tab_line_height > 0.0 {
            // Tab-line is above header-line (at the very top of the window)
            let tl_y = params.bounds.y;
            let tl_row = 0i64;
            let tl_face = tab_line_face
                .as_ref()
                .expect("tab-line face should exist when tab-line height is positive");

            let tab_line_target_cols = ((params.bounds.width / char_w.max(1.0)).round().max(1.0)
                as usize)
                .saturating_sub(usize::from(reserve_right_border_col))
                .max(1);
            let tab_text = eval_status_line_format_value(
                evaluator,
                "tab-line-format",
                params.window_id,
                params.buffer_id,
                tab_line_target_cols,
            )
            .unwrap_or_else(|| Value::string(""));

            let tab_row_output = ChromeRowOutput {
                row: tl_row,
                y: tl_y,
            };
            let tab_row_spec = DisplayRowSpec::from_base_face(
                DisplayRowGeometry {
                    y: tl_y,
                    width: params.bounds.width,
                    height: tab_line_height,
                    char_width: char_w,
                    ascent: font_ascent,
                    tab_policy: text_display_tab_policy(0.0, params),
                },
                &mut current_face_id,
                tl_face,
                GlyphRowRole::TabLine,
                status_line_symbol_values.clone(),
            );
            self.render_window_chrome_display_row(
                evaluator,
                &mut output_emitter,
                face_resolver,
                &mut current_face_id,
                0,
                tab_row_output,
                DisplayRowOwner::WindowChrome {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::TabLine,
                },
                Rect::new(params.bounds.x, tl_y, params.bounds.width, tab_line_height),
                tab_row_spec,
                DisplayTextFragment::tab_line(tab_text),
            );
        }

        // Header-line: evaluate format-mode-line with header-line-format.
        // Emit top chrome in visual order so live output progression does not
        // regress from later body rows back to row 0.
        if params.header_line_height > 0.0 {
            let hl_y = params.bounds.y + tab_line_height;
            let hl_row = i64::from(tab_line_height > 0.0);
            let hl_face = header_line_face
                .as_ref()
                .expect("header-line face should exist when header-line height is positive");

            let header_line_target_cols = ((params.bounds.width / char_w.max(1.0)).round().max(1.0)
                as usize)
                .saturating_sub(usize::from(reserve_right_border_col))
                .max(1);
            let header_text = eval_status_line_format_value(
                evaluator,
                "header-line-format",
                params.window_id,
                params.buffer_id,
                header_line_target_cols,
            )
            .unwrap_or_else(|| Value::string(""));

            let header_row_output = ChromeRowOutput {
                row: hl_row,
                y: hl_y,
            };
            let header_row_spec = DisplayRowSpec::from_base_face(
                DisplayRowGeometry {
                    y: hl_y,
                    width: params.bounds.width,
                    height: header_line_height,
                    char_width: char_w,
                    ascent: font_ascent,
                    tab_policy: text_display_tab_policy(0.0, params),
                },
                &mut current_face_id,
                hl_face,
                GlyphRowRole::HeaderLine,
                status_line_symbol_values.clone(),
            );
            self.render_window_chrome_display_row(
                evaluator,
                &mut output_emitter,
                face_resolver,
                &mut current_face_id,
                usize::from(tab_line_height > 0.0),
                header_row_output,
                DisplayRowOwner::WindowChrome {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::HeaderLine,
                },
                Rect::new(
                    params.bounds.x,
                    hl_y,
                    params.bounds.width,
                    header_line_height,
                ),
                header_row_spec,
                DisplayTextFragment::header_line(header_text, params.selected),
            );
        }

        // Mode-line: evaluate format-mode-line or fall back to buffer name.
        // Commit it last so live output progression ends on the visually last
        // row in the window matrix.
        if params.mode_line_height > 0.0 {
            let ml_y = params.bounds.y + params.bounds.height - mode_line_height;
            let ml_row = mode_line_matrix_row as i64;
            let ml_face = mode_line_face
                .as_ref()
                .expect("mode-line face should exist when mode-line height is positive");

            // GNU `display_mode_line` walks the format in
            // `MODE_LINE_DISPLAY` mode, so `%-` fills the remaining
            // row width with dashes. Compute the row width in
            // character cells and pass it through.
            let mode_line_target_cols = ((params.bounds.width / char_w.max(1.0)).round().max(1.0)
                as usize)
                .saturating_sub(usize::from(reserve_right_border_col))
                .max(1);
            let mode_text = {
                let result = eval_status_line_format_value(
                    evaluator,
                    "mode-line-format",
                    params.window_id,
                    params.buffer_id,
                    mode_line_target_cols,
                )
                .unwrap_or_else(|| Value::string(format!(" {} ", buffer_name)));
                tracing::debug!(
                    "mode-line eval result: {:?} (len={})",
                    result
                        .as_utf8_str()
                        .map(|s| &s[..s.len().min(120)])
                        .unwrap_or(""),
                    result.as_utf8_str().map(str::len).unwrap_or(0)
                );
                result
            };

            let mode_row_output = ChromeRowOutput {
                row: ml_row,
                y: ml_y,
            };
            let mode_row_spec = DisplayRowSpec::from_base_face(
                DisplayRowGeometry {
                    y: ml_y,
                    width: params.bounds.width,
                    height: mode_line_height,
                    char_width: char_w,
                    ascent: font_ascent,
                    tab_policy: text_display_tab_policy(0.0, params),
                },
                &mut current_face_id,
                ml_face,
                GlyphRowRole::ModeLine,
                status_line_symbol_values.clone(),
            );
            self.render_window_chrome_display_row(
                evaluator,
                &mut output_emitter,
                face_resolver,
                &mut current_face_id,
                mode_line_matrix_row,
                mode_row_output,
                DisplayRowOwner::WindowChrome {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::ModeLine,
                },
                Rect::new(params.bounds.x, ml_y, params.bounds.width, mode_line_height),
                mode_row_spec,
                DisplayTextFragment::mode_line(mode_text, params.selected),
            );
        }

        self.matrix_builder.end_window();

        // Store hit-test data for this window
        self.hit_data.push(WindowHitData {
            window_id: params.window_id,
            content_x,
            char_w,
            rows: hit_rows,
        });

        let snapshot = output_emitter.finish_snapshot(
            evaluator,
            (text_area_left - params.bounds.x).round() as i64,
            mode_line_height.round() as i64,
            header_line_height.round() as i64,
            tab_line_height.round() as i64,
        );
        self.display_snapshots.push(snapshot);

        // Persist the face-id counter back to the frame-wide
        // slot so the NEXT window in this frame starts allocating
        // face_ids past the ones we just used. Without this
        // write-back every sibling window would reuse ids 1..N
        // and overwrite this window's entries in the shared
        // `matrix_builder.faces` HashMap — the original
        // manifestation of the "C-x 2 paints both mode lines
        // with mode-line-inactive colors" bug. Mirrors GNU's
        // single `face_cache->used` counter at
        // `src/xfaces.c::init_frame_faces`.
        self.frame_face_id_counter = current_face_id;
    }

    /// Trigger fontification for a buffer region via the Rust Context.
    ///
    /// Delegates to the neovm-core redisplay helper modeled after GNU
    /// `handle_fontified_prop`: walk the visible Lisp character region and
    /// invoke `fontification-functions` at each unfontified position.
    fn ensure_fontified_rust(
        evaluator: &mut neovm_core::emacs_core::Context,
        buf_id: neovm_core::buffer::BufferId,
        from: i64,
        to: i64,
    ) {
        if let Err(e) = neovm_core::emacs_core::xdisp::ensure_fontified_for_redisplay(
            evaluator, buf_id, from, to,
        ) {
            tracing::debug!("ensure_fontified_rust: fontification error: {:?}", e);
        }
    }
}

fn minibuffer_resize_line_count(buffer: &neovm_core::buffer::Buffer, window_id: u64) -> usize {
    let text_lines = buffer
        .buffer_substring_bytes_range(buffer.accessible_emacs_byte_range())
        .into_iter()
        .filter(|&byte| byte == b'\n')
        .count();

    let window_sym = Value::symbol("window");
    let accessible_end_byte = buffer.accessible_emacs_byte_region().end();
    let overlays = buffer.overlays();
    let overlay_lines: usize = overlays
        .overlays_in_emacs_byte_range(EmacsByteRange::new(
            EmacsBytePos::ZERO,
            EmacsBytePos::ZERO.add_len(buffer.total_emacs_byte_len()),
        ))
        .iter()
        .filter(|ov| match overlays.overlay_get_named(**ov, window_sym) {
            Some(prop) => prop
                .as_window_id()
                .is_none_or(|overlay_window_id| overlay_window_id == window_id),
            None => true,
        })
        .map(|ov| {
            let before_lines = if overlays
                .overlay_start_emacs_byte_pos(*ov)
                .is_some_and(|start| start < accessible_end_byte)
            {
                overlays
                    .overlay_get_named(*ov, Value::symbol("before-string"))
                    .and_then(|value| value.as_lisp_string())
                    .map(|string| {
                        string
                            .as_bytes()
                            .iter()
                            .filter(|&&byte| byte == b'\n')
                            .count()
                    })
                    .unwrap_or(0)
            } else {
                0
            };
            let after_lines = overlays
                .overlay_get_named(*ov, Value::symbol("after-string"))
                .and_then(|value| value.as_lisp_string())
                .map(|string| {
                    string
                        .as_bytes()
                        .iter()
                        .filter(|&&byte| byte == b'\n')
                        .count()
                })
                .unwrap_or(0);
            before_lines + after_lines
        })
        .sum();

    text_lines + overlay_lines + 1
}

impl LayoutEngine {
    /// Build minibuffer echo rows through the shared display-source path.
    ///
    /// The returned rows retain their realized faces and progress metadata so
    /// the caller can install them through the same path used by chrome rows.
    pub(crate) fn render_minibuffer_echo_rows(
        &mut self,
        y: f32,
        text_width: f32,
        char_w: f32,
        ascent: f32,
        row_height: f32,
        default_resolved: &crate::neovm_bridge::ResolvedFace,
        face_resolver: &crate::neovm_bridge::FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        echo_message: Value,
        max_rows: usize,
        truncate_lines: bool,
        reserve_right_special_col: bool,
        next_face_id: &mut u32,
    ) -> Vec<RenderedDisplayRow> {
        use neomacs_display_protocol::glyph_matrix::Glyph;

        let mut base_face = default_resolved.clone();
        if base_face.font_char_width <= 0.0 {
            base_face.font_char_width = char_w.max(1.0);
        }
        if base_face.font_ascent <= 0.0 {
            base_face.font_ascent = ascent.max(row_height * 0.8);
        }
        let row_face = self.realize_display_row_face(0, &base_face, char_w, ascent, row_height);
        let base_render_face = row_face.render_face();
        let char_width = self.display_row_char_width(&row_face, char_w);
        let reserve_width = if reserve_right_special_col {
            char_width.max(1.0)
        } else {
            0.0
        };
        let wrap_width = if truncate_lines {
            text_width
        } else {
            (text_width - reserve_width).max(char_width.max(1.0))
        };
        let matrix_cols = (text_width / char_w.max(1.0)).ceil().max(1.0) as usize;
        let special_col = matrix_cols.saturating_sub(1);
        let base_face_id = if base_face.face_id != 0 {
            base_face.face_id
        } else {
            let face_id = *next_face_id;
            *next_face_id += 1;
            face_id
        };
        let Some(mut source) = crate::display_source::LispStringSourceCursor::new(
            1,
            echo_message,
            crate::display_item::RenderFaceRef::FaceId(base_face_id),
        ) else {
            return empty_minibuffer_echo_row(y, ascent, row_height);
        };
        let mut source_state = DisplayRowSourceState::default();
        let mut renderer = DisplayRowRenderer::new(&mut self.font_metrics);

        let mut rows = Vec::new();
        let max_rows = max_rows.max(1);
        while rows.len() < max_rows {
            let row_spec = DisplayRowSpec {
                geometry: DisplayRowGeometry {
                    y: y + rows.len() as f32 * row_height,
                    width: wrap_width,
                    height: row_height,
                    char_width: char_w,
                    ascent,
                    tab_policy: DisplayTabPolicy::every(8),
                },
                render_bounds: DisplayRowRenderBounds::whole_row(wrap_width),
                base_face_id,
                base_face: &base_face,
                role: GlyphRowRole::Minibuffer,
                symbol_values: std::collections::HashMap::new(),
            };
            let Some(result) = renderer.render_display_item_source_row_step_with_display_host(
                row_spec,
                &mut source,
                &mut source_state,
                face_resolver,
                display_host,
                next_face_id,
            ) else {
                break;
            };
            let stop = result.stop;
            let mut rendered = result.rendered;
            let special_face_id = rendered
                .faces
                .first()
                .map(|face| face.id)
                .unwrap_or(base_render_face.id);
            rendered.row.role = GlyphRowRole::Minibuffer;
            rendered.row.mode_line = false;
            if reserve_right_special_col && stop == DisplayRowRenderStop::Clipped {
                let ch = if truncate_lines { '$' } else { '\\' };
                while rendered.row.glyphs[1].len() < special_col {
                    rendered.row.glyphs[1].push(
                        Glyph::char(' ', special_face_id, 0).with_pixel_width(char_width.max(1.0)),
                    );
                }
                rendered.row.glyphs[1].push(
                    Glyph::char(ch, special_face_id, 0).with_pixel_width(char_width.max(1.0)),
                );
                rendered.progress.end_x = text_width.max(0.0);
                rendered.progress.end_col = matrix_cols as i64;
            }
            rows.push(rendered);
            match stop {
                DisplayRowRenderStop::SourceExhausted => break,
                DisplayRowRenderStop::RowBreak => {}
                DisplayRowRenderStop::Clipped => {
                    if truncate_lines {
                        break;
                    }
                }
            }
        }
        if rows.is_empty() {
            return empty_minibuffer_echo_row(y, ascent, row_height);
        }
        rows
    }

    pub(crate) fn display_row_char_width(
        &mut self,
        face: &DisplayRowFace,
        fallback_char_width: f32,
    ) -> f32 {
        crate::display_row::DisplayRowFaceRealizer::new(&mut self.font_metrics)
            .char_width(face, fallback_char_width)
    }

    /// Render the frame-level tab-bar from GNU Lisp keymap output on the Rust path.
    ///
    /// Build the frame-level tab-bar row and attach it to the published
    /// `FrameDisplayState` as frame chrome, not as a leaf-window row.
    ///
    /// GNU handles the tab bar outside ordinary leaf-window text rows:
    /// - GUI uses `frame->tab_bar_window`
    /// - TTY writes tab-bar rows directly into the frame matrix
    ///
    /// Neomacs keeps immutable snapshots, so this method records a
    /// frame-level `FrameChromeRow` that renderers can consume directly.
    fn render_frame_tab_bar_rust(
        &mut self,
        evaluator: &mut neovm_core::emacs_core::Context,
        frame_window_id: i64,
        face_resolver: &super::neovm_bridge::FaceResolver,
        frame_params: &FrameParams,
        tab_bar_height: f32,
    ) -> Option<f32> {
        let gc_roots = ScratchGcRootScope::new();
        let Some(tab_bar) = build_tab_bar_display(evaluator, frame_window_id as u64, &gc_roots)
        else {
            return None;
        };

        let width = frame_params.width;
        let mut tab_bar_face = face_resolver.resolve_named_face("tab-bar");
        if tab_bar_face.font_char_width <= 0.0 {
            tab_bar_face.font_char_width = frame_params.char_width;
        }
        if tab_bar_face.font_ascent <= 0.0 {
            tab_bar_face.font_ascent = frame_params.char_height * 0.8;
        }
        if tab_bar_face.font_line_height <= 0.0 {
            tab_bar_face.font_line_height = frame_params.char_height.max(tab_bar_face.font_ascent);
        }
        let chrome_before_tab = frame_params.menu_bar_height
            + frame_params.tool_bar_height
            + frame_params.compact_bar_height;
        let row_index = if frame_params.char_height > 0.0 {
            (chrome_before_tab / frame_params.char_height)
                .round()
                .max(0.0) as u32
        } else {
            0
        };
        let tab_bar_y = chrome_before_tab;
        let mut current_face_id = self.frame_face_id_counter.max(BasicFaceId::SENTINEL);
        let tab_bar_spec = DisplayRowSpec::from_base_face(
            DisplayRowGeometry {
                y: tab_bar_y,
                width,
                height: tab_bar_height,
                char_width: frame_params.char_width,
                ascent: tab_bar_face.font_ascent,
                tab_policy: DisplayTabPolicy::every(8),
            },
            &mut current_face_id,
            &tab_bar_face,
            GlyphRowRole::TabBar,
            std::collections::HashMap::new(),
        );
        let Some(rendered) = self.render_display_text_fragment_row_with_display_host(
            tab_bar_spec,
            DisplayTextFragment::tab_bar(tab_bar.text),
            face_resolver,
            evaluator.display_host.as_deref(),
            &mut current_face_id,
        ) else {
            return None;
        };
        self.frame_face_id_counter = current_face_id;
        if rendered.row.glyphs[neomacs_display_protocol::glyph_matrix::GlyphArea::Text.index()]
            .is_empty()
        {
            return None;
        }
        let measured = MeasuredDisplayRow::new(
            DisplayRowOwner::FrameChrome {
                kind: FrameChromeKind::TabBar,
            },
            row_index,
            Rect::new(0.0, tab_bar_y, width, tab_bar_height),
            rendered,
            DisplayRowBoundsPolicy::MeasureContent,
        );
        let actual_tab_bar_height = measured.bounds.height;
        install_measured_frame_chrome_row(
            &mut self.matrix_builder,
            &mut self.pending_frame_chrome_rows,
            &measured,
        );
        self.pending_tab_bar = Some(neomacs_display_protocol::frame_glyphs::FrameTabBarState {
            items: tab_bar.items,
            y: tab_bar_y,
            height: actual_tab_bar_height,
        });
        Some(actual_tab_bar_height)
    }

    /// Layout a MockFrameContent into FrameDisplayState snapshots.
    ///
    /// This is the mock-display entry point.  The real neomacs GUI pipeline
    /// goes through `layout_frame_rust()` which takes a live Lisp evaluator.
    pub fn layout_mock_frame(
        &mut self,
        content: &super::mock_frame::MockFrameContent,
        char_w: f32,
        char_h: f32,
    ) -> Vec<neomacs_display_protocol::glyph_matrix::FrameDisplayState> {
        use super::matrix_builder::GlyphMatrixBuilder;
        use super::mock_frame::MockDisplayProperty;
        use neomacs_display_protocol::face::FaceAttributes;
        use neomacs_display_protocol::glyph_matrix::Glyph;
        use neomacs_display_protocol::types::Color;

        let font_metrics = self.font_metrics.as_mut();
        let mut builder = GlyphMatrixBuilder::new();

        builder.set_frame_identity(
            content.frame_id,
            0,
            0.0,
            0.0,
            0,
            false,
            0.0,
            Color::BLACK,
            1.0,
            false,
        );
        builder.set_background_color(content.background);

        let mut face_map = std::collections::HashMap::new();
        for face in &content.faces {
            let mut f = face.clone();
            // Convert points to physical pixels so the glyph atlas renders
            // at the same DPI-aware size the layout engine measured.
            f.font_size = crate::fontconfig::points_to_pixels(f.font_size);
            face_map.insert(f.id, f);
        }
        builder.set_faces(face_map);

        let default_face = content.faces.first();
        // Face.font_size is in points (matching GNU Emacs).  Convert to
        // physical pixels via fontconfig DPI, same as GNU's POINT_TO_PIXEL.
        let default_size =
            crate::fontconfig::points_to_pixels(default_face.map(|f| f.font_size).unwrap_or(12.0));
        let default_family = default_face
            .map(|f| f.font_family.as_str())
            .unwrap_or("monospace");
        let default_weight = default_face.map(|f| f.font_weight).unwrap_or(400);
        let default_italic = default_face
            .map(|f| f.attributes.contains(FaceAttributes::ITALIC))
            .unwrap_or(false);

        let ascent = font_metrics
            .and_then(|fm| {
                let m =
                    fm.font_metrics(default_family, default_weight, default_italic, default_size);
                Some(m.ascent)
            })
            .unwrap_or(char_h * 0.8);
        tracing::info!(
            "layout_mock_frame: default_size={:.1} family={} weight={} italic={} char_w={:.1} char_h={:.1}",
            default_size,
            default_family,
            default_weight,
            default_italic,
            char_w,
            char_h
        );

        // Per-window layout.
        //
        // Row metrics (pixel_y, height, ascent) must be set so the
        // renderer knows where to place each row.  Text rows stack from
        // the window top; the mode-line is pinned to the window bottom.
        for window in &content.windows {
            let nrows = window.lines.len();
            let ncols = (window.pixel_bounds.width / char_w.max(1.0)) as usize;
            builder.begin_window(
                window.window_id,
                nrows,
                ncols,
                window.pixel_bounds,
                window.selected,
            );
            for (row_idx, line) in window.lines.iter().enumerate() {
                builder.begin_row(row_idx, GlyphRowRole::Text);
                builder.set_current_row_metrics(
                    window.pixel_bounds.y + row_idx as f32 * char_h,
                    char_h,
                    ascent,
                );
                let lnum = format!("{:>3} ", row_idx + 1);
                for ch in lnum.chars() {
                    builder.push_left_margin_char(ch, 2);
                }
                let mut cp = 0usize;
                for glyph in &line.glyphs {
                    match &glyph.display {
                        Some(MockDisplayProperty::Invisible) => {
                            cp += 1;
                            continue;
                        }
                        Some(MockDisplayProperty::Replace(text, fid)) => {
                            for ch in text.chars() {
                                builder.push_char(ch, *fid, cp);
                                cp += 1;
                            }
                            continue;
                        }
                        Some(MockDisplayProperty::Composition(composed)) => {
                            for cg in composed {
                                builder.push_char(cg.ch, cg.face_id, cp);
                                cp += 1;
                            }
                            continue;
                        }
                        _ => {}
                    }
                    builder.push_char(glyph.ch, glyph.face_id, cp);
                    cp += 1;
                }
                builder.end_row();
            }

            // Mode-line pinned to window bottom.
            builder.begin_status_line_row(GlyphRowRole::ModeLine);
            builder.set_current_row_metrics(
                window.pixel_bounds.y + window.pixel_bounds.height - char_h,
                char_h,
                ascent,
            );
            let ml_ncols = (window.pixel_bounds.width / char_w.max(1.0)) as usize;
            let mut ml: Vec<Glyph> = window
                .mode_line
                .glyphs
                .iter()
                .map(|g| Glyph::char(g.ch, g.face_id, 0))
                .collect();
            while ml.len() < ml_ncols {
                ml.push(Glyph::char(' ', 1, 0));
            }
            ml.truncate(ml_ncols);
            builder.install_status_line_row_glyphs(ml);

            builder.end_window();
        }

        // Minibuffer at frame bottom — a real window with text rows
        // and optionally a thin mode-line, matching GNU's design where
        // the echo-area text is buffer content, not a mode-line.
        if let Some(ref mini) = content.minibuffer {
            let nrows = mini.lines.len();
            let ncols = (mini.pixel_bounds.width / char_w.max(1.0)) as usize;
            builder.begin_window(
                mini.window_id,
                nrows,
                ncols,
                mini.pixel_bounds,
                mini.selected,
            );

            for (row_idx, line) in mini.lines.iter().enumerate() {
                builder.begin_row(row_idx, GlyphRowRole::Minibuffer);
                builder.set_current_row_metrics(
                    mini.pixel_bounds.y + row_idx as f32 * char_h,
                    char_h,
                    ascent,
                );
                let mut cp = 0usize;
                for glyph in &line.glyphs {
                    builder.push_char(glyph.ch, glyph.face_id, cp);
                    cp += 1;
                }
                builder.end_row();
            }

            if !mini.mode_line.glyphs.is_empty() {
                builder.begin_status_line_row(GlyphRowRole::ModeLine);
                builder.set_current_row_metrics(
                    mini.pixel_bounds.y + mini.pixel_bounds.height - char_h,
                    char_h,
                    ascent,
                );
                let mini_ncols = (mini.pixel_bounds.width / char_w.max(1.0)) as usize;
                let mut ml: Vec<Glyph> = mini
                    .mode_line
                    .glyphs
                    .iter()
                    .map(|g| Glyph::char(g.ch, g.face_id, 0))
                    .collect();
                while ml.len() < mini_ncols {
                    ml.push(Glyph::char(' ', 1, 0));
                }
                ml.truncate(mini_ncols);
                builder.install_status_line_row_glyphs(ml);
            }

            builder.end_window();
        }

        let main_state = builder.finish(
            (content.frame_pixel_width / char_w.max(1.0)) as usize,
            (content.frame_pixel_height / char_h.max(1.0)) as usize,
            char_w,
            char_h,
        );

        let mut child_frames = Vec::new();
        for cf in &content.child_frames {
            let mut cb = GlyphMatrixBuilder::new();
            cb.set_frame_identity(
                cf.frame_id,
                content.frame_id,
                cf.parent_x,
                cf.parent_y,
                cf.z_order,
                true,
                0.0,
                Color::BLACK,
                1.0,
                false,
            );
            cb.set_background_color(Color::new(0.0, 0.0, 0.0, 0.0));
            let mut cfm = std::collections::HashMap::new();
            for face in &content.faces {
                cfm.insert(face.id, face.clone());
            }
            cb.set_faces(cfm);
            let nrows = cf.window.lines.len();
            let ncols = (cf.window.pixel_bounds.width / char_w.max(1.0)) as usize;
            cb.begin_window(
                cf.window.window_id,
                nrows,
                ncols,
                cf.window.pixel_bounds,
                false,
            );
            for (ri, line) in cf.window.lines.iter().enumerate() {
                cb.begin_row(ri, GlyphRowRole::Text);
                let mut cp = 0usize;
                for g in &line.glyphs {
                    cb.push_char(g.ch, g.face_id, cp);
                    cp += 1;
                }
                cb.end_row();
            }
            cb.end_window();
            let cs = cb.finish(
                (cf.window.pixel_bounds.width / char_w.max(1.0)) as usize,
                cf.window.lines.len().max(1),
                char_w,
                char_h,
            );
            child_frames.push(cs);
        }

        let mut all = vec![main_state];
        all.extend(child_frames);
        all
    }
}

#[cfg(test)]
#[path = "engine_test.rs"]
mod tests;
