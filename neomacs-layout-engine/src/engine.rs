//! The Rust layout engine — Phase 1+2: Monospace layout with face resolution.
//!
//! Reads buffer text and display state from neovm-core, resolves faces per
//! character position, computes line breaks, positions glyphs on a fixed-width
//! grid, and publishes `FrameDisplayState` snapshots for render backends.

#[cfg(test)]
use super::display_status_line::eval_status_line_format;
use super::display_status_line::{
    ChromeRowRenderServices, FrameTabBarDisplayRowRender, FrameTabBarDisplayRowRequest,
    ResizeMiniWindowsMode, ScratchGcRootScope, build_tab_bar_display,
    max_mini_window_lines_from_value,
};
use super::font_metrics::FontMetricsService;
use super::gui_chrome::{collect_gui_menu_bar_items_for_frame, collect_gui_tool_bar_items};
use super::hit_test::*;
use super::types::*;
#[cfg(test)]
use super::window_output::RowMetricsSnapshot;
use crate::display_buffer_window_render::{
    BufferSourceRenderAttemptContext, BufferSourceRenderAttemptOutcome, BufferWindowRenderRequest,
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
    FrameLineAnimationHintsRenderRequest, FrameOutputIdentity, FrameOutputOwner,
    FrameOutputStateRenderRequest, FrameThemeTransitionHintRenderRequest,
    FrameTopologyTransitionHintRenderRequest, FrameWindowSwitchHintRenderRequest,
    WindowFrameDecorationsRenderRequest, WindowFrameGeometryRequest,
    WindowFrameInfoEffectsRenderRequest, WindowFrameInfoRenderRequest, WindowFrameMetadata,
};
use crate::display_mock_frame::layout_mock_frame_content;
use crate::display_origin::DisplayOrigin;
#[cfg(test)]
use crate::display_row_geometry::{DisplayRowHitRange, DisplayRowMarker, DisplayRowStartMarker};
#[cfg(test)]
use crate::display_row_lisp_string::DisplayRowPrefixRequest;
#[cfg(test)]
use crate::display_row_lisp_string::DisplayRowPrefixValues;
use crate::display_row_metrics::DisplayRowFallbackMetrics;
#[cfg(test)]
use crate::display_row_overlay_string::OverlayStringRenderSource;
#[cfg(test)]
use crate::display_row_walk_state::FaceScanCheckpoint;
#[cfg(test)]
use crate::display_row_walk_state::WordWrapBreakCandidate;
#[cfg(test)]
use crate::display_row_walk_state::{
    BoxFaceRowState, HitRowRangeTracker, HorizontalScrollSkipState, InvisibleTextScanCheckpoint,
    LineNumberRenderState, TrailingWhitespaceRenderState, WordWrapRenderState,
};
use crate::fontconfig::FontSizing;
use crate::incremental_layout::{
    CursorOnlyReplay, LayoutClass, LayoutStats, MatrixValidity, RetainedWindowKey,
    RetainedWindowMatrix, RowDamage, ScrollReplay,
};
use neomacs_display_protocol::face::BasicFaceId;
#[cfg(test)]
use neomacs_display_protocol::frame_glyphs::CursorStyle;
use neomacs_display_protocol::frame_glyphs::WindowInfo;
use neomacs_display_protocol::types::Color;
#[cfg(test)]
use neomacs_display_protocol::types::Rect;
use neomacs_display_protocol::types::{DisplayFrameId, DisplayWindowId};
use neovm_core::emacs_core::Value;
use neovm_core::window::WindowDisplaySnapshot;

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

/// Wrapping-aware display-row count of echo-area `text` at `cols` columns,
/// approximating GNU `resize_mini_window`'s `move_it_to (ZV)` content
/// measurement. Empty text is one line; each logical line contributes at
/// least one row plus any wrapped continuation rows.
fn echo_content_rows(text: &str, cols: usize) -> usize {
    let cols = cols.max(1);
    if text.is_empty() {
        return 1;
    }
    let mut rows = 0usize;
    for line in text.split('\n') {
        let width = line.chars().count();
        rows += width.div_ceil(cols).max(1);
    }
    rows.max(1)
}

fn max_mini_window_lines_for_window(
    evaluator: &mut neovm_core::emacs_core::Context,
    params: &WindowParams,
    frame_rows: f32,
) -> f32 {
    let window_id = neovm_core::window::WindowId(params.window_id as u64);
    let buf_id = if params.is_minibuffer() && !evaluator.minibuffer_window_is_active(window_id) {
        evaluator.ensure_echo_area_buffers();
        evaluator
            .buffer_manager()
            .find_buffer_by_name(" *Echo Area 0*")
            .unwrap_or_else(|| neovm_core::buffer::BufferId(params.buffer_id))
    } else {
        neovm_core::buffer::BufferId(params.buffer_id)
    };
    let raw = evaluator
        .buffer_manager()
        .get(buf_id)
        .and_then(|buffer| buffer.buffer_local_value("max-mini-window-height"))
        .or_else(|| {
            evaluator
                .obarray()
                .symbol_value("max-mini-window-height")
                .copied()
        })
        .unwrap_or_else(|| Value::make_float(0.25));
    max_mini_window_lines_from_value(raw, frame_rows)
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
    prev_window_infos: std::collections::HashMap<DisplayWindowId, WindowInfo>,
    /// Previous selected window id for switch-fade detection.
    prev_selected_window_id: DisplayWindowId,
    /// Previous frame background for theme-transition detection.
    prev_background: Option<(f32, f32, f32, f32)>,
    /// Authoritative frame output owner for the current frame layout pass.
    frame_output: FrameOutputOwner,
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
    /// frame-wide output face map: the first window
    /// inserted `mode-line` at face_id=2, the second window then
    /// inserted `mode-line-inactive` ALSO at face_id=2 and
    /// overwrote the first entry, causing both mode lines to
    /// render with the inactive face after `C-x 2`.
    /// Frame-scoped face-ID counter.  Starts at
    /// [`BasicFaceId::SENTINEL`] so dynamic face IDs never collide
    /// with the fixed basic-face slots (0–19).
    pub(crate) frame_face_id_counter: u32,
    /// Per-window retained layout, owned across cycles (incremental-layout
    /// Phase 0a). Committed at the accepted `break` only; NOT read yet — the
    /// engine still rebuilds every window every cycle. The container a later
    /// phase reuses rows out of.
    retained_window_matrices: std::collections::HashMap<DisplayWindowId, RetainedWindowMatrix>,
    /// Windows that took the Phase 1 cursor-only fast path this frame (their body
    /// rows were reused, not relaid). Populated as each window is laid out, read
    /// by the commit path to attribute rows to `reused_rows` and classify the
    /// window `CursorOnly`. Reset per frame.
    cursor_only_window_ids: std::collections::HashSet<DisplayWindowId>,
    /// Windows that took the Phase 2 pure-scroll fast path this frame, mapped to
    /// `(reused_shifted_row_count, dvpos)`. Read by the commit path to attribute
    /// rows + classify `Scroll` + emit `RowDamage::ReusedShifted`.
    scroll_window_ids: std::collections::HashMap<DisplayWindowId, (usize, f32)>,
    /// Windows that took the Phase 3 localized-edit fast path this frame, mapped
    /// to their reused (verbatim, above-the-edit) row count. Read by the commit
    /// path to attribute rows + classify `Edit`.
    edit_window_ids: std::collections::HashMap<DisplayWindowId, usize>,
    /// Phase 3 below-reuse switch (default true). The localized edit fast path
    /// reuses the rows BELOW the dirty span too (charpos-shifted, same pixel_y),
    /// relaying ONLY the edited line — but ONLY for a simple insert that provably
    /// keeps the edited line one row (the ASCII gate in `build_edit_replay` + the
    /// width gate in `edit_replay`); a newline/tab/wide/wrapping insert or a
    /// delete falls back to above-only. Settable for tests.
    allow_below_reuse: bool,
    /// Instrumentation from the most recent `layout_frame_rust` pass: the
    /// relaid-row-count gate metric (spec §7). Reset per frame.
    layout_stats: LayoutStats,
}

impl LayoutEngine {
    fn reset_frame_output_state(&mut self) {
        self.frame_output.reset();
        self.frame_face_id_counter = BasicFaceId::SENTINEL;
    }

    fn latest_output_window_info(&self, window_id: i64) -> Option<WindowInfo> {
        self.frame_output.latest_window_info(window_id)
    }

    fn latest_output_window_enabled_rows(&self) -> Option<usize> {
        self.frame_output.latest_window_enabled_rows()
    }

    fn finish_frame_output(
        &mut self,
        frame_params: &FrameParams,
    ) -> neomacs_display_protocol::glyph_matrix::FrameDisplayState {
        self.frame_output.finish(frame_params)
    }

    fn render_window_output_decorations(
        &mut self,
        params: &WindowParams,
        frame_params: &FrameParams,
        window_geometry: crate::display_frame_output::WindowFrameGeometry,
        info: &WindowInfo,
        face_resolver: &super::neovm_bridge::FaceResolver,
    ) {
        let mut decoration_face_ids = FrameFaceIdAllocator::new(self.frame_face_id_counter);
        let font_metrics = &mut self.font_metrics;
        self.frame_output.render_window_decorations(
            WindowFrameDecorationsRenderRequest::new(params, frame_params, window_geometry, info),
            ChromeRowRenderServices::new(font_metrics, face_resolver, &mut decoration_face_ids),
        );
        decoration_face_ids.finish_into(&mut self.frame_face_id_counter);
    }

    fn render_latest_window_output_info_effects(
        &mut self,
        curr_window_infos: &mut std::collections::HashMap<DisplayWindowId, WindowInfo>,
    ) {
        let prev_window_infos = &self.prev_window_infos;
        self.frame_output.render_latest_window_info_effects(
            WindowFrameInfoEffectsRenderRequest::new(prev_window_infos),
            curr_window_infos,
        );
    }

    fn render_frame_output_hints(
        &mut self,
        curr_window_infos: &std::collections::HashMap<DisplayWindowId, WindowInfo>,
        frame_params: &FrameParams,
    ) {
        let prev_window_infos = &self.prev_window_infos;
        let prev_selected_window_id = &mut self.prev_selected_window_id;
        let prev_background = &mut self.prev_background;
        self.frame_output
            .render_line_animation_hints(FrameLineAnimationHintsRenderRequest::new(
                prev_window_infos,
                curr_window_infos,
            ));
        self.frame_output
            .render_window_switch_hint(FrameWindowSwitchHintRenderRequest::new(
                prev_selected_window_id,
            ));
        self.frame_output
            .render_theme_transition_hint(FrameThemeTransitionHintRenderRequest::new(
                prev_background,
                frame_params.width,
                frame_params.height,
            ));
        self.frame_output.render_topology_transition_hint(
            FrameTopologyTransitionHintRenderRequest::new(
                prev_window_infos,
                curr_window_infos,
                frame_params.width,
                frame_params.height,
            ),
        );
    }

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
            prev_selected_window_id: DisplayWindowId::new(0),
            prev_background: None,
            frame_output: FrameOutputOwner::new(),
            last_frame_display_state: None,
            frame_face_id_counter: BasicFaceId::SENTINEL,
            retained_window_matrices: std::collections::HashMap::new(),
            cursor_only_window_ids: std::collections::HashSet::new(),
            scroll_window_ids: std::collections::HashMap::new(),
            edit_window_ids: std::collections::HashMap::new(),
            allow_below_reuse: true,
            layout_stats: LayoutStats::default(),
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
            prev_selected_window_id: DisplayWindowId::new(0),
            prev_background: None,
            frame_output: FrameOutputOwner::new(),
            last_frame_display_state: None,
            frame_face_id_counter: BasicFaceId::SENTINEL,
            retained_window_matrices: std::collections::HashMap::new(),
            cursor_only_window_ids: std::collections::HashSet::new(),
            scroll_window_ids: std::collections::HashMap::new(),
            edit_window_ids: std::collections::HashMap::new(),
            allow_below_reuse: true,
            layout_stats: LayoutStats::default(),
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

    /// Instrumentation from the most recent `layout_frame_rust` pass.
    ///
    /// THE gate metric for the incremental-layout phases: a phase ships only
    /// when its bench cases prove the win on relaid-row-count, not wall-time
    /// alone (spec §7). Phase 0a always reports a full rebuild (every body row
    /// relaid, zero reused, all windows `Full`).
    pub fn last_layout_stats(&self) -> &LayoutStats {
        &self.layout_stats
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
        // Incremental-layout instrumentation (Phase 0a): start each frame from
        // a clean slate; populated as the accepted frame is committed below.
        self.layout_stats = LayoutStats::default();
        // Phase 1/2: forget which windows took an incremental fast path last
        // frame; repopulated as windows are laid out this frame.
        self.cursor_only_window_ids.clear();
        self.scroll_window_ids.clear();
        self.edit_window_ids.clear();

        // The font service can exist on the engine even while laying out a
        // terminal frame in tests. Match GNU's redisplay split: window-system
        // frames use realized font pixels, terminal frames stay on cell
        // metrics.

        // Reset the per-redisplay mode-line eval counter. Each chrome row is
        // laid out (and thus its `*-format` evaluated) exactly once per window
        // per frame; the single-eval invariant test asserts this stays at 1.
        crate::display_status_line::reset_mode_line_eval_count();

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

        let (frame_params, curr_window_infos, retained_keys) = loop {
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

            // Snapshot each window's layout inputs for the incremental-layout
            // retained key (Phase 0a). Built by reference before the params are
            // consumed by layout below; only the accepted iteration's snapshot
            // escapes via the `break`, so a resize-retry `continue` discards it.
            let retained_keys: Vec<(DisplayWindowId, RetainedWindowKey)> = window_params_list
                .iter()
                .map(|p| {
                    (
                        DisplayWindowId::new(p.window_id),
                        RetainedWindowKey::from_params(p, evaluator),
                    )
                })
                .collect();

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

            self.reset_frame_output_state();
            let mut curr_window_infos: std::collections::HashMap<DisplayWindowId, WindowInfo> =
                std::collections::HashMap::new();
            let default_resolved = face_resolver.default_face();

            // Set up frame dimensions in the builder
            let frame_identity = if let Some(frame) = evaluator.frame_manager().get(frame_id) {
                let (origin_x, origin_y) = evaluator
                    .frame_manager()
                    .frame_origin_in_root(frame_id)
                    .unwrap_or((frame.left_pos as f32, frame.top_pos as f32));
                Some(FrameOutputIdentity {
                    frame_id: frame.id.0,
                    parent_id: frame.parent_frame.as_frame_id().unwrap_or(0),
                    parent_x: origin_x,
                    parent_y: origin_y,
                    z_order: frame.z_order,
                    undecorated: frame.undecorated,
                    border_width: frame.internal_border_width() as f32,
                    border_color: Color::BLACK,
                    background_alpha: 1.0,
                    no_accept_focus: frame.no_accept_focus,
                })
            } else {
                None
            };
            self.frame_output
                .render_frame_state(FrameOutputStateRenderRequest::new(
                    frame_identity,
                    Color::from_pixel(frame_params.background),
                    frame_params.font_pixel_size,
                    default_resolved,
                    default_metrics,
                ));

            // Clear hit-test data for new frame
            self.hit_data.clear();
            self.display_snapshots.clear();

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
                .filter(|params| !params.is_minibuffer())
                .map(|params| params.bounds.y + params.bounds.height)
                .fold(0.0_f32, f32::max);

            // --- Phase A (single-threaded gather, spec §4.5) ---
            //
            // Classify each window's incremental fast path UP FRONT, before any
            // window is laid out — reading the previous frame's retained matrices
            // (untouched until this frame's commit) + the evaluator. This is also
            // the multi-window-same-buffer ordering guarantee (every window of a
            // buffer is classified before any reset), and the seam the
            // (feature-flagged) window parallelism builds on: Phase B below
            // positions windows from these plans. Today Phase B is single-threaded
            // and the output is identical to building each plan inline.
            let window_plans: Vec<(Option<CursorOnlyReplay>, Option<ScrollReplay>, bool)> =
                window_params_list
                    .iter()
                    .map(|params| {
                        let cursor_only = self.build_cursor_only_replay(params, evaluator);
                        let mut is_edit = false;
                        let scroll = if cursor_only.is_none() {
                            if let Some(scroll) = self.build_scroll_replay(params, evaluator) {
                                Some(scroll)
                            } else if let Some(edit) = self.build_edit_replay(params, evaluator) {
                                is_edit = true;
                                Some(edit)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        (cursor_only, scroll, is_edit)
                    })
                    .collect();

            // --- Phase B (per-window layout; single-threaded today) ---
            for (params, (cursor_only_replay, scroll_replay, is_edit)) in
                window_params_list.iter().zip(window_plans)
            {
                tracing::debug!(
                    "layout window: id={} buf={} bounds=({:.0},{:.0},{:.0},{:.0}) mini={} selected={} mode_line_h={:.0}",
                    params.window_id,
                    params.buffer_id,
                    params.bounds.x,
                    params.bounds.y,
                    params.bounds.width,
                    params.bounds.height,
                    params.is_minibuffer(),
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
                        buffer_name: buffer
                            .map(|b| b.name_runtime_string_owned())
                            .unwrap_or_default(),
                        buffer_file_name: buffer
                            .and_then(|b| b.file_name_runtime_string_owned())
                            .unwrap_or_default(),
                        modified: buffer.map(|b| b.is_modified()).unwrap_or(false),
                    }
                };
                self.frame_output
                    .render_window_info(WindowFrameInfoRenderRequest::new(params, metadata));
                self.render_latest_window_output_info_effects(&mut curr_window_infos);

                // Simplified layout for this window (no face resolution, no overlays)
                self.layout_window_rust(
                    evaluator,
                    frame_id,
                    params,
                    &frame_params,
                    &face_resolver,
                    window_geometry.reserve_terminal_right_border_col,
                    MAX_WINDOW_VISIBILITY_RETRIES,
                    cursor_only_replay,
                    scroll_replay,
                    is_edit,
                );

                if let Some(info) = self.latest_output_window_info(params.window_id) {
                    self.render_window_output_decorations(
                        params,
                        &frame_params,
                        window_geometry,
                        &info,
                        &face_resolver,
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
                if let Some(mini_rows_used) = self.latest_output_window_enabled_rows() {
                    if let Some(mini_params) = window_params_list.last() {
                        if mini_params.is_minibuffer() {
                            let char_h = frame_params.char_height.max(1.0);
                            let allocated_rows =
                                (mini_params.bounds.height / char_h).floor().max(1.0) as usize;
                            let frame_rows = frame_params.height / char_h;
                            let max_mini_lines = max_mini_window_lines_for_window(
                                evaluator,
                                mini_params,
                                frame_rows,
                            );
                            // GNU `resize_mini_window` reads `Vresize_mini_windows`
                            // after `set_buffer_internal (XBUFFER (w->contents))`
                            // (xdisp.c:13296,13318), so a buffer-local binding in the
                            // mini-window's buffer takes effect. Read buffer-local-
                            // then-global from that buffer, not the raw global.
                            let resize_policy = evaluator
                                .buffer_manager()
                                .get(neovm_core::buffer::BufferId(mini_params.buffer_id))
                                .and_then(|buffer| buffer.buffer_local_value("resize-mini-windows"))
                                .or_else(|| {
                                    evaluator
                                        .obarray()
                                        .symbol_value("resize-mini-windows")
                                        .copied()
                                });
                            let resize_mode =
                                ResizeMiniWindowsMode::from_lisp_value(resize_policy.as_ref());

                            // GNU `resize_mini_window` measures the mini-window's
                            // CONTENT height via `move_it_to (ZV)` (xdisp.c:13340) and
                            // shrinks a grow-only window when `height < old_height &&
                            // (exact_p || BEGV == ZV)` (xdisp.c:13395). An empty
                            // mini/echo buffer is exactly one line.
                            //
                            // We measure with the glyph matrix
                            // (`latest_output_window_enabled_rows`) instead, which can
                            // be STALE: after find-file the mini-window is laid out
                            // with zero text height and SKIPPED, so its matrix keeps
                            // the vertico candidate row count and the echo area never
                            // shrinks back (it stays ~9 rows tall and empty). Mirror
                            // GNU's content measurement for the empty case: when the
                            // buffer the window actually displays is empty (BEGV == ZV)
                            // the used height is 1, regardless of the stale matrix.
                            //
                            // `buf_id` is that displayed buffer: the swapped
                            // ` *Echo Area 0*` for an inactive mini-window (GNU
                            // `with_echo_area_buffer`), or the window's own buffer when
                            // the minibuffer is active.
                            let mini_window_id =
                                neovm_core::window::WindowId(mini_params.window_id as u64);
                            let minibuffer_active =
                                evaluator.minibuffer_window_is_active(mini_window_id);
                            let buf_id = if !minibuffer_active {
                                evaluator.ensure_echo_area_buffers();
                                evaluator
                                    .buffer_manager()
                                    .find_buffer_by_name(" *Echo Area 0*")
                                    .unwrap_or(neovm_core::buffer::BufferId(mini_params.buffer_id))
                            } else {
                                neovm_core::buffer::BufferId(mini_params.buffer_id)
                            };
                            let visible_region_empty = evaluator
                                .buffer_manager()
                                .get(buf_id)
                                .map(|b| b.accessible_emacs_byte_range().is_empty())
                                .unwrap_or(true);
                            // For an INACTIVE mini-window, measure the displayed echo
                            // buffer's content height directly (GNU `resize_mini_window`
                            // measures `w->contents` via `move_it_to (ZV)`) rather than
                            // the engine's cached enabled-row count. That matrix goes
                            // STALE: the inactive mini-window is laid out with ~zero text
                            // height and skipped, so it keeps the active minibuffer's
                            // candidate-overlay row count (e.g. 35). With the stale
                            // matrix, the instant any echo message ("Quit" after C-g)
                            // makes the buffer non-empty, the window re-grows to that
                            // stale height. Content measurement keeps "Quit"/empty one
                            // line. When the minibuffer is ACTIVE the matrix is fresh and
                            // includes the candidate overlay, so keep using it there.
                            let mini_rows_used = if !minibuffer_active {
                                let cols = (mini_params.bounds.width
                                    / frame_params.char_width.max(1.0))
                                .floor()
                                .max(1.0) as usize;
                                let text = evaluator
                                    .buffer_manager()
                                    .get(buf_id)
                                    .map(|b| b.full_text_string())
                                    .unwrap_or_default();
                                echo_content_rows(&text, cols)
                            } else {
                                mini_rows_used
                            };

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
                                // `exact_p` is GNU's post-command exact resize
                                // (`resize_echo_area_exactly`, run with
                                // `minibuf_level == 0`); `visible_region_empty`
                                // (computed above) is the `BEGV == ZV` case.
                                let exact = evaluator.echo_area_resize_exact_pending();
                                let should_shrink =
                                    resize_mode.should_shrink(exact, visible_region_empty);

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

            self.render_frame_output_hints(&curr_window_infos, &frame_params);

            break (frame_params, curr_window_infos, retained_keys);
        };

        let mut frame_display_state = self.finish_frame_output(&frame_params);

        // Embed the user-defined fringe bitmaps once per frame so the renderer
        // can expand any `GlyphRow::left_fringe_bitmap` reference (magit section
        // heading fold arrows). GC-safe: copied out as plain `u16`/`u8` data.
        for (index, bitmap) in evaluator.fringe_bitmap_registry().iter_indexed() {
            if index > u32::from(u16::MAX) {
                continue;
            }
            frame_display_state.fringe_bitmaps.insert(
                index as u16,
                neomacs_display_protocol::frame_glyphs::FringeBitmapData {
                    bits: bitmap.bits.clone(),
                    width: bitmap.width,
                    height: bitmap.height,
                    period: bitmap.period,
                    align: bitmap.align.as_u8(),
                },
            );
        }

        // NOTE: GlyphMatrix vs FrameGlyphBuffer character count validation removed.
        // FrameGlyphBuffer no longer receives glyph output; the DisplayOutputBuilder
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
        if frame_display_state.parent_id == DisplayFrameId::new(0) {
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

        // --- Incremental-layout commit (Phase 0a) ---
        //
        // Populate the relaid-row-count gate metric and retain each accepted
        // window's matrix. We are past the accepted `break`, so this never runs
        // on a resize-retry `continue`. Phase 0a always full-rebuilds: every
        // enabled row is `relaid`, every window is classified `Full`, and the
        // retained matrices are written but NOT read (no fast path exists yet).
        {
            let key_map: std::collections::HashMap<DisplayWindowId, RetainedWindowKey> =
                retained_keys.into_iter().collect();
            let frame_state = self
                .last_frame_display_state
                .as_mut()
                .expect("frame display state just set");
            let mut retained: std::collections::HashMap<DisplayWindowId, RetainedWindowMatrix> =
                std::collections::HashMap::new();
            // Snapshot the resolved Face for every face_id each window's rows
            // reference, from THIS frame's faces table. The fast paths reuse these
            // rows verbatim carrying their prior-frame face_ids, but the next
            // frame's faces table is rebuilt from scratch (counter reset to
            // SENTINEL, faces cleared), so a replay MUST re-register these
            // (face_id -> Face) pairs and reserve their id range against the chrome
            // re-walk — else the reused glyphs resolve to a missing/wrong face at
            // render (face-id collision audit fix).
            let mut window_face_snapshots: std::collections::HashMap<
                DisplayWindowId,
                std::collections::HashMap<u32, neomacs_display_protocol::face::Face>,
            > = frame_state
                .window_matrices
                .iter()
                .map(|entry| {
                    let wid = DisplayWindowId::new(entry.window_id as i64);
                    let mut faces = std::collections::HashMap::new();
                    for row in &entry.matrix.rows {
                        for area in &row.glyphs {
                            for g in area {
                                if let std::collections::hash_map::Entry::Vacant(slot) =
                                    faces.entry(g.face_id)
                                {
                                    if let Some(face) = frame_state.faces.get(&g.face_id) {
                                        slot.insert(face.clone());
                                    }
                                }
                            }
                        }
                    }
                    (wid, faces)
                })
                .collect();
            for entry in &mut frame_state.window_matrices {
                let window_id = DisplayWindowId::new(entry.window_id as i64);
                let cursor_only = self.cursor_only_window_ids.contains(&window_id);
                let scroll_reused = self.scroll_window_ids.get(&window_id).copied();
                let edit_reused = self.edit_window_ids.get(&window_id).copied();
                // Fast paths classify body vs chrome by ROLE (they reuse the
                // buffer-text `Text` rows and re-walk all chrome roles); a full
                // rebuild counts by the `mode_line` flag (the Phase 0a baseline).
                let role_based = cursor_only || scroll_reused.is_some() || edit_reused.is_some();
                let mut enabled_body = 0usize;
                let mut enabled_chrome = 0usize;
                for row in &entry.matrix.rows {
                    if !row.enabled {
                        continue;
                    }
                    let is_chrome = if role_based {
                        RetainedWindowMatrix::is_chrome_role(row.role)
                    } else {
                        row.mode_line
                    };
                    if is_chrome {
                        enabled_chrome += 1;
                    } else {
                        enabled_body += 1;
                    }
                }
                self.layout_stats.relaid_chrome_rows += enabled_chrome;
                if cursor_only {
                    // Body rows were reused verbatim (0 relaid); chrome re-walked.
                    self.layout_stats.reused_rows += enabled_body;
                    self.layout_stats.record_window_class(LayoutClass::CursorOnly);
                } else if let Some((reused, _dvpos)) = scroll_reused {
                    // Overlapping rows reused shifted; the rest were newly exposed
                    // and walked.
                    let reused = reused.min(enabled_body);
                    self.layout_stats.reused_shifted_rows += reused;
                    self.layout_stats.relaid_body_rows += enabled_body - reused;
                    self.layout_stats.record_window_class(LayoutClass::Scroll);
                } else if let Some(reused) = edit_reused {
                    // Rows above the edit reused verbatim; the dirty line + below
                    // were relaid.
                    let reused = reused.min(enabled_body);
                    self.layout_stats.reused_rows += reused;
                    self.layout_stats.relaid_body_rows += enabled_body - reused;
                    self.layout_stats.record_window_class(LayoutClass::Edit);
                } else {
                    self.layout_stats.relaid_body_rows += enabled_body;
                    self.layout_stats.record_window_class(LayoutClass::Full);
                }

                // Phase 5 (#44) per-row damage, parallel to matrix.rows. The fast
                // paths reuse the FIRST `reused` enabled body rows (cursor-only
                // reuses all); chrome + disabled + relaid body rows are `New`.
                {
                    let mut damage = Vec::with_capacity(entry.matrix.rows.len());
                    let mut body_seen = 0usize;
                    for row in &entry.matrix.rows {
                        let is_chrome = if role_based {
                            RetainedWindowMatrix::is_chrome_role(row.role)
                        } else {
                            row.mode_line
                        };
                        if !row.enabled || is_chrome {
                            damage.push(RowDamage::New);
                            continue;
                        }
                        let d = if cursor_only {
                            RowDamage::Reused
                        } else if let Some((reused, dvpos)) = scroll_reused {
                            if body_seen < reused {
                                RowDamage::ReusedShifted { dvpos }
                            } else {
                                RowDamage::New
                            }
                        } else if let Some(reused) = edit_reused {
                            if body_seen < reused {
                                RowDamage::Reused
                            } else {
                                RowDamage::New
                            }
                        } else {
                            RowDamage::New
                        };
                        damage.push(d);
                        body_seen += 1;
                    }
                    entry.damage = damage;
                }

                // Probe-pass exclusion: a window that laid out <=1 enabled row
                // is the scroll-off hazard (spec §4.1); never retain it as a
                // clean reusable matrix.
                if enabled_body + enabled_chrome <= 1 {
                    continue;
                }
                if let Some(key) = key_map.get(&window_id) {
                    // The window's display snapshot (point-independent body rows
                    // + per-span display points) is needed to replay this window
                    // on a later cursor-only pass. A retained window without one
                    // cannot be reused, so skip retention if it is missing.
                    let Some(display_snapshot) = self
                        .display_snapshots
                        .iter()
                        .find(|snapshot| snapshot.window_id.0 == entry.window_id)
                        .cloned()
                    else {
                        continue;
                    };
                    let damage = vec![RowDamage::New; entry.matrix.rows.len()];
                    retained.insert(
                        window_id,
                        RetainedWindowMatrix {
                            matrix: entry.matrix.clone(),
                            key: key.clone(),
                            validity: MatrixValidity::Valid,
                            damage,
                            display_snapshot,
                            faces: window_face_snapshots.remove(&window_id).unwrap_or_default(),
                        },
                    );
                }
            }
            self.retained_window_matrices = retained;

            // Phase 3 redisplay ACK: reset each laid-out buffer's unchanged-region
            // accumulator at the committed (accepted) break — NEVER on a
            // retry/`continue` (which would under-invalidate, spec §6). From here
            // the accumulated dirty span is the edits the NEXT frame must relay.
            let mut acked: std::collections::HashSet<u64> = std::collections::HashSet::new();
            for key in key_map.values() {
                if acked.insert(key.buffer_id) {
                    if let Some(buffer) = evaluator
                        .buffer_manager()
                        .get(neovm_core::buffer::BufferId(key.buffer_id))
                    {
                        buffer.reset_unchanged_region();
                    }
                }
            }
        }

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
    /// Phase 1: if this window's previous-frame matrix can be reused with only
    /// the cursor re-decorated (point moved, every other layout input and the
    /// neovm-core invalidation ticks unchanged, cursor row structurally simple),
    /// return the replay bundle; else `None` (→ full rebuild). Reads the retained
    /// matrix from the *previous* frame (committed at the prior frame's accepted
    /// break; never overwritten until this frame's commit).
    fn build_cursor_only_replay(
        &self,
        params: &WindowParams,
        evaluator: &neovm_core::emacs_core::Context,
    ) -> Option<CursorOnlyReplay> {
        // The cursor-only fast path applies to ANY window. The render cursor branch
        // handles both styles (replay.cursor_style is hollow for a non-selected
        // window), and the reused rows' faces are re-registered + their id range
        // reserved at render time, so a non-selected window co-resident with a
        // re-laid window no longer corrupts face resolution (the face-id collision
        // audit fix). The dominant multi-window win: a non-selected window that did
        // not change reuses its body verbatim instead of full-rebuilding when
        // another window is edited.
        let window_id = DisplayWindowId::new(params.window_id);
        let prev = self.retained_window_matrices.get(&window_id)?;
        let curr_key = RetainedWindowKey::from_params(params, evaluator);
        prev.cursor_only_replay(&curr_key)
    }

    /// Smooth scroll (Phase 1): the laid-out body rows of `window_id` from the
    /// retained matrix, as `(start_charpos, height_px)` metrics in top-to-bottom
    /// order, for resolving a pixel scroll via
    /// [`crate::pixel_scroll::resolve_pixel_scroll`]. `None` if the window has no
    /// retained matrix yet or no body (non-chrome, text-displaying) rows.
    pub fn current_body_row_metrics(
        &self,
        window_id: DisplayWindowId,
    ) -> Option<Vec<crate::pixel_scroll::ScrollRowMetric>> {
        let retained = self.retained_window_matrices.get(&window_id)?;
        let rows: Vec<crate::pixel_scroll::ScrollRowMetric> = retained
            .matrix
            .rows
            .iter()
            .filter(|row| {
                row.enabled
                    && row.displays_text
                    && !RetainedWindowMatrix::is_chrome_role(row.role)
            })
            .map(|row| crate::pixel_scroll::ScrollRowMetric {
                start_charpos: row.start_charpos as i64,
                height_px: row.height_px.round() as i32,
            })
            .collect();
        if rows.is_empty() { None } else { Some(rows) }
    }

    /// Phase 2: if this window's previous-frame matrix can be reused after a
    /// whole-row scroll (overlapping rows shifted, only newly-exposed rows
    /// walked), return the scroll replay; else `None`. Selected window only, for
    /// the same reason as [`Self::build_cursor_only_replay`].
    fn build_scroll_replay(
        &self,
        params: &WindowParams,
        evaluator: &neovm_core::emacs_core::Context,
    ) -> Option<ScrollReplay> {
        if !params.selected {
            return None;
        }
        let window_id = DisplayWindowId::new(params.window_id);
        let prev = self.retained_window_matrices.get(&window_id)?;
        let curr_key = RetainedWindowKey::from_params(params, evaluator);
        prev.scroll_replay(&curr_key)
    }

    /// Phase 3: if this window's previous-frame matrix can be reused after a
    /// localized (plain) edit — reuse the rows above the dirty span verbatim and
    /// re-walk only the dirty line + below — return the replay (a [`ScrollReplay`]
    /// with `dvpos = 0`); else `None`. Reads the accumulated dirty char span from
    /// the buffer. Selected window only.
    fn build_edit_replay(
        &self,
        params: &WindowParams,
        evaluator: &neovm_core::emacs_core::Context,
    ) -> Option<ScrollReplay> {
        if !params.selected {
            return None;
        }
        let window_id = DisplayWindowId::new(params.window_id);
        let prev = self.retained_window_matrices.get(&window_id)?;
        let curr_key = RetainedWindowKey::from_params(params, evaluator);
        let buffer = evaluator
            .buffer_manager()
            .get(neovm_core::buffer::BufferId(params.buffer_id))?;
        let (dirty_start, dirty_end) = buffer.changed_char_range()?;
        // Below-reuse SAFETY GATE: only a simple insert — every inserted char is
        // printable ASCII (graphic or space) — keeps the edited line one logical
        // line of char_width glyphs, which is what makes the rows-below reuse
        // (shift charpos, keep pixel_y) sound. A newline/tab/wide char (or a
        // delete, dirty_end == dirty_start) escalates to above-only. Combined with
        // edit_replay's monospace + width check this proves no row-structure change.
        let simple_insert = self.allow_below_reuse
            && dirty_end > dirty_start
            && (dirty_start..dirty_end).all(|cp| {
                let byte = buffer.char_pos_to_emacs_byte_pos_clamped(
                    neovm_core::buffer::CharPos0::new(cp as usize),
                );
                matches!(buffer.char_at_emacs_byte_pos(byte), Some(c) if c.is_ascii_graphic() || c == ' ')
            });
        prev.edit_replay(&curr_key, dirty_start, simple_insert)
    }

    fn layout_window_rust(
        &mut self,
        evaluator: &mut neovm_core::emacs_core::Context,
        frame_id: neovm_core::window::FrameId,
        params: &WindowParams,
        frame_params: &FrameParams,
        face_resolver: &super::neovm_bridge::FaceResolver,
        reserve_right_border_col: bool,
        remaining_visibility_retries: usize,
        // Phase A (gather) classified this window's incremental fast path against
        // the *original* params (before any echo-buffer swap below), reading the
        // same retained key the predicate was snapshotted from. Phase B (here)
        // consumes the plan inside the render path in place of the body walk.
        // `is_edit` only steers the commit-path stats classification.
        cursor_only_replay: Option<CursorOnlyReplay>,
        scroll_replay: Option<ScrollReplay>,
        is_edit: bool,
    ) {
        let window_id = neovm_core::window::WindowId(params.window_id as u64);
        let scroll_dvpos = scroll_replay.as_ref().map(|replay| replay.dvpos).unwrap_or(0.0);
        // GNU `with_echo_area_buffer` (xdisp.c:12904): an inactive mini-window
        // displays the echo-area buffer (whose contents `set_message_1` mirrored
        // the current message into), NOT the ordinary buffer attached to the
        // window record. Resolve the echo buffer for layout only — GNU does the
        // same temporary `wset_buffer` for display without a full
        // set-window-buffer; `params.buffer_id` (the window record) is untouched.
        let buf_id = if params.is_minibuffer() && !evaluator.minibuffer_window_is_active(window_id)
        {
            // GNU `with_echo_area_buffer` `ensure_echo_area_buffers ()` first, so
            // ` *Echo Area 0*` always exists here — empty when there is no current
            // message. This is what makes an idle echo area blank instead of
            // re-displaying the buffer the mini-window record happens to point at
            // (which is the frame's root buffer, window/mod.rs).
            evaluator.ensure_echo_area_buffers();
            evaluator
                .buffer_manager()
                .find_buffer_by_name(" *Echo Area 0*")
                .unwrap_or_else(|| neovm_core::buffer::BufferId(params.buffer_id))
        } else {
            neovm_core::buffer::BufferId(params.buffer_id)
        };
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

        // When swapped to the echo buffer (above), the window record's position
        // markers still point into the minibuffer's own buffer, so the source
        // read would use that stale (short) accessible range and truncate the
        // echo message. GNU `with_echo_area_buffer` moves `pointm`/`old_pointm`
        // to BEG and lets the echo buffer's BEGV/ZV bound the display; mirror
        // that by resetting the position params to the echo buffer's full range.
        let echo_swapped_params;
        let params: &WindowParams = if buf_id.0 != params.buffer_id {
            use super::neovm_bridge::LayoutBufferView;
            let mut swapped = params.clone();
            swapped.buffer_id = buf_id.0;
            swapped.window_start = 0;
            swapped.window_end = 0;
            swapped.point = 0;
            swapped.buffer_begv = 0;
            swapped.buffer_size = buffer.layout_point_max_char_pos().get() as i64;
            echo_swapped_params = swapped;
            &echo_swapped_params
        } else {
            params
        };

        // Capture buffer name as owned String for use in mode-line fallback.
        // This avoids holding a borrow on `evaluator` through eval calls.
        let buffer_name = buffer.name().to_owned();
        let render_outcome = BufferWindowRenderRequest::new(
            frame_id,
            window_id,
            params,
            frame_params,
            buf_id,
            buffer,
            &buffer_name,
            reserve_right_border_col,
        )
        .render_into(
            BufferSourceRenderAttemptContext::from_frame_output_owner(
                &mut self.frame_output,
                evaluator,
                &mut self.font_metrics,
                face_resolver,
                &mut self.frame_face_id_counter,
                &mut self.hit_data,
                &mut self.display_snapshots,
            ),
            &mut self.text_buf,
            remaining_visibility_retries,
            cursor_only_replay,
            scroll_replay,
        );

        let redisplay_positions = match render_outcome {
            BufferSourceRenderAttemptOutcome::Skipped => return,
            BufferSourceRenderAttemptOutcome::Retry { window_start } => {
                let mut retry_params = params.clone();
                retry_params.window_start = window_start;
                retry_params.window_end = 0;
                // A visibility retry re-flows the window from a new window_start,
                // so the Phase A fast-path plan (snapshotted against the original
                // window_start) no longer applies — re-lay from scratch.
                self.layout_window_rust(
                    evaluator,
                    frame_id,
                    &retry_params,
                    frame_params,
                    face_resolver,
                    reserve_right_border_col,
                    remaining_visibility_retries.saturating_sub(1),
                    None,
                    None,
                    false,
                );
                return;
            }
            BufferSourceRenderAttemptOutcome::Finished {
                redisplay_positions,
                cursor_only,
                scroll_reused_rows,
            } => {
                if cursor_only {
                    self.cursor_only_window_ids
                        .insert(DisplayWindowId::new(params.window_id));
                }
                if let Some(reused) = scroll_reused_rows {
                    let window_id = DisplayWindowId::new(params.window_id);
                    if is_edit {
                        self.edit_window_ids.insert(window_id, reused);
                    } else {
                        self.scroll_window_ids
                            .insert(window_id, (reused, scroll_dvpos));
                    }
                }
                redisplay_positions
            }
        };

        tracing::debug!(
            "  layout_window_rust: window_start={} window_end={}",
            redisplay_positions.window_start.as_i64(),
            redisplay_positions.window_end.as_i64()
        );
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

impl LayoutEngine {
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
        let tab_bar_face =
            face_resolver.default_base_face_for_origin_without_buffer(&DisplayOrigin::TabBar);
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
        let Some(rendered_tab_bar) = self.frame_output.render_frame_tab_bar_row(
            FrameTabBarDisplayRowRequest {
                row_index,
                y: tab_bar_y,
                width,
                height: tab_bar_height,
                metrics: DisplayRowFallbackMetrics::from_frame_defaults(
                    frame_params,
                    tab_bar_ascent,
                ),
                base_face: &tab_bar_face,
                text: tab_bar.text,
            },
            ChromeRowRenderServices::new(&mut self.font_metrics, face_resolver, &mut face_ids),
            evaluator.display_host.as_deref(),
        ) else {
            return None;
        };
        face_ids.finish_into(&mut self.frame_face_id_counter);
        let FrameTabBarDisplayRowRender::Measured(measured) = rendered_tab_bar else {
            return None;
        };
        let actual_tab_bar_height = measured.bounds().height;
        self.frame_output
            .set_tab_bar(neomacs_display_protocol::frame_glyphs::FrameTabBarState {
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
        layout_mock_frame_content(content, char_w, char_h, &mut self.font_metrics)
    }
}

#[cfg(test)]
#[path = "engine_test.rs"]
mod tests;
