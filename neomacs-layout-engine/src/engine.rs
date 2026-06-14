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
    WindowChromeRowsRenderRequest, build_tab_bar_display, max_mini_window_lines,
    message_truncate_lines, minibuffer_echo_message_for_window, minibuffer_resize_line_count,
};
use super::font_metrics::FontMetricsService;
use super::gui_chrome::{collect_gui_menu_bar_items_for_frame, collect_gui_tool_bar_items};
use super::hit_test::*;
use super::types::*;
#[cfg(test)]
use super::window_output::RowMetricsSnapshot;
use crate::coords::layout_i64_char_pos_to_lisp_char_pos;
use crate::display_buffer_text_source::BufferTextWindowSourceReadRequest;
use crate::display_buffer_text_walk::{
    BufferTextWindowGeometry, BufferTextWindowGeometryRequest, BufferTextWindowLocalDisplayPolicy,
    BufferTextWindowLoopRenderState, BufferTextWindowLoopRequestContext,
    BufferTextWindowOutputSetup, BufferTextWindowOutputSetupRequest, BufferTextWindowPostLoopState,
    BufferTextWindowRenderContexts, BufferTextWindowRenderContextsRequest,
    BufferTextWindowTailRequestContext, BufferTextWindowWalkSetup,
    BufferTextWindowWalkSetupRequest,
};
#[cfg(test)]
use crate::display_cursor::CapturedCursorVisualState;
#[cfg(test)]
use crate::display_cursor::CursorCaptureState;
#[cfg(test)]
use crate::display_cursor::CursorSlotWidthPolicy;
#[cfg(test)]
use crate::display_cursor::resolve_cursor_vertical_metrics;
#[cfg(test)]
use crate::display_cursor::{CapturedCursorInfo, CapturedCursorPlacement, CapturedCursorSlotWidth};
#[cfg(test)]
use crate::display_cursor::{CursorSlotWidthRequest, VisualCursorGeometryContext};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_frame_output::{
    FrameLineAnimationHintsRenderRequest, FrameThemeTransitionHintRenderRequest,
    FrameTopologyTransitionHintRenderRequest, FrameWindowSwitchHintRenderRequest,
    WindowFrameDecorationsRenderRequest, WindowFrameGeometryRequest,
    WindowFrameInfoEffectsRenderRequest, WindowFrameInfoRenderRequest, WindowFrameMetadata,
};
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFace, DisplayRowFallbackMetrics,
    DisplayRowMeasurementPolicy, insert_resolved_display_row_face,
};
#[cfg(test)]
use crate::display_row_append::DisplayRowPrefixRequest;
#[cfg(test)]
use crate::display_row_append::DisplayRowPrefixValues;
#[cfg(test)]
use crate::display_row_append::OverlayStringRenderSource;
use crate::display_row_append::{
    BufferTextWindowBeginState, BufferTextWindowCursorEffectsRequest, BufferTextWindowFinishState,
};
use crate::display_row_builder::{
    DisplayRowLayout, DisplayRowPosition, DisplayRowWriter, DisplayTabPolicy,
    display_row_text_glyph_count, new_display_row,
};
#[cfg(test)]
use crate::display_row_geometry::{DisplayRowHitRange, DisplayRowMarker, DisplayRowStartMarker};
use crate::display_row_walk_state::FaceScanCheckpoint;
#[cfg(test)]
use crate::display_row_walk_state::WordWrapBreakCandidate;
#[cfg(test)]
use crate::display_row_walk_state::{
    ActiveDisplayPropertySpan, BoxFaceRowState, HitRowRangeTracker, HorizontalScrollSkipState,
    LineNumberRenderState, TextPropertyScanCheckpoints, TrailingWhitespaceRenderState,
    WordWrapRenderState,
};
use crate::fontconfig::FontSizing;
use neomacs_display_protocol::face::BasicFaceId;
#[cfg(test)]
use neomacs_display_protocol::frame_glyphs::CursorStyle;
use neomacs_display_protocol::frame_glyphs::{GlyphRowRole, WindowInfo};
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphRow};
use neomacs_display_protocol::types::Color;
#[cfg(test)]
use neomacs_display_protocol::types::Rect;
use neovm_core::buffer::{EmacsBytePos, LispCharPos1};
use neovm_core::window::{WindowDisplaySnapshot, WindowId};

/// Bound redisplay convergence work when point begins outside the visible span.
const MAX_WINDOW_VISIBILITY_RETRIES: usize = 128;

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
                let window_geometry =
                    WindowFrameGeometryRequest::new(params, &frame_params, main_area_bottom)
                        .resolve();
                let metadata = {
                    let buf_id = neovm_core::buffer::BufferId(params.buffer_id);
                    let buffer = evaluator.buffer_manager().get(buf_id);
                    WindowFrameMetadata {
                        buffer_file_name: buffer
                            .and_then(|b| b.file_name_runtime_string_owned())
                            .unwrap_or_default(),
                        modified: buffer.map(|b| b.is_modified()).unwrap_or(false),
                    }
                };
                WindowFrameInfoRenderRequest::new(params, metadata)
                    .render_and_apply(&mut self.matrix_builder);
                WindowFrameInfoEffectsRenderRequest::new(&self.prev_window_infos)
                    .render_latest_and_apply(&mut self.matrix_builder, &mut curr_window_infos);

                // Simplified layout for this window (no face resolution, no overlays)
                self.layout_window_rust(
                    evaluator,
                    frame_id,
                    params,
                    &frame_params,
                    &face_resolver,
                    window_geometry.reserve_terminal_right_border_col,
                    MAX_WINDOW_VISIBILITY_RETRIES,
                );

                if let Some(info) = self
                    .matrix_builder
                    .window_infos()
                    .iter()
                    .rev()
                    .find(|info| info.window_id == params.window_id)
                    .cloned()
                {
                    WindowFrameDecorationsRenderRequest::new(
                        params,
                        &frame_params,
                        window_geometry,
                        &info,
                        &face_resolver,
                    )
                    .render_and_apply(&mut self.matrix_builder);
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

            FrameLineAnimationHintsRenderRequest::new(&self.prev_window_infos, &curr_window_infos)
                .render_and_apply(&mut self.matrix_builder);
            FrameWindowSwitchHintRenderRequest::new(&mut self.prev_selected_window_id)
                .render_and_apply(&mut self.matrix_builder);
            FrameThemeTransitionHintRenderRequest::new(
                &mut self.prev_background,
                frame_params.width,
                frame_params.height,
            )
            .render_and_apply(&mut self.matrix_builder);
            FrameTopologyTransitionHintRenderRequest::new(
                &self.prev_window_infos,
                &curr_window_infos,
                frame_params.width,
                frame_params.height,
            )
            .render_and_apply(&mut self.matrix_builder);

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
        BufferTextWindowCursorEffectsRequest::new(params.window_id, params.cursor_effects.clone())
            .install_and_apply(&mut self.matrix_builder);

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

        let local_display_policy = BufferTextWindowLocalDisplayPolicy::from_buffer(buffer);
        let has_prefix = local_display_policy.has_prefix();

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
        let geometry_request = BufferTextWindowGeometryRequest::new(
            params,
            char_w,
            char_h,
            mode_line_height,
            header_line_height,
            tab_line_height,
        );
        let lnum_cols = local_display_policy
            .line_number_columns(&buf_access, geometry_request.line_number_row_capacity());

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
        let minibuffer_content_rows = if params.is_minibuffer {
            let buf_id = neovm_core::buffer::BufferId(params.buffer_id);
            let content_lines = evaluator
                .buffer_manager()
                .get(buf_id)
                .map(|buffer| minibuffer_resize_line_count(buffer, params.window_id as u64))
                .unwrap_or(1);
            let frame_rows = frame_params.height / char_h;
            let max_mini = max_mini_window_lines(evaluator, frame_rows).ceil() as usize;
            Some(content_lines.clamp(1, max_mini))
        } else {
            None
        };
        let BufferTextWindowGeometry {
            text_x,
            text_y,
            text_width,
            text_height,
            max_rows,
            text_matrix_row_base,
            text_matrix_rows,
            bottom_chrome_rows,
            mode_line_matrix_row,
            cols,
            line_number_pixel_width: lnum_pixel_width,
            content_x,
        } = geometry_request.into_geometry(lnum_cols, minibuffer_content_rows);

        // GNU Emacs redisplay advances iterators until the visible window is
        // fully resolved; it does not stop at an arbitrary "rows * cols"
        // character budget.  Capping the text slice here truncates long
        // wrapped or truncated lines before they are actually offscreen, which
        // breaks both redisplay and geometry queries.
        let text_source = BufferTextWindowSourceReadRequest::new(params, max_rows)
            .read_into(&buf_access, &mut self.text_buf);
        let window_start = text_source.window_start();
        let text_start_byte = text_source.text_start_byte();
        let bytes_read = text_source.bytes_read();
        let point_charpos = text_source.point_charpos();
        let accessible_start = text_source.accessible_start();
        let accessible_end = text_source.accessible_end();

        let text = if bytes_read > 0 {
            &self.text_buf[..bytes_read]
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

        let mut line_numbers =
            local_display_policy.initial_line_numbers(&buf_access, window_start, point_charpos);

        let reserve_right_special_col =
            !frame_params.window_system && params.right_fringe_width == 0.0;
        let walk_setup = BufferTextWindowWalkSetupRequest::new(
            window_start,
            content_x,
            text_x,
            text_width,
            text_y,
            params.bounds.y,
            lnum_pixel_width,
            max_rows,
            char_w,
            char_h,
            default_face_ascent,
            params.truncate_lines,
            params.hscroll,
            params.word_wrap,
            has_prefix,
            local_display_policy.has_line_default_prefix(),
            reserve_right_border_col,
            reserve_right_special_col,
            params.tab_width,
            &params.tab_stop_list,
            params.show_trailing_whitespace,
            params.trailing_ws_bg,
        )
        .into_setup();
        let BufferTextWindowOutputSetup {
            begin_request,
            row_visibility_limit,
            row_limit,
            body_install_context,
            retry_bounds,
        } = BufferTextWindowOutputSetupRequest::new(
            frame_id,
            window_id,
            params.window_id as u64,
            text_matrix_row_base,
            text_matrix_rows,
            bottom_chrome_rows,
            cols,
            params.bounds,
            params.text_bounds,
            params.selected,
            text_y,
            text_height,
        )
        .into_setup(max_rows, &walk_setup);
        let BufferTextWindowWalkSetup {
            mut x,
            mut col,
            mut byte_idx,
            mut charpos,
            text_area_left,
            window_top,
            mut text_property_checkpoints,
            mut raise_span,
            mut height_span,
            mut row_flags,
            mut hscroll_skip,
            mut word_wrap,
            mut prefix_request,
            text_append_surface,
            row_geometry_defaults,
            mut row_geometry,
            mut row_y_positions,
            mut trailing_whitespace,
            mut buffer_text_append_state,
            mut row_extend,
            mut box_face,
            mut cursor_info,
            mut hit_rows,
            mut hit_row_range,
        } = walk_setup;

        let BufferTextWindowRenderContexts {
            has_overlays,
            face_resolution: face_resolution_context,
            overlay_text_row: overlay_text_row_context,
        } = BufferTextWindowRenderContextsRequest::new(
            buffer,
            face_resolver,
            measurement_policy,
            default_resolved,
            default_face_char_w,
            default_face_ascent,
            default_face_h,
            char_w,
            char_h,
            font_ascent,
            frame_params.window_system,
            params.window_id as u64,
            &text_append_surface,
            text_y,
            text_matrix_row_base,
            max_rows,
        )
        .into_contexts();
        let loop_request_context = BufferTextWindowLoopRequestContext::new(
            buf_id,
            text_start_byte,
            accessible_end,
            point_charpos,
            params,
            content_x,
            has_prefix,
            default_face_ascent,
            char_h,
            char_w,
            row_visibility_limit,
            row_geometry_defaults,
            text_matrix_row_base,
            max_rows,
            row_limit,
        );
        let row_prelude_request_context =
            local_display_policy.row_prelude_context(lnum_cols, char_w, char_h);
        let tail_request_context = BufferTextWindowTailRequestContext::new(
            params,
            window_start,
            accessible_start,
            accessible_end,
            text_start_byte,
            text_matrix_row_base,
            text_area_left,
            window_top,
            text_y,
            text_height,
            content_x,
            cols,
            char_w,
            char_h,
            default_fg,
            max_rows,
            row_limit,
            row_geometry_defaults,
            retry_bounds,
            body_install_context,
            reserve_right_special_col,
            reserve_right_border_col,
            mode_line_height,
            header_line_height,
            tab_line_height,
        );

        let mut output_emitter = begin_request.begin_and_apply(BufferTextWindowBeginState {
            builder: &mut self.matrix_builder,
            evaluator,
        });

        while byte_idx < text.len() && row_geometry.current_row_is_visible(row_visibility_limit) {
            row_prelude_request_context
                .line_number_margin_request()
                .render_pending(
                    &mut line_numbers,
                    face_resolver,
                    &mut face_ids,
                    &mut self.matrix_builder,
                    &row_geometry,
                    &mut face_scan,
                    row_prelude_request_context.char_width(),
                );

            row_prelude_request_context
                .line_prefix_request(
                    &text_append_surface,
                    &row_geometry,
                    &active_face_state,
                    raise_span.value_or(0.0),
                    DisplayRowPosition { x_px: x, col },
                )
                .render_requested_to_text_row_and_apply(
                    &mut prefix_request,
                    evaluator,
                    &mut output_emitter,
                    buffer,
                    charpos,
                    &mut self.font_metrics,
                    face_resolver,
                    &mut face_ids,
                    &mut self.matrix_builder,
                    &mut x,
                    &mut col,
                );

            let mut loop_render_state = BufferTextWindowLoopRenderState::new(
                &mut buffer_text_append_state,
                &mut text_property_checkpoints,
                &mut byte_idx,
                &mut charpos,
                &mut col,
                &mut output_emitter,
                &mut row_extend,
                &mut box_face,
                &mut x,
                &mut line_numbers,
                &mut row_geometry,
                &mut row_flags,
                &mut hit_rows,
                &mut hit_row_range,
                &mut self.matrix_builder,
                evaluator,
                &mut prefix_request,
                &mut hscroll_skip,
                &mut word_wrap,
                &mut trailing_whitespace,
                &mut face_scan,
                &mut row_y_positions,
                &mut self.font_metrics,
                face_resolver,
                &mut cursor_info,
                &mut face_ids,
                &mut raise_span,
                &mut height_span,
            );

            // --- Invisible text check ---
            if loop_render_state
                .render_invisible_text_for_context(
                    loop_request_context,
                    text,
                    &text_append_surface,
                    overlay_text_row_context,
                    &active_face_state,
                    buffer,
                )
                .should_continue_buffer_walk()
            {
                continue;
            }

            // Handle hscroll: skip columns consumed by horizontal scroll
            if loop_render_state.hscroll_should_skip() {
                if loop_render_state
                    .render_hscroll_skip_for_context(
                        loop_request_context,
                        text,
                        &text_append_surface,
                        &active_face_state,
                    )
                    .should_break()
                {
                    break;
                }
                continue;
            }

            // --- Display property check ---
            // Only call check_display_prop at property change boundaries for efficiency
            let display_property_walk = loop_render_state
                .render_display_property_checkpoint_for_context(
                    loop_request_context,
                    face_resolution_context.clone(),
                    text,
                    params,
                    &text_append_surface,
                    &mut active_face_state,
                );
            if display_property_walk.should_continue_buffer_walk() {
                continue;
            }

            // Decode UTF-8 character. Keep the original byte/char position in
            // the source object so wrap/newline/cursor paths all use the same
            // typed buffer source coordinates.
            let Some(decoded_source_char) = loop_render_state.consume_source_char(text) else {
                break;
            };
            let ch = decoded_source_char.ch();

            let selective_display_outcome = loop_render_state
                .render_selective_display_tail_for_context(
                    loop_request_context,
                    decoded_source_char,
                    text,
                    &text_append_surface,
                    &active_face_state,
                    buffer,
                );
            if selective_display_outcome.should_break() {
                break;
            }
            if selective_display_outcome.should_continue_buffer_walk() {
                continue;
            }

            if ch == '\n' {
                if loop_render_state
                    .render_line_break_for_context(
                        loop_request_context,
                        decoded_source_char,
                        text,
                        &active_face_state,
                        buffer,
                    )
                    .should_break()
                {
                    break;
                }
                continue;
            }

            let char_render_outcome = loop_render_state.render_source_char_for_context(
                loop_request_context,
                decoded_source_char,
                text,
                &text_append_surface,
                overlay_text_row_context,
                &active_face_state,
                params,
                buffer,
            );
            if char_render_outcome.should_break() {
                break;
            }
            if char_render_outcome.should_continue_buffer_walk() {
                continue;
            }
        }

        let (retry_outcome, rendered_rows_len) = {
            let mut post_loop_state = BufferTextWindowPostLoopState::new(
                &mut output_emitter,
                &mut x,
                &mut col,
                &mut row_geometry,
                &mut cursor_info,
                &mut hit_rows,
                &mut hit_row_range,
                &mut row_y_positions,
                &mut face_ids,
                &mut self.matrix_builder,
                evaluator,
                &mut self.font_metrics,
                face_resolver,
                &row_flags,
                &row_extend,
                &box_face,
            );
            let point_is_visible_eob = post_loop_state.render_end_of_buffer_tail(
                loop_request_context,
                byte_idx,
                charpos,
                has_overlays,
                overlay_text_row_context,
                &active_face_state,
                buffer,
            );

            post_loop_state.apply_tail_decorations(&tail_request_context, &text_append_surface);

            post_loop_state.finalize_tail(
                &tail_request_context,
                text,
                charpos,
                point_is_visible_eob,
            );

            // GNU redisplay keeps iterating until point visibility converges or no
            // further progress can be made.  Advance by actual rendered row spans
            // from this pass rather than rescanning by logical newlines, since
            // wrapped and variable-height lines are exactly where newline-based
            // retry selection goes wrong.
            let retry_outcome = post_loop_state.decide_visibility_retry(
                &tail_request_context,
                charpos,
                point_is_visible_eob,
                &buf_access,
            );
            (retry_outcome, post_loop_state.rendered_rows_len())
        };
        if retry_outcome.scroll_down_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} beyond visible_end={:?} (charpos_end={}), visible_rows={}, new_window_start={:?}",
                layout_i64_char_pos_to_lisp_char_pos(point_charpos).as_i64(),
                retry_outcome.visible_end_lisp(),
                charpos,
                rendered_rows_len,
                retry_outcome.scroll_down_window_start()
            );
        }
        if retry_outcome.point_row_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} row partially visible within {}..{}, new_window_start={:?}",
                point_charpos,
                retry_bounds.text_area_top,
                retry_bounds.text_area_bottom,
                retry_outcome.point_row_window_start()
            );
        }
        if retry_outcome.point_line_window_start().is_some() {
            tracing::debug!(
                "layout_window_rust: point={} line continues below final visible row, new_window_start={:?}",
                point_charpos,
                retry_outcome.point_line_window_start()
            );
        }
        let retry_window_start = retry_outcome.retry_window_start();

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

        let redisplay_positions = {
            let mut post_loop_state = BufferTextWindowPostLoopState::new(
                &mut output_emitter,
                &mut x,
                &mut col,
                &mut row_geometry,
                &mut cursor_info,
                &mut hit_rows,
                &mut hit_row_range,
                &mut row_y_positions,
                &mut face_ids,
                &mut self.matrix_builder,
                evaluator,
                &mut self.font_metrics,
                face_resolver,
                &row_flags,
                &row_extend,
                &box_face,
            );
            post_loop_state.install_body(&tail_request_context, byte_idx)
        };

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

        self.render_window_chrome_display_rows(
            evaluator,
            &mut output_emitter,
            face_resolver,
            &mut face_ids,
            WindowChromeRowsRenderRequest {
                params,
                tab_line_face: tab_line_face.as_ref(),
                header_line_face: header_line_face.as_ref(),
                mode_line_face: mode_line_face.as_ref(),
                tab_line_height,
                header_line_height,
                mode_line_height,
                mode_line_matrix_row,
                reserve_right_border_col,
                char_width: char_w,
                font_ascent,
                buffer_name: &buffer_name,
            },
        );

        let finished_window = tail_request_context.finish_request().finish_and_snapshot(
            BufferTextWindowFinishState {
                builder: &mut self.matrix_builder,
                output_emitter,
                evaluator,
                hit_rows,
            },
        );
        self.hit_data.push(finished_window.hit_data);
        self.display_snapshots.push(finished_window.snapshot);

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
