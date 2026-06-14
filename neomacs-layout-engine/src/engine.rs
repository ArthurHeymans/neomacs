//! The Rust layout engine — Phase 1+2: Monospace layout with face resolution.
//!
//! Reads buffer text and display state from neovm-core, resolves faces per
//! character position, computes line breaks, positions glyphs on a fixed-width
//! grid, and publishes `FrameDisplayState` snapshots for render backends.

#[cfg(test)]
use super::display_status_line::eval_status_line_format;
use super::display_status_line::{
    EchoMinibufferDisplayRowsRequest, FrameTabBarDisplayRowRender, FrameTabBarDisplayRowRequest,
    InactiveMinibufferDisplayRowRequest, ResizeMiniWindowsMode, ScratchGcRootScope,
    WindowChromeDisplayRowRequest, WindowChromeDisplayText, build_tab_bar_display,
    eval_status_line_format_value, max_mini_window_lines, message_truncate_lines,
    minibuffer_echo_message_for_window, minibuffer_resize_line_count,
};
use super::font_metrics::FontMetricsService;
use super::gui_chrome::{collect_gui_menu_bar_items_for_frame, collect_gui_tool_bar_items};
use super::hit_test::*;
use super::types::*;
#[cfg(test)]
use super::window_output::RowMetricsSnapshot;
use super::window_output::{
    ChromeRowOutput, TextWindowBegin, TextWindowCursor, TextWindowCursorEffects,
    TextWindowDecorativeCursor, TextWindowLineNumberMargin, TextWindowOutputInstall,
    TextWindowPendingRowFinish, TextWindowRedisplayPositions, TextWindowRightBorder,
    TextWindowRightEdgeMarkerColumn, TextWindowRightEdgeMarkers, WindowOutputEmitter,
    begin_text_window_output, close_text_window_output, current_text_window_cluster_tail,
    emit_text_window_line_number_margin, finish_pending_text_window_row,
    install_last_window_right_border, install_text_window_cursor_effects,
    install_text_window_output, mark_current_text_row_truncated_left, publish_text_window_cursor,
    publish_text_window_decorative_cursor, record_text_window_redisplay_positions,
};
use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
#[cfg(test)]
use crate::display_cursor::CapturedCursorVisualState;
#[cfg(test)]
use crate::display_cursor::CursorSlotWidthPolicy;
#[cfg(test)]
use crate::display_cursor::resolve_cursor_vertical_metrics;
use crate::display_cursor::{
    CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth, CursorCaptureState,
    CursorGeometryContext, CursorGeometrySource, capture_cursor_info, cursor_style_for_visual,
    cursor_style_for_window, resolve_cursor_geometry, row_metrics_for_cursor,
    visual_cursor_source_from_point,
};
#[cfg(test)]
use crate::display_cursor::{CursorSlotWidthRequest, VisualCursorGeometryContext};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_layout::{DisplayHeightFaceBasis, height_adjusted_face};
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFace, DisplayRowFallbackMetrics,
    DisplayRowMeasurementPolicy, WindowChromeKind, insert_resolved_display_row_face,
};
#[cfg(test)]
use crate::display_row_append::OverlayStringRenderSource;
use crate::display_row_append::{
    BufferDisplayPropertyTextAppendAction, BufferDisplayPropertyTextRenderContext,
    BufferHscrollSkipAction, BufferHscrollSkipSourceChar, BufferInvisibleTextScanAction,
    BufferInvisibleTextScanContext, BufferLinePrefixRenderContext,
    BufferOverlayStringRenderContext, BufferSelectiveDisplayContext,
    BufferSyntheticTextRenderContext, BufferTextCharacterWrapSourceAction,
    BufferTextLineBreakSourceAction, BufferTextPreparedSourceCharAppend,
    BufferTextRowAppendContext, BufferTextRowAppendState, BufferTextSourceChar,
    BufferTextSourceCharOverflowAction, BufferTextSpecialSourceCharOverflowAction,
    BufferTextTruncationSkipAction, BufferTextWordWrapSourceAction,
    DisplayRowLineBreakTransitionPlan, DisplayRowPrefixRequest, DisplayRowPrefixValues,
    DisplayRowTextWindowEmitContext, DisplayRowTextWindowTransitionContext,
    DisplayRowTransitionPrefixContext, SyntheticTextMarker, TextWindowAppendSurfaceRequest,
};
use crate::display_row_builder::{
    DisplayRowLayout, DisplayRowPosition, DisplayRowWriter, DisplayTabPolicy,
    display_row_text_glyph_count, new_display_row,
};
use crate::display_row_geometry::{
    DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults, DisplayRowLimit,
    DisplayRowScopedValue, DisplayRowVisibilityLimit, DisplayRowYPositions,
};
#[cfg(test)]
use crate::display_row_geometry::{DisplayRowHitRange, DisplayRowMarker, DisplayRowStartMarker};
#[cfg(test)]
use crate::display_row_walk_state::WordWrapBreakCandidate;
use crate::display_row_walk_state::{
    ActiveDisplayPropertySpan, BoxFaceRowState, FaceScanCheckpoint, HitRowRangeTracker,
    HorizontalScrollSkipState, LineNumberRenderState, TextPropertyScanCheckpoints,
    TrailingWhitespaceRenderState, WordWrapRenderState,
    next_window_start_for_partially_visible_point_row,
    next_window_start_for_point_line_continuation, next_window_start_from_visible_rows,
};
use crate::fontconfig::FontSizing;
use neomacs_display_protocol::face::BasicFaceId;
#[cfg(test)]
use neomacs_display_protocol::frame_glyphs::CursorStyle;
#[cfg(test)]
use neomacs_display_protocol::frame_glyphs::DisplaySlotId;
use neomacs_display_protocol::frame_glyphs::{
    FrameGlyphBuffer, GlyphRowRole, WindowEffectHint, WindowInfo, WindowTransitionHint,
    WindowTransitionKind,
};
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow, ScrollBarItem};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::Value;
use neovm_core::window::{WindowDisplaySnapshot, WindowId};

/// Bound redisplay convergence work when point begins outside the visible span.
const MAX_WINDOW_VISIBILITY_RETRIES: usize = 128;

#[derive(Clone, Copy, Debug)]
struct ScrollBarMetrics {
    position: i64,
    portion: i64,
    whole: i64,
    thumb_start: f32,
    thumb_size: f32,
}

#[cfg(test)]
#[inline]
fn cursor_point_columns(text: &[u8], byte_idx: usize, col: i32, params: &WindowParams) -> usize {
    CursorSlotWidthRequest::from_window_params(CursorStyle::FilledBox, text, byte_idx, col, params)
        .point_columns()
}

#[cfg(test)]
#[inline]
fn cursor_width_for_style(
    style: CursorStyle,
    text: &[u8],
    byte_idx: usize,
    col: i32,
    params: &WindowParams,
    face_char_w: f32,
) -> f32 {
    CursorSlotWidthRequest::from_window_params(style, text, byte_idx, col, params)
        .width_px(face_char_w)
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
    pub(crate) pending_frame_chrome_rows:
        Vec<neomacs_display_protocol::glyph_matrix::FrameChromeRow>,
    /// Frame-level tab bar metadata for render-thread hit-testing.
    pending_tab_bar: Option<neomacs_display_protocol::frame_glyphs::FrameTabBarState>,
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
                        install_last_window_right_border(
                            &mut self.matrix_builder,
                            TextWindowRightBorder {
                                ch: '|',
                                face_id: border_face_id,
                                char_width: frame_params.char_width,
                            },
                        );
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
            install_text_window_cursor_effects(
                &mut self.matrix_builder,
                TextWindowCursorEffects {
                    window_id: params.window_id,
                    effects,
                },
            );
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

        let prefix_values = DisplayRowPrefixValues::default_values(
            super::neovm_bridge::buffer_local_value(buffer, "line-prefix"),
            super::neovm_bridge::buffer_local_value(buffer, "wrap-prefix"),
        );
        let has_prefix = prefix_values.has_default_prefix();

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
        let mut face_scan = FaceScanCheckpoint::initial();
        // Load the frame-wide face-id counter so this window's
        // glyph/mode-line/header-line faces get IDs that do NOT
        // collide with earlier siblings' faces in the frame-scoped
        // `matrix_builder.faces` HashMap. Write back below before
        // returning. Mirrors GNU's single `face_cache->used`
        // counter per frame at `src/xfaces.c::lookup_face` /
        // `init_frame_faces`.
        let mut face_ids = FrameFaceIdAllocator::new(self.frame_face_id_counter);
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
            self.render_echo_minibuffer_window(
                face_resolver,
                evaluator.display_host.as_deref(),
                &mut face_ids,
                EchoMinibufferDisplayRowsRequest {
                    window_id: params.window_id as u64,
                    window_bounds: params.bounds,
                    text_bounds: params.text_bounds,
                    selected: params.selected,
                    text_width,
                    char_width: char_w,
                    ascent: default_face_ascent,
                    row_height: char_h,
                    base_face: default_resolved,
                    message: echo_message,
                    max_rows: max_mini,
                    truncate_lines: truncate_echo_lines,
                    reserve_right_special_col,
                },
            );
            return;
        }

        if params.is_minibuffer && !active_minibuffer_window {
            // GNU `display_echo_area` temporarily displays an echo-area
            // buffer in the minibuffer window.  With no current message that
            // buffer is empty; the inactive minibuffer must not redisplay the
            // ordinary buffer attached to the window record.
            self.render_inactive_minibuffer_window(
                face_resolver,
                evaluator.display_host.as_deref(),
                &mut face_ids,
                InactiveMinibufferDisplayRowRequest {
                    window_id: params.window_id as u64,
                    window_bounds: params.bounds,
                    text_bounds: params.text_bounds,
                    selected: params.selected,
                    text_width,
                    row_height: char_h,
                    char_width: char_w,
                    ascent: default_face_ascent,
                    base_face: default_resolved,
                },
            );
            return;
        }

        // Line number state
        let window_start_byte = buf_access.charpos_to_bytepos(window_start);
        let begin_byte = if lnum_widen { 0 } else { buf_access.begv() };
        let current_line: i64 = if lnum_enabled {
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
        let mut line_numbers = LineNumberRenderState::new(lnum_enabled, current_line, point_line);

        // Simple monospace text layout
        let mut x = content_x;
        let mut col = 0usize;
        let mut byte_idx = 0usize;
        let mut charpos = window_start;
        let mut text_property_checkpoints = TextPropertyScanCheckpoints::new(window_start);

        // Display :raise property: vertical Y offset for glyphs
        let mut raise_span = ActiveDisplayPropertySpan::inactive();

        // Display :height property: font scale factor applied as a real face
        // transformation, matching GNU `face_with_height`.
        let mut height_span = ActiveDisplayPropertySpan::inactive();

        // Fringe state tracking
        let left_fringe_x = params.text_bounds.x - params.left_fringe_width;
        let right_fringe_x = params.text_bounds.x + params.text_bounds.width;
        let mut row_flags = DisplayRowFlags::new(max_rows);

        let mut hscroll_skip =
            HorizontalScrollSkipState::new(params.truncate_lines, params.hscroll);

        // Word-wrap break tracking
        let mut word_wrap = WordWrapRenderState::new(params.word_wrap);

        let mut prefix_request =
            DisplayRowPrefixRequest::initial(has_prefix, prefix_values.has_line_default_prefix());

        let reserve_right_special_col =
            !frame_params.window_system && params.right_fringe_width == 0.0;
        let text_append_surface = TextWindowAppendSurfaceRequest::new(
            content_x,
            text_width,
            lnum_pixel_width,
            reserve_right_border_col,
            reserve_right_special_col,
            char_w,
            params.tab_width,
            &params.tab_stop_list,
        )
        .into_surface();

        // Variable-height row tracking
        let row_geometry_defaults =
            DisplayRowGeometryDefaults::new(text_y, char_h, default_face_ascent);
        let mut row_geometry = row_geometry_defaults.initial_state();
        let mut row_y_positions =
            DisplayRowYPositions::with_capacity_and_first_row(max_rows, text_y);
        let mut trailing_whitespace = TrailingWhitespaceRenderState::new(
            params.show_trailing_whitespace,
            params.trailing_ws_bg,
        );
        // Exact joined-form advances for the current contextual-shaping run,
        // shaped once via shape_run and keyed by absolute byte offset (robust
        // to wrap re-processing). Empty/unused for non-complex text.
        let mut buffer_text_append_state = BufferTextRowAppendState::default();

        // Check if the buffer has any overlays (optimization: skip per-char overlay checks if empty)
        let has_overlays = !buffer.overlays().is_empty();

        // Face :extend tracking — extends face background to end of line
        let mut row_extend = DisplayRowScopedValue::inactive();

        // Box face tracking: track active :box face regions
        let mut box_face = BoxFaceRowState::inactive();

        // Cursor metrics captured during the main layout loop.
        let mut cursor_info = CursorCaptureState::new();

        // Hit-test data for this window
        let mut hit_rows: Vec<HitRow> = Vec::new();
        let mut hit_row_range = HitRowRangeTracker::new(window_start);
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
                if face_scan.should_resolve_at(charpos as usize) {
                    let mut resolved = face_resolver.face_at_pos(
                        buffer,
                        charpos as usize,
                        face_scan.next_check_mut(),
                    );
                    if let Some(factor) = height_span.value()
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
                    let face_id = face_ids.allocate();

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
                    let face_metrics = active_face_state.metrics();
                    row_geometry.include_row_extents(face_metrics.row_height, face_metrics.ascent);

                    if resolved.extend {
                        let ext_bg = Color::from_pixel(resolved.bg);
                        row_extend.activate(row_geometry.current_row_marker(), (ext_bg, face_id));
                    }

                    if box_face.is_active() && resolved.box_type == 0 {
                        box_face.clear();
                    }
                    if resolved.box_type > 0 {
                        box_face.activate(row_geometry.current_row_marker(), x);
                    }
                }
            };
        }

        macro_rules! save_word_wrap_candidate {
            ($ch:expr, $break_byte_idx:expr) => {
                if word_wrap.can_record_candidate($ch) {
                    word_wrap.record_candidate(
                        $ch,
                        $break_byte_idx,
                        charpos,
                        output_emitter.display_point_len(),
                        output_emitter.current_row_display_positions(),
                    );
                }
            };
        }

        macro_rules! overlay_string_context {
            () => {
                BufferOverlayStringRenderContext::for_text_row(
                    has_overlays,
                    params.window_id as u64,
                    &text_append_surface,
                    &active_face_state,
                    char_h,
                    default_face_ascent,
                    text_y,
                    text_matrix_row_base,
                    max_rows,
                )
            };
        }

        macro_rules! synthetic_text_context {
            ($glyph_y_offset:expr) => {
                BufferSyntheticTextRenderContext::new(
                    &text_append_surface,
                    &active_face_state,
                    $glyph_y_offset,
                    char_h,
                    default_face_ascent,
                    char_w,
                )
            };
        }

        // --- GlyphMatrix builder: begin window and first row ---
        let matrix_rows = text_matrix_row_base + text_matrix_rows + bottom_chrome_rows;
        let matrix_cols = cols.max(1);
        begin_text_window_output(
            &mut self.matrix_builder,
            &mut output_emitter,
            evaluator,
            TextWindowBegin {
                window_id: params.window_id as u64,
                rows: matrix_rows,
                cols: matrix_cols,
                bounds: params.bounds,
                text_bounds: params.text_bounds,
                selected: params.selected,
                first_row: row_geometry.text_matrix_row_begin(text_matrix_row_base, col, x),
            },
        );

        let row_visibility_limit = DisplayRowVisibilityLimit {
            max_rows,
            bottom_y: text_y + text_height,
        };
        let row_limit = DisplayRowLimit { max_rows };

        while byte_idx < text.len() && row_geometry.current_row_is_visible(row_visibility_limit) {
            if let Some(line_number_request) = line_numbers.margin_render_request(
                lnum_mode,
                lnum_current_absolute,
                lnum_offset,
                lnum_major_tick,
                lnum_cols,
            ) {
                let lnum_face =
                    face_resolver.resolve_named_face(line_number_request.face().face_name());
                let _lnum_bg = Color::from_pixel(lnum_face.bg);
                let lnum_face_id = face_ids.allocate();
                insert_resolved_display_row_face(
                    &mut self.matrix_builder,
                    lnum_face_id,
                    &lnum_face,
                    None,
                );

                let num_str = line_number_request.text();
                emit_text_window_line_number_margin(
                    &mut self.matrix_builder,
                    TextWindowLineNumberMargin {
                        text: &num_str,
                        cols: line_number_request.cols(),
                        face_id: lnum_face_id,
                        row_y: row_geometry.y(),
                        row_height: row_geometry.height(),
                        row_ascent: row_geometry.ascent(),
                        char_width: char_w,
                    },
                );
                face_scan.invalidate();

                line_numbers.consume_render_request();
            }

            // --- Line/wrap prefix rendering ---
            if prefix_request.is_requested() {
                let position = BufferLinePrefixRenderContext::new(
                    prefix_values,
                    &text_append_surface,
                    &row_geometry,
                    &active_face_state,
                    raise_span.value_or(0.0),
                    char_h,
                )
                .render_requested_to_text_row_and_emit(
                    &mut prefix_request,
                    evaluator,
                    &mut output_emitter,
                    buffer,
                    charpos,
                    &mut self.font_metrics,
                    face_resolver,
                    &mut face_ids,
                    &mut self.matrix_builder,
                    DisplayRowPosition { x_px: x, col },
                );
                x = position.x_px;
                col = position.col;
            }

            // --- Invisible text check ---
            if let BufferInvisibleTextScanAction::Hidden(hidden_text) =
                BufferInvisibleTextScanContext::new(
                    text,
                    accessible_end,
                    point_charpos,
                    cursor_info.is_missing(),
                )
                .consume_at_checkpoint(
                    buffer,
                    &mut text_property_checkpoints,
                    &mut byte_idx,
                    &mut charpos,
                )
            {
                if hidden_text.point_in_hidden_region() {
                    capture_cursor_info(
                        &mut cursor_info,
                        CapturedCursorInfo::from_active_face_state(
                            &active_face_state,
                            CapturedCursorPlacement::from_row_text_position(
                                row_geometry.text_position(x, hidden_text.start_byte_idx(), col),
                                CapturedCursorSlotWidth::FaceChar,
                                false,
                            ),
                        ),
                    );
                }

                // GNU displays ellipsis only when the matching
                // `buffer-invisibility-spec' entry requests it.
                if hidden_text.ellipsis() {
                    if let Some(position) = synthetic_text_context!(raise_span.value_or(0.0))
                        .render_active_marker_to_text_row(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            evaluator,
                            &mut self.font_metrics,
                            face_resolver,
                            &row_geometry,
                            DisplayRowPosition { x_px: x, col },
                            SyntheticTextMarker::InvisibleEllipsis,
                        )
                    {
                        x = position.x_px;
                        col = position.col;
                    }
                }

                // Check for overlay strings at invisible region boundary.
                // Packages like org-mode use overlay after-strings at invisible
                // boundaries to show fold indicators (e.g. "[N lines]").
                overlay_string_context!().render_after_at(
                    evaluator,
                    &mut output_emitter,
                    buffer,
                    charpos,
                    &mut self.font_metrics,
                    face_resolver,
                    &mut x,
                    &mut col,
                    &mut row_geometry,
                    &mut cursor_info,
                    &mut hit_rows,
                    &mut hit_row_range,
                    &mut row_y_positions,
                    &mut face_ids,
                    &mut self.matrix_builder,
                );
                continue;
            }

            // Handle hscroll: skip columns consumed by horizontal scroll
            if hscroll_skip.should_skip() {
                let Some(hscroll_action) = BufferHscrollSkipSourceChar::consume_from_text(
                    text,
                    &mut byte_idx,
                    &mut charpos,
                    &mut hscroll_skip,
                    params.tab_width,
                ) else {
                    break;
                };
                let ch_start_byte_idx = hscroll_action.ch_start_byte_idx();

                if matches!(hscroll_action, BufferHscrollSkipAction::LineBreak { .. }) {
                    x = content_x;
                    // Record newline position on the row (see main \n handler).
                    output_emitter
                        .note_display_buffer_pos(LispCharPos1::new(hscroll_action.charpos()));
                    row_extend.clear();

                    let line_break_transition =
                        DisplayRowLineBreakTransitionPlan::hscroll_line_break();
                    let hit_range = hit_row_range.range_to(hscroll_action.charpos());
                    // Record hit-test row (hscroll newline)
                    hit_row_range.advance_to(hscroll_action.charpos());
                    let row_transition = DisplayRowTextWindowEmitContext::new(
                        row_geometry_defaults,
                        text_matrix_row_base,
                        &mut row_y_positions,
                        max_rows,
                        &mut row_geometry,
                        &mut row_flags,
                        row_limit,
                        &mut hit_rows,
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_line_break(
                        line_break_transition,
                        hit_range,
                        DisplayRowPosition { x_px: x, col },
                        0.0,
                    );
                    if row_transition.is_exhausted() {
                        break;
                    }
                    let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                        &mut prefix_request,
                        has_prefix,
                        &mut line_numbers,
                        &mut hscroll_skip,
                        &mut word_wrap,
                        &mut trailing_whitespace,
                    );
                    line_break_transition
                        .apply_row_start_prefix_action(&mut col, &mut transition_prefix);
                    if cursor_info.is_missing() && point_charpos == hscroll_action.charpos() {
                        capture_cursor_info(
                            &mut cursor_info,
                            CapturedCursorInfo::line_break_from_active_face_state(
                                &active_face_state,
                                CapturedCursorPlacement::from_row_text_position(
                                    row_geometry.text_position(x, ch_start_byte_idx, col),
                                    CapturedCursorSlotWidth::FaceChar,
                                    false,
                                ),
                                char_h,
                            ),
                        );
                    }
                } else {
                    // When hscroll is exhausted, show $ indicator at left edge
                    if hscroll_action.should_show_left_truncation() {
                        if let Some(position) = synthetic_text_context!(0.0)
                            .render_hscroll_truncation_marker_to_text_row(
                                &mut self.matrix_builder,
                                &mut output_emitter,
                                evaluator,
                                &mut self.font_metrics,
                                face_resolver,
                                &row_geometry,
                                content_x,
                            )
                        {
                            x = position.x_px;
                            col = position.col;
                        }
                        mark_current_text_row_truncated_left(&mut self.matrix_builder);
                    }
                    if cursor_info.is_missing() && point_charpos == hscroll_action.charpos() {
                        capture_cursor_info(
                            &mut cursor_info,
                            CapturedCursorInfo::from_active_face_state(
                                &active_face_state,
                                CapturedCursorPlacement::from_row_text_position(
                                    row_geometry.text_position(x, ch_start_byte_idx, col),
                                    CapturedCursorSlotWidth::FaceChar,
                                    false,
                                ),
                            ),
                        );
                    }
                }
                continue;
            }

            // --- Display property check ---
            // Only call check_display_prop at property change boundaries for efficiency
            if height_span.clear_if_expired(charpos, window_start) {
                face_scan.invalidate();
            }
            resolve_current_face_state!();
            match BufferDisplayPropertyTextRenderContext::new(
                buf_id,
                text_start_byte,
                text,
                &active_face_state,
                x,
                content_x,
                params,
                raise_span.value_or(0.0),
                char_h,
                DisplayRowPosition { x_px: x, col },
            )
            .resolve_and_append_at_checkpoint(
                buffer,
                evaluator,
                &mut output_emitter,
                &mut self.matrix_builder,
                &mut self.font_metrics,
                face_resolver,
                &mut face_ids,
                &text_append_surface,
                &mut row_geometry,
                &mut text_property_checkpoints,
                charpos,
                byte_idx,
                accessible_end,
            ) {
                BufferDisplayPropertyTextAppendAction::Replacement(replacement_outcome) => {
                    if cursor_info.is_missing()
                        && replacement_outcome.point_in_replacement(point_charpos, charpos)
                    {
                        let start_position = replacement_outcome.start_position();
                        capture_cursor_info(
                            &mut cursor_info,
                            replacement_outcome.cursor_info(
                                &active_face_state,
                                row_geometry.text_position(
                                    start_position.x_px,
                                    byte_idx,
                                    start_position.col,
                                ),
                            ),
                        );
                    }
                    let position = replacement_outcome.end_position();
                    x = position.x_px;
                    col = position.col;

                    // Skip covered buffer text
                    replacement_outcome.skip_covered_buffer_text(text, &mut byte_idx, &mut charpos);
                    continue;
                }
                BufferDisplayPropertyTextAppendAction::Modifiers(modifiers) => {
                    if let Some(raise_offset_px) = modifiers.raise_offset_px() {
                        raise_span.set(raise_offset_px, modifiers.next_change());
                    }
                    if let Some(factor) = modifiers.height_factor() {
                        height_span.set(factor, modifiers.next_change());
                        face_scan.invalidate();
                        resolve_current_face_state!();
                    }
                }
                BufferDisplayPropertyTextAppendAction::None => {}
            }

            // Decode UTF-8 character. Keep the original byte/char position so
            // character-wrap can resume from the same buffer position on the
            // next visual row, like GNU Emacs restoring its iterator state.
            let ch_start_byte_idx = byte_idx;
            let ch_start_charpos = charpos;
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
            let selective_display_context =
                BufferSelectiveDisplayContext::new(text, selective_display, params.tab_width);
            if selective_display_context.hides_carriage_return_tail(ch) {
                if let Some(position) = synthetic_text_context!(raise_span.value_or(0.0))
                    .render_active_marker_to_text_row(
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                        &mut self.font_metrics,
                        face_resolver,
                        &row_geometry,
                        DisplayRowPosition { x_px: x, col },
                        SyntheticTextMarker::SelectiveEllipsis,
                    )
                {
                    x = position.x_px;
                    col = position.col;
                }
                // Skip remaining chars until newline
                let selective_tail_action = selective_display_context
                    .skip_rest_of_line_after_carriage_return(&mut byte_idx, &mut charpos);
                if selective_tail_action.is_line_break() {
                    // Advance to next row (same as newline handler)
                    x = content_x;
                    row_extend.clear();
                    box_face.continue_on_row(row_geometry.next_row_marker(), content_x);
                    let line_break_transition =
                        DisplayRowLineBreakTransitionPlan::hidden_line_break();
                    let row_transition = DisplayRowTextWindowEmitContext::new(
                        row_geometry_defaults,
                        text_matrix_row_base,
                        &mut row_y_positions,
                        max_rows,
                        &mut row_geometry,
                        &mut row_flags,
                        row_limit,
                        &mut hit_rows,
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_line_break(
                        line_break_transition,
                        hit_row_range.range_to(charpos),
                        DisplayRowPosition { x_px: x, col },
                        0.0,
                    );
                    if row_transition.is_exhausted() {
                        break;
                    }
                    charpos = sync_charpos_from_byte_idx(byte_idx);
                    hit_row_range.advance_to(charpos);
                    let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                        &mut prefix_request,
                        has_prefix,
                        &mut line_numbers,
                        &mut hscroll_skip,
                        &mut word_wrap,
                        &mut trailing_whitespace,
                    );
                    line_break_transition
                        .apply_row_start_prefix_action(&mut col, &mut transition_prefix);
                }
                continue;
            }

            save_word_wrap_candidate!(ch, ch_start_byte_idx);

            if ch == '\n' {
                let line_break_action = BufferTextLineBreakSourceAction::for_newline(
                    buffer,
                    charpos,
                    ch_start_byte_idx,
                    char_h,
                    params.extra_line_spacing,
                );
                if cursor_info.is_missing() && line_break_action.point_matches(point_charpos) {
                    // GNU `set_cursor_from_row` treats the terminating
                    // newline as an exact match for point on this row.  The
                    // newline itself has no rendered text glyph, so the
                    // physical cursor uses the row-end cell width instead of
                    // waiting for the next row.
                    capture_cursor_info(
                        &mut cursor_info,
                        line_break_action.cursor_info(&active_face_state, &row_geometry, x, col),
                    );
                }
                // Highlight trailing whitespace before advancing to next row
                if let Some((_tw_bg, tw_x)) = trailing_whitespace.highlight_start_x(&row_geometry) {
                    let tw_w = x - tw_x;
                    if tw_w > 0.0 {}
                }
                trailing_whitespace.reset_after_row_transition();

                // Face :extend: fill rest of row with extending face background
                if let Some((_ext_bg, _ext_face_id)) = row_extend.value_on(&row_geometry) {
                    let right_edge = text_append_surface.right_edge();
                    if x < right_edge {}
                }
                row_extend.clear();

                // Box face tracking: box stays active across line breaks
                box_face.continue_on_row(row_geometry.current_row_marker(), content_x);

                charpos = line_break_action.next_charpos();
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
                let line_break_transition = DisplayRowLineBreakTransitionPlan::line_break();
                let row_transition = DisplayRowTextWindowEmitContext::new(
                    row_geometry_defaults,
                    text_matrix_row_base,
                    &mut row_y_positions,
                    max_rows,
                    &mut row_geometry,
                    &mut row_flags,
                    row_limit,
                    &mut hit_rows,
                    &mut self.matrix_builder,
                    &mut output_emitter,
                    evaluator,
                )
                .emit_line_break(
                    line_break_transition,
                    hit_row_range.range_to(charpos),
                    DisplayRowPosition { x_px: x, col },
                    line_break_action.line_spacing(),
                );
                if row_transition.is_exhausted() {
                    break;
                }
                charpos = sync_charpos_from_byte_idx(byte_idx);
                hit_row_range.advance_to(charpos);
                box_face.continue_on_row(row_geometry.current_row_marker(), content_x);
                let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                    &mut prefix_request,
                    has_prefix,
                    &mut line_numbers,
                    &mut hscroll_skip,
                    &mut word_wrap,
                    &mut trailing_whitespace,
                );
                line_break_transition
                    .apply_row_start_prefix_action(&mut col, &mut transition_prefix);
                // Selective display: skip lines indented beyond threshold
                let selective_display_context =
                    BufferSelectiveDisplayContext::new(text, selective_display, params.tab_width);
                if selective_display_context.hides_indented_lines_after_line_break(byte_idx) {
                    let hidden_lines = selective_display_context
                        .skip_hidden_indented_lines_after_line_break(&mut byte_idx, &mut charpos);
                    for _ in 0..hidden_lines.hidden_line_count() {
                        line_numbers.advance_hidden_line();
                    }
                }
                continue;
            }

            let buffer_source_char = BufferTextSourceChar::new(
                ch,
                CharPos0::new(charpos as usize),
                params.nobreak_char_display,
            );
            let buffer_row_append_context = BufferTextRowAppendContext::new(
                buffer,
                buf_id,
                &text_append_surface,
                &active_face_state,
                raise_span.value_or(0.0),
                char_h,
            );

            // Grapheme-cluster continuation is decided BEFORE glyphless
            // handling: a zero-width joiner / non-joiner / variation selector
            // that continues an emoji composition (the ZWJs in 👨‍👩‍👧, VS-16 in
            // ❤️ or keycaps) is a format char that glyphless classification would
            // otherwise SKIP, splitting the composition. GNU consumes such
            // characters into the active composition instead of drawing them
            // glyphless. Only suppress glyphless handling when there is a
            // preceding glyph to merge into — a standalone joiner still renders
            // glyphless.
            let cluster_tail = current_text_window_cluster_tail(&self.matrix_builder);
            let append_position = DisplayRowPosition { x_px: x, col };
            let append_geometry = row_geometry;

            let prepared_append = buffer_row_append_context.prepare_source_char_at(
                &append_geometry,
                &mut buffer_text_append_state,
                &mut self.matrix_builder,
                evaluator,
                &mut self.font_metrics,
                face_resolver,
                &buffer_source_char,
                &text,
                ch_start_byte_idx,
                append_position,
                cluster_tail,
            );
            let prepared_append = match prepared_append {
                BufferTextPreparedSourceCharAppend::Special(special_prepared_append) => {
                    if let Some(overflow_action) = special_prepared_append.overflow_action(
                        x,
                        text_append_surface.full_text_right_edge(),
                        params.truncate_lines,
                    ) {
                        match overflow_action {
                            BufferTextSpecialSourceCharOverflowAction::Fits => {}
                            BufferTextSpecialSourceCharOverflowAction::Truncate { transition } => {
                                let truncation_skip =
                                    BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
                                        text,
                                        &mut byte_idx,
                                        &mut charpos,
                                    );
                                if truncation_skip.reached_line_break() {
                                    line_numbers.advance_line();
                                }
                                x = content_x;
                                row_extend.clear();
                                let row_transition = DisplayRowTextWindowEmitContext::new(
                                    row_geometry_defaults,
                                    text_matrix_row_base,
                                    &mut row_y_positions,
                                    max_rows,
                                    &mut row_geometry,
                                    &mut row_flags,
                                    row_limit,
                                    &mut hit_rows,
                                    &mut self.matrix_builder,
                                    &mut output_emitter,
                                    evaluator,
                                )
                                .emit_overflow(
                                    transition,
                                    hit_row_range.range_to(charpos),
                                    DisplayRowPosition { x_px: x, col },
                                );
                                if row_transition.is_exhausted() {
                                    break;
                                }
                                charpos = sync_charpos_from_byte_idx(byte_idx);
                                hit_row_range.advance_to(charpos);
                                let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                                    &mut prefix_request,
                                    has_prefix,
                                    &mut line_numbers,
                                    &mut hscroll_skip,
                                    &mut word_wrap,
                                    &mut trailing_whitespace,
                                );
                                transition.apply_row_start_prefix_action(
                                    &mut col,
                                    &mut transition_prefix,
                                );
                                continue;
                            }
                            BufferTextSpecialSourceCharOverflowAction::Wrap { transition } => {
                                x = content_x;
                                row_extend.clear();
                                let boundary_request = DisplayRowTextWindowTransitionContext::new(
                                    row_geometry_defaults,
                                    text_matrix_row_base,
                                    &mut row_y_positions,
                                    max_rows,
                                )
                                .overflow(
                                    transition,
                                    hit_row_range.range_to(charpos),
                                    DisplayRowPosition { x_px: x, col },
                                );
                                hit_row_range.advance_to(charpos);
                                let row_transition = boundary_request.emit(
                                    &mut row_geometry,
                                    &mut row_flags,
                                    row_limit,
                                    &mut hit_rows,
                                    &mut self.matrix_builder,
                                    &mut output_emitter,
                                    evaluator,
                                );
                                if row_transition.is_exhausted() {
                                    break;
                                }
                                let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                                    &mut prefix_request,
                                    has_prefix,
                                    &mut line_numbers,
                                    &mut hscroll_skip,
                                    &mut word_wrap,
                                    &mut trailing_whitespace,
                                );
                                transition.apply_row_start_prefix_action(
                                    &mut col,
                                    &mut transition_prefix,
                                );
                                if !row_geometry.current_row_is_visible(row_visibility_limit) {
                                    break;
                                }
                            }
                        }
                    }

                    if let Some(append_outcome) = special_prepared_append.append_to_text_row(
                        &buffer_row_append_context,
                        &row_geometry,
                        params,
                        &mut face_ids,
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                        &mut self.font_metrics,
                        face_resolver,
                    ) {
                        append_outcome.apply_rendered_special_char_to_walk_state(
                            &mut face_scan,
                            &mut word_wrap,
                            &mut x,
                            &mut col,
                            &mut charpos,
                        );
                    }
                    continue;
                }
                BufferTextPreparedSourceCharAppend::Text(prepared_append) => prepared_append,
            };

            // Check for line wrap / truncation. Use the same append renderer
            // that materializes buffer text where builder semantics differ
            // from a simple per-face ASCII advance.
            prepared_append.update_cursor_info_for_main_char(&mut cursor_info, ch_start_byte_idx);
            match prepared_append.overflow_action(
                ch,
                text_append_surface.right_edge(),
                params.truncate_lines,
                word_wrap,
            ) {
                BufferTextSourceCharOverflowAction::Fits => {}
                BufferTextSourceCharOverflowAction::Truncate { transition } => {
                    let truncation_skip =
                        BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
                            text,
                            &mut byte_idx,
                            &mut charpos,
                        );
                    if truncation_skip.reached_line_break() {
                        line_numbers.advance_line();
                    }
                    x = content_x;
                    row_extend.clear();
                    // Record hit-test row (wrap/truncation break)
                    let row_transition = DisplayRowTextWindowEmitContext::new(
                        row_geometry_defaults,
                        text_matrix_row_base,
                        &mut row_y_positions,
                        max_rows,
                        &mut row_geometry,
                        &mut row_flags,
                        row_limit,
                        &mut hit_rows,
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_overflow(
                        transition,
                        hit_row_range.range_to(charpos),
                        DisplayRowPosition { x_px: x, col },
                    );
                    if row_transition.is_exhausted() {
                        break;
                    }
                    let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                        &mut prefix_request,
                        has_prefix,
                        &mut line_numbers,
                        &mut hscroll_skip,
                        &mut word_wrap,
                        &mut trailing_whitespace,
                    );
                    transition.apply_row_start_prefix_action(&mut col, &mut transition_prefix);
                    continue;
                }
                BufferTextSourceCharOverflowAction::WordWrap {
                    break_candidate: wrap_break,
                    transition,
                } => {
                    let word_wrap_action = BufferTextWordWrapSourceAction::new(wrap_break);
                    word_wrap_action.restore_row_output_progress(&mut output_emitter);
                    word_wrap_action.rewind_source_state(&mut byte_idx, &mut charpos, &mut col);

                    x = content_x;
                    row_extend.clear();
                    // Record hit-test row (wrap/truncation break)
                    let row_transition = DisplayRowTextWindowEmitContext::new(
                        row_geometry_defaults,
                        text_matrix_row_base,
                        &mut row_y_positions,
                        max_rows,
                        &mut row_geometry,
                        &mut row_flags,
                        row_limit,
                        &mut hit_rows,
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_overflow(
                        transition,
                        hit_row_range.range_to(charpos),
                        DisplayRowPosition { x_px: x, col },
                    );
                    if row_transition.is_exhausted() {
                        break;
                    }
                    charpos = word_wrap_action.charpos();
                    hit_row_range.advance_to(charpos);
                    let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                        &mut prefix_request,
                        has_prefix,
                        &mut line_numbers,
                        &mut hscroll_skip,
                        &mut word_wrap,
                        &mut trailing_whitespace,
                    );
                    transition.apply_prefix_action(&mut transition_prefix);

                    // Force face re-check since we rewound
                    face_scan.invalidate();

                    if !row_geometry.current_row_is_visible(row_visibility_limit) {
                        break;
                    }
                    continue;
                }
                BufferTextSourceCharOverflowAction::CharacterWrap { transition } => {
                    let character_wrap_action = BufferTextCharacterWrapSourceAction::new(
                        ch_start_byte_idx,
                        ch_start_charpos,
                    );
                    // Character wrap (no break point available)
                    x = content_x;
                    row_extend.clear();
                    // Record hit-test row (wrap/truncation break)
                    let row_transition = DisplayRowTextWindowEmitContext::new(
                        row_geometry_defaults,
                        text_matrix_row_base,
                        &mut row_y_positions,
                        max_rows,
                        &mut row_geometry,
                        &mut row_flags,
                        row_limit,
                        &mut hit_rows,
                        &mut self.matrix_builder,
                        &mut output_emitter,
                        evaluator,
                    )
                    .emit_overflow(
                        transition,
                        hit_row_range.range_to(charpos),
                        DisplayRowPosition { x_px: x, col },
                    );
                    if row_transition.is_exhausted() {
                        break;
                    }
                    let mut transition_prefix = DisplayRowTransitionPrefixContext::new(
                        &mut prefix_request,
                        has_prefix,
                        &mut line_numbers,
                        &mut hscroll_skip,
                        &mut word_wrap,
                        &mut trailing_whitespace,
                    );
                    transition.apply_row_start_prefix_action(&mut col, &mut transition_prefix);
                    character_wrap_action.rewind_source_state(&mut byte_idx, &mut charpos);
                    hit_row_range.advance_to(charpos);
                    face_scan.invalidate();
                    if !row_geometry.current_row_is_visible(row_visibility_limit) {
                        break;
                    }
                    continue;
                }
            }

            // Reset raise offset when past the raise region
            raise_span.clear_if_expired(charpos, window_start);

            prepared_append.capture_cursor_info_for_main_char_if_point(
                &mut cursor_info,
                &active_face_state,
                &row_geometry,
                x,
                ch_start_byte_idx,
                col,
                ch == '\t',
                charpos,
                point_charpos,
            );

            // --- Overlay before-strings ---
            overlay_string_context!().render_before_at(
                evaluator,
                &mut output_emitter,
                buffer,
                charpos,
                &mut self.font_metrics,
                face_resolver,
                &mut x,
                &mut col,
                &mut row_geometry,
                &mut cursor_info,
                &mut hit_rows,
                &mut hit_row_range,
                &mut row_y_positions,
                &mut face_ids,
                &mut self.matrix_builder,
            );

            let appended = prepared_append.append_to_text_row(
                &buffer_row_append_context,
                &append_geometry,
                &mut self.matrix_builder,
                &mut output_emitter,
                evaluator,
                &mut self.font_metrics,
                face_resolver,
            );
            let Some(append_outcome) = appended else {
                break;
            };
            append_outcome.apply_rendered_char_to_walk_state(
                &mut trailing_whitespace,
                &mut word_wrap,
                ch,
                &row_geometry,
                &mut x,
                &mut col,
                &mut charpos,
            );

            // --- Overlay after-strings ---
            overlay_string_context!().render_after_at(
                evaluator,
                &mut output_emitter,
                buffer,
                charpos,
                &mut self.font_metrics,
                face_resolver,
                &mut x,
                &mut col,
                &mut row_geometry,
                &mut cursor_info,
                &mut hit_rows,
                &mut hit_row_range,
                &mut row_y_positions,
                &mut face_ids,
                &mut self.matrix_builder,
            );
        }

        let point_is_visible_eob = point_charpos == accessible_end && charpos == accessible_end;

        // Capture cursor at end-of-buffer position.
        // GNU Emacs shows point at point-max+1 as a real cursor location.
        // In the layout engine's internal 0-based space, that is `accessible_end`.
        if cursor_info.is_missing() && (charpos == point_charpos || point_is_visible_eob) {
            if point_is_visible_eob {
                tracing::debug!(
                    "layout_window_rust: capturing EOB cursor at x={:.1} y={:.1} point={} point-max={}",
                    x,
                    row_geometry.glyph_y(0.0),
                    point_charpos,
                    accessible_end
                );
            }
            capture_cursor_info(
                &mut cursor_info,
                CapturedCursorInfo::from_active_face_state(
                    &active_face_state,
                    CapturedCursorPlacement::from_row_text_position(
                        row_geometry.text_position(x, byte_idx, col),
                        CapturedCursorSlotWidth::FaceChar,
                        false,
                    ),
                ),
            );
        }

        // Close any remaining box face region at end of text
        if box_face.is_active() {
            let _ = (box_face.start_x(), box_face.row()); // suppress unused warnings
        }

        // EOB overlay strings: check for overlay strings at the end-of-buffer position
        if has_overlays && row_geometry.is_within_row_limit(row_limit) {
            overlay_string_context!().render_both_at(
                evaluator,
                &mut output_emitter,
                buffer,
                charpos,
                &mut self.font_metrics,
                face_resolver,
                &mut x,
                &mut col,
                &mut row_geometry,
                &mut cursor_info,
                &mut hit_rows,
                &mut hit_row_range,
                &mut row_y_positions,
                &mut face_ids,
                &mut self.matrix_builder,
            );
        }

        // Face :extend at end-of-buffer: fill remaining empty rows
        // with the last :extend face's background color
        if let Some((_ext_bg, _ext_face_id)) = row_extend.value() {
            let right_edge = text_append_surface.right_edge();
            // First, extend the current (partially filled) row if text didn't fill it
            if x < right_edge && row_geometry.is_within_row_limit(row_limit) {
                let _ry = row_geometry.current_row_y(&row_y_positions, text_y, char_h);
            }
            // Then fill completely empty rows below
            let start_row = row_geometry.first_row_below_current(row_limit);
            for r in start_row..max_rows {
                let ry = row_geometry.row_y(r, &row_y_positions, text_y, char_h);
                if ry + char_h > text_y + text_height {
                    break;
                } // Don't extend past text area
            }
        }

        // Render fringe indicators
        if params.left_fringe_width > 0.0 || params.right_fringe_width > 0.0 {
            let _fringe_char_w = params.left_fringe_width.min(char_w).max(char_w * 0.5);

            for r in 0..row_geometry.rendered_row_count(row_limit) {
                let _gy = row_y_positions.y_for_row(r, row_geometry_defaults.row_y_fallback(0.0));

                // Right fringe: continuation arrow for wrapped lines
                if params.right_fringe_width > 0.0
                    && row_flags.is_set(r, DisplayRowFlagKind::Continued)
                {}

                // Right fringe: truncation indicator
                if params.right_fringe_width > 0.0
                    && row_flags.is_set(r, DisplayRowFlagKind::Truncated)
                {}

                // Left fringe: continuation from previous line
                if params.left_fringe_width > 0.0
                    && row_flags.is_set(r, DisplayRowFlagKind::Continuation)
                {}
            }

            // Empty line indicators (after buffer text ends)
            if params.indicate_empty_lines > 0 {
                let eob_start = row_geometry.rendered_row_count(row_limit);
                for r in eob_start..max_rows {
                    let _gy = row_geometry.row_y(r, &row_y_positions, text_y, char_h);
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
                let total_rows = row_geometry.rendered_row_count(row_limit);
                for r in 0..total_rows {
                    let _gy =
                        row_y_positions.y_for_row(r, row_geometry_defaults.row_y_fallback(0.0));
                    if indicator_x < text_append_surface.right_edge() {}
                }
            }
        }

        if point_charpos >= window_start && (point_charpos <= charpos || point_is_visible_eob) {
            if let Some(cursor) = cursor_info.captured() {
                let row_metric = row_metrics_for_cursor(
                    output_emitter.row_metrics(),
                    text_matrix_row_base + cursor.matrix_row,
                    row_geometry.row_metrics_snapshot(text_matrix_row_base),
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
                        publish_text_window_cursor(
                            &mut self.matrix_builder,
                            &mut output_emitter,
                            TextWindowCursor {
                                selected: params.selected,
                                window_id: resolved_cursor.window_id(),
                                charpos: point_charpos.max(0) as usize,
                                slot_id: resolved_cursor.slot_id,
                                x: resolved_cursor.x,
                                y: resolved_cursor.y,
                                width: resolved_cursor.width,
                                height: resolved_cursor.height,
                                ascent: resolved_cursor.ascent,
                                style: resolved_cursor.style,
                                color: resolved_cursor.color,
                                cursor_fg: resolved_cursor.cursor_fg,
                                text_area_left,
                                window_top,
                            },
                        );

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

        finish_pending_text_window_row(
            &mut self.matrix_builder,
            &mut output_emitter,
            evaluator,
            TextWindowPendingRowFinish {
                row_geometry: &row_geometry,
                row_limit,
                row_y_positions: &row_y_positions,
                text_y,
                char_height: char_h,
                charpos,
                hit_row_range: &mut hit_row_range,
                hit_rows: &mut hit_rows,
            },
        );

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
            publish_text_window_decorative_cursor(
                &mut self.matrix_builder,
                TextWindowDecorativeCursor {
                    window_id: resolved_cursor.window_id(),
                    slot_id: resolved_cursor.slot_id,
                    x: resolved_cursor.x,
                    y: resolved_cursor.y,
                    width: resolved_cursor.width,
                    height: resolved_cursor.height,
                    style: resolved_cursor.style,
                    color: resolved_cursor.color,
                    effects: spec.effects.clone(),
                },
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
            self.frame_face_id_counter = face_ids.finish();
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

        let redisplay_positions = TextWindowRedisplayPositions::from_output_rows(
            &output_emitter,
            window_start,
            text_start_byte,
            byte_idx,
        );
        record_text_window_redisplay_positions(
            &mut self.matrix_builder,
            params.window_id as u64,
            redisplay_positions,
        );

        tracing::debug!(
            "  layout_window_rust: window_start={} window_end={}",
            redisplay_positions.window_start.as_i64(),
            redisplay_positions.window_end.as_i64()
        );

        // GNU status-line percent specs read the live window state from the
        // just-produced redisplay. Publish the authoritative window geometry
        // before evaluating mode-line/header-line/tab-line forms so `%p/%P/%o`
        // reflect the frame we are about to render, not stale state from the
        // previous redisplay.
        evaluator.publish_redisplay_window_positions(
            frame_id,
            neovm_core::window::WindowId(params.window_id as u64),
            redisplay_positions.window_start,
            LispCharPos1::from_one_based_usize(accessible_end_lisp_char),
            EmacsBytePos::new(accessible_end_emacs_byte),
            redisplay_positions.window_end,
            redisplay_positions.window_end_byte,
            redisplay_positions.window_end_vpos,
        );

        // --- GlyphMatrix builder: finalize text rows, then emit chrome rows
        // into their real glyph-matrix slots before closing the window. ---
        let right_edge_markers = reserve_right_special_col.then(|| TextWindowRightEdgeMarkers {
            text_matrix_row_base,
            matrix_cols,
            column: if reserve_right_border_col {
                TextWindowRightEdgeMarkerColumn::BeforeRightBorder
            } else {
                TextWindowRightEdgeMarkerColumn::LastColumn
            },
            row_flags: &row_flags,
            face_id: 0,
            char_width: char_w,
        });
        install_text_window_output(
            &mut self.matrix_builder,
            &output_emitter,
            TextWindowOutputInstall { right_edge_markers },
        );

        let mut status_line_symbol_values = std::collections::HashMap::new();
        if let Some(buffer) = evaluator
            .buffer_manager()
            .get(neovm_core::buffer::BufferId(params.buffer_id))
        {
            if let Some(value) = buffer.buffer_local_value("header-line-indent-width") {
                status_line_symbol_values.insert("header-line-indent-width".to_string(), value);
            }
        }
        let chrome_tab_policy = DisplayTabPolicy::from_tab_width_and_stops(
            0.0,
            params.tab_width,
            &params.tab_stop_list,
        );

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
            self.render_window_chrome_display_row(
                evaluator,
                &mut output_emitter,
                face_resolver,
                &mut face_ids,
                WindowChromeDisplayRowRequest {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::TabLine,
                    matrix_row: 0,
                    output: tab_row_output,
                    bounds: Rect::new(params.bounds.x, tl_y, params.bounds.width, tab_line_height),
                    char_width: char_w,
                    ascent: font_ascent,
                    tab_policy: chrome_tab_policy.clone(),
                    base_face: tl_face,
                    symbol_values: status_line_symbol_values.clone(),
                    text: WindowChromeDisplayText::new(tab_text, params.selected),
                },
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
            self.render_window_chrome_display_row(
                evaluator,
                &mut output_emitter,
                face_resolver,
                &mut face_ids,
                WindowChromeDisplayRowRequest {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::HeaderLine,
                    matrix_row: usize::from(tab_line_height > 0.0),
                    output: header_row_output,
                    bounds: Rect::new(
                        params.bounds.x,
                        hl_y,
                        params.bounds.width,
                        header_line_height,
                    ),
                    char_width: char_w,
                    ascent: font_ascent,
                    tab_policy: chrome_tab_policy.clone(),
                    base_face: hl_face,
                    symbol_values: status_line_symbol_values.clone(),
                    text: WindowChromeDisplayText::new(header_text, params.selected),
                },
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
            self.render_window_chrome_display_row(
                evaluator,
                &mut output_emitter,
                face_resolver,
                &mut face_ids,
                WindowChromeDisplayRowRequest {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::ModeLine,
                    matrix_row: mode_line_matrix_row,
                    output: mode_row_output,
                    bounds: Rect::new(params.bounds.x, ml_y, params.bounds.width, mode_line_height),
                    char_width: char_w,
                    ascent: font_ascent,
                    tab_policy: chrome_tab_policy,
                    base_face: ml_face,
                    symbol_values: status_line_symbol_values.clone(),
                    text: WindowChromeDisplayText::new(mode_text, params.selected),
                },
            );
        }

        close_text_window_output(&mut self.matrix_builder);

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
        self.frame_face_id_counter = face_ids.finish();
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

const MOCK_DISPLAY_SOURCE_ID: u64 = 0x6d6f_636b;

fn mock_display_row_layout(
    role: GlyphRowRole,
    pixel_y: f32,
    width_px: f32,
    char_w: f32,
    char_h: f32,
    ascent: f32,
) -> DisplayRowLayout {
    DisplayRowLayout {
        role,
        y_px: pixel_y,
        width_px: width_px.max(1.0),
        height_px: char_h.max(1.0),
        ascent_px: ascent.max(0.0).min(char_h.max(1.0)),
        char_width_px: char_w.max(1.0),
        tab_policy: DisplayTabPolicy::every(8),
        base_face: RenderFaceRef::FaceId(0),
        symbol_values: std::collections::HashMap::new(),
    }
}

fn mock_display_text_item(text: String, face_id: u32, source_offset: usize) -> DisplayItem {
    let char_len = text.chars().count();
    DisplayItem::new(
        SourceSpan::synthetic(
            MOCK_DISPLAY_SOURCE_ID,
            source_offset,
            source_offset.saturating_add(char_len),
        ),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    )
}

fn push_mock_display_text(
    writer: &mut DisplayRowWriter<'_, '_, '_>,
    text: String,
    face_id: u32,
    source_offset: &mut usize,
) {
    let char_len = text.chars().count();
    if char_len == 0 {
        return;
    }
    writer.push_item(mock_display_text_item(text, face_id, *source_offset));
    *source_offset = source_offset.saturating_add(char_len);
}

fn mock_display_row_from_line(
    role: GlyphRowRole,
    line: &super::mock_frame::MockStyledLine,
    pixel_y: f32,
    width_px: f32,
    char_w: f32,
    char_h: f32,
    ascent: f32,
    left_margin: Option<&str>,
    fill_to_cols: Option<(usize, u32)>,
) -> GlyphRow {
    use super::mock_frame::MockDisplayProperty;

    let layout = mock_display_row_layout(role, pixel_y, width_px, char_w, char_h, ascent);
    let mut row = new_display_row(&layout);
    if let Some(left_margin) = left_margin {
        let mut writer = DisplayRowWriter::for_area(&layout, &mut row, GlyphArea::LeftMargin);
        let mut margin_source_offset = 0usize;
        push_mock_display_text(
            &mut writer,
            left_margin.to_owned(),
            2,
            &mut margin_source_offset,
        );
    }
    let mut writer = DisplayRowWriter::new(&layout, &mut row);
    let mut source_offset = 0usize;
    for glyph in &line.glyphs {
        match &glyph.display {
            Some(MockDisplayProperty::Invisible) => {
                source_offset = source_offset.saturating_add(1);
            }
            Some(MockDisplayProperty::Replace(text, face_id)) => {
                push_mock_display_text(&mut writer, text.clone(), *face_id, &mut source_offset);
            }
            Some(MockDisplayProperty::Composition(composed)) => {
                for composed_glyph in composed {
                    push_mock_display_text(
                        &mut writer,
                        composed_glyph.ch.to_string(),
                        composed_glyph.face_id,
                        &mut source_offset,
                    );
                }
            }
            None => {
                push_mock_display_text(
                    &mut writer,
                    glyph.ch.to_string(),
                    glyph.face_id,
                    &mut source_offset,
                );
            }
        }
    }
    drop(writer);
    if let Some((target_cols, face_id)) = fill_to_cols {
        let current_cols = display_row_text_glyph_count(&row);
        if current_cols < target_cols {
            let mut writer = DisplayRowWriter::new(&layout, &mut row);
            push_mock_display_text(
                &mut writer,
                " ".repeat(target_cols - current_cols),
                face_id,
                &mut source_offset,
            );
        }
    }
    crate::glyph_row_writer::normalize_external_row(&mut row);
    row
}

impl LayoutEngine {
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
        let tab_bar_face = face_resolver.resolve_named_face("tab-bar");
        let tab_bar_ascent = frame_params.char_height * 0.8;
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
        let mut face_ids = FrameFaceIdAllocator::new(self.frame_face_id_counter);
        let Some(rendered_tab_bar) = self.render_frame_tab_bar_display_row(
            face_resolver,
            evaluator.display_host.as_deref(),
            &mut face_ids,
            FrameTabBarDisplayRowRequest {
                row_index,
                y: tab_bar_y,
                width,
                height: tab_bar_height,
                char_width: frame_params.char_width,
                ascent: tab_bar_ascent,
                row_height: frame_params.char_height,
                base_face: &tab_bar_face,
                text: tab_bar.text,
            },
        ) else {
            return None;
        };
        self.frame_face_id_counter = face_ids.finish();
        let FrameTabBarDisplayRowRender::Measured(measured) = rendered_tab_bar else {
            return None;
        };
        let actual_tab_bar_height = measured.bounds.height;
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
        use neomacs_display_protocol::face::FaceAttributes;
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
            let nrows = window.lines.len() + 1;
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
                let row_y = window.pixel_bounds.y + row_idx as f32 * char_h;
                let lnum = format!("{:>3} ", row_idx + 1);
                let row = mock_display_row_from_line(
                    GlyphRowRole::Text,
                    line,
                    row_y,
                    window.pixel_bounds.width,
                    char_w,
                    char_h,
                    ascent,
                    Some(&lnum),
                    None,
                );
                builder.install_prebuilt_current_row(&row);
                builder.end_prebuilt_row();
            }

            // Mode-line pinned to window bottom.
            let mode_line_row = window.lines.len();
            builder.begin_row(mode_line_row, GlyphRowRole::ModeLine);
            let ml_ncols = (window.pixel_bounds.width / char_w.max(1.0)) as usize;
            let row = mock_display_row_from_line(
                GlyphRowRole::ModeLine,
                &window.mode_line,
                window.pixel_bounds.y + window.pixel_bounds.height - char_h,
                window.pixel_bounds.width,
                char_w,
                char_h,
                ascent,
                None,
                Some((ml_ncols, 1)),
            );
            builder.install_prebuilt_current_row(&row);
            builder.end_prebuilt_row();

            builder.end_window();
        }

        // Minibuffer at frame bottom — a real window with text rows
        // and optionally a thin mode-line, matching GNU's design where
        // the echo-area text is buffer content, not a mode-line.
        if let Some(ref mini) = content.minibuffer {
            let has_mode_line = !mini.mode_line.glyphs.is_empty();
            let nrows = mini.lines.len() + usize::from(has_mode_line);
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
                let row_y = mini.pixel_bounds.y + row_idx as f32 * char_h;
                let row = mock_display_row_from_line(
                    GlyphRowRole::Minibuffer,
                    line,
                    row_y,
                    mini.pixel_bounds.width,
                    char_w,
                    char_h,
                    ascent,
                    None,
                    None,
                );
                builder.install_prebuilt_current_row(&row);
                builder.end_prebuilt_row();
            }

            if has_mode_line {
                let mode_line_row = mini.lines.len();
                builder.begin_row(mode_line_row, GlyphRowRole::ModeLine);
                let mini_ncols = (mini.pixel_bounds.width / char_w.max(1.0)) as usize;
                let row = mock_display_row_from_line(
                    GlyphRowRole::ModeLine,
                    &mini.mode_line,
                    mini.pixel_bounds.y + mini.pixel_bounds.height - char_h,
                    mini.pixel_bounds.width,
                    char_w,
                    char_h,
                    ascent,
                    None,
                    Some((mini_ncols, 1)),
                );
                builder.install_prebuilt_current_row(&row);
                builder.end_prebuilt_row();
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
                let row = mock_display_row_from_line(
                    GlyphRowRole::Text,
                    line,
                    cf.window.pixel_bounds.y + ri as f32 * char_h,
                    cf.window.pixel_bounds.width,
                    char_w,
                    char_h,
                    ascent,
                    None,
                    None,
                );
                cb.install_prebuilt_current_row(&row);
                cb.end_prebuilt_row();
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
