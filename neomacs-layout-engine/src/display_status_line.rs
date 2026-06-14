//! Display-walker chrome row rendering.
//!
//! Mode-line, header-line, tab-line, tab-bar, and minibuffer echo rows share
//! the face realization helpers defined here. The generic display-row spec,
//! property harvester, and row renderer live in `display_row`; this module
//! retains the status-line filename because it grew from the older
//! mode-line-only path.
//!
//! History: this module started as a divergent
//! parallel implementation of display-line rendering that did not
//! process display properties and dropped doom-modeline's
//! (space :align-to ...) forms. Steps 3.3' through 3.6 of the
//! display-engine unification plan merged it into the backend
//! trait and renamed the file to reflect its new role.

use super::engine::LayoutEngine;
use super::neovm_bridge::{FaceResolver, ResolvedFace};
use super::window_output::{ChromeRowOutput, DisplayProgressSink, WindowOutputEmitter};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_row::{
    DisplayRowBoundsPolicy, DisplayRowLispStringRenderRequest, DisplayRowLispStringSourceSession,
    DisplayRowLispStringSourceSessionRequest, DisplayRowOwner, DisplayRowRenderContext,
    DisplayRowRenderStop, DisplayRowRenderer, DisplayRowSourceRequestPolicy, FrameChromeKind,
    MeasuredDisplayRow, RenderedDisplayRow, WindowChromeKind, install_measured_frame_chrome_row,
    install_measured_window_display_row, install_rendered_display_row,
};
pub(crate) use crate::display_row::{
    DisplayRowFace, DisplayRowFaceRealizer, DisplayRowOutputProgress,
};
use crate::display_row_builder::DisplayTabPolicy;
use crate::matrix_builder::GlyphMatrixBuilder;
#[cfg(test)]
use neomacs_display_protocol::face::BoxType;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow};
use neomacs_display_protocol::types::Rect;
use neovm_core::buffer::{BufferId, EmacsBytePos, EmacsByteRange};
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::keymap::{KeymapMarker, is_list_keymap};
use neovm_core::emacs_core::value::list_to_vec;
use neovm_core::window::WindowId;
use strum::{EnumString, IntoStaticStr};

fn empty_minibuffer_echo_row(y: f32, ascent: f32, row_height: f32) -> Vec<RenderedDisplayRow> {
    let mut row = GlyphRow::new(GlyphRowRole::Minibuffer);
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

pub(crate) enum FrameTabBarDisplayRowRender {
    Empty,
    Measured(MeasuredDisplayRow),
}

#[derive(Clone, Copy)]
pub(crate) struct WindowChromeDisplayText {
    value: Value,
}

impl WindowChromeDisplayText {
    pub(crate) fn new(value: Value, _selected_window: bool) -> Self {
        Self { value }
    }

    fn value(self) -> Value {
        self.value
    }
}

pub(crate) struct WindowChromeDisplayRowRequest<'face> {
    pub(crate) window_id: u64,
    pub(crate) kind: WindowChromeKind,
    pub(crate) matrix_row: usize,
    pub(crate) output: ChromeRowOutput,
    pub(crate) bounds: Rect,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) tab_policy: DisplayTabPolicy,
    pub(crate) base_face: &'face ResolvedFace,
    pub(crate) symbol_values: std::collections::HashMap<String, Value>,
    pub(crate) text: WindowChromeDisplayText,
}

pub(crate) struct InactiveMinibufferDisplayRowRequest<'face> {
    pub(crate) window_id: u64,
    pub(crate) window_bounds: Rect,
    pub(crate) text_bounds: Rect,
    pub(crate) selected: bool,
    pub(crate) text_width: f32,
    pub(crate) row_height: f32,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) base_face: &'face ResolvedFace,
}

pub(crate) struct EchoMinibufferDisplayRowsRequest<'face> {
    pub(crate) window_id: u64,
    pub(crate) window_bounds: Rect,
    pub(crate) text_bounds: Rect,
    pub(crate) selected: bool,
    pub(crate) text_width: f32,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) row_height: f32,
    pub(crate) base_face: &'face ResolvedFace,
    pub(crate) message: Value,
    pub(crate) max_rows: usize,
    pub(crate) truncate_lines: bool,
    pub(crate) reserve_right_special_col: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
pub(crate) enum ResizeMiniWindowsMode {
    #[strum(to_string = "nil")]
    Disabled,
    #[strum(to_string = "grow-only")]
    GrowOnly,
    #[strum(to_string = "t")]
    Exact,
}

impl ResizeMiniWindowsMode {
    pub(crate) fn from_lisp_value(value: Option<&Value>) -> Self {
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

    pub(crate) fn should_grow(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub(crate) fn should_shrink(self, visible_region_empty: bool) -> bool {
        match self {
            Self::Disabled => false,
            Self::GrowOnly => visible_region_empty,
            Self::Exact => true,
        }
    }
}

#[cfg(test)]
pub(crate) fn eval_status_line_format(
    evaluator: &mut Context,
    format_symbol: &str,
    window_id: i64,
    buffer_id: u64,
    target_cols: usize,
) -> Option<String> {
    eval_status_line_format_value(evaluator, format_symbol, window_id, buffer_id, target_cols)
        .and_then(|val| val.as_runtime_string_owned())
        .filter(|s| !s.is_empty())
}

pub(crate) fn eval_status_line_format_value(
    evaluator: &mut Context,
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

pub(crate) struct BuiltTabBar {
    pub(crate) text: Value,
    pub(crate) items: Vec<neomacs_display_protocol::ui_types::TabBarItem>,
}

pub(crate) struct ScratchGcRootScope {
    saved_len: usize,
}

impl ScratchGcRootScope {
    pub(crate) fn new() -> Self {
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

pub(crate) fn build_tab_bar_display(
    evaluator: &mut Context,
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

pub(crate) fn max_mini_window_lines(evaluator: &Context, frame_rows: f32) -> f32 {
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

pub(crate) fn message_truncate_lines(evaluator: &Context) -> bool {
    evaluator
        .obarray()
        .symbol_value("message-truncate-lines")
        .is_some_and(|value| !value.is_nil())
}

pub(crate) fn minibuffer_echo_message_for_window(
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

pub(crate) fn minibuffer_resize_line_count(
    buffer: &neovm_core::buffer::Buffer,
    window_id: u64,
) -> usize {
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

fn window_chrome_glyph_row_role(kind: WindowChromeKind) -> GlyphRowRole {
    match kind {
        WindowChromeKind::TabLine => GlyphRowRole::TabLine,
        WindowChromeKind::HeaderLine => GlyphRowRole::HeaderLine,
        WindowChromeKind::ModeLine => GlyphRowRole::ModeLine,
    }
}

impl LayoutEngine {
    pub(crate) fn realize_display_row_face(
        &mut self,
        face_id: u32,
        face: &ResolvedFace,
        char_w: f32,
        ascent: f32,
        row_height: f32,
    ) -> DisplayRowFace {
        DisplayRowFaceRealizer::new(&mut self.font_metrics)
            .realize_face(face_id, face, char_w, ascent, row_height)
    }

    pub(crate) fn display_row_height_for_face(
        &mut self,
        face: &ResolvedFace,
        char_w: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> f32 {
        DisplayRowFaceRealizer::new(&mut self.font_metrics).row_height_for_face(
            face,
            char_w,
            fallback_ascent,
            fallback_row_height,
        )
    }

    pub(crate) fn render_window_chrome_display_row(
        &mut self,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        request: WindowChromeDisplayRowRequest<'_>,
    ) -> Option<MeasuredDisplayRow> {
        let output = request.output;
        let owner = DisplayRowOwner::WindowChrome {
            window_id: request.window_id,
            kind: request.kind,
        };
        let row_request = DisplayRowSourceRequestPolicy::new(
            request.bounds.y,
            request.bounds.width,
            request.bounds.height,
            request.char_width,
            request.ascent,
            request.tab_policy,
            window_chrome_glyph_row_role(request.kind),
        )
        .with_symbol_values(request.symbol_values);
        let render_request = DisplayRowLispStringRenderRequest::from_base_face_policy(
            row_request,
            face_ids,
            request.base_face,
            request.text.value(),
        );
        let mut builder = std::mem::replace(&mut self.matrix_builder, GlyphMatrixBuilder::new());
        output_emitter.begin_chrome_progress(evaluator, output);
        let mut render_context = DisplayRowRenderContext::new(
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
        );
        let mut renderer = DisplayRowRenderer::new(&mut self.font_metrics);
        let rendered_row = render_request.render_with_context(&mut renderer, &mut render_context);
        let measured_row = rendered_row.map(|rendered| {
            MeasuredDisplayRow::new(
                owner,
                request.matrix_row.min(u32::MAX as usize) as u32,
                request.bounds,
                rendered,
                DisplayRowBoundsPolicy::PreserveAllocatedMinimum,
            )
        });
        if let Some(ref measured_row) = measured_row {
            install_measured_window_display_row(&mut builder, measured_row);
            output_emitter.emit_chrome_progress(evaluator, output, measured_row.output_progress());
        }
        self.matrix_builder = builder;
        if let Some(ref measured_row) = measured_row {
            output_emitter.finish_chrome_progress(measured_row.output_progress());
        }
        measured_row
    }

    pub(crate) fn render_frame_tab_bar_display_row(
        &mut self,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        row_index: u32,
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        row_height: f32,
        tab_bar_face: &ResolvedFace,
        rendered_text: Value,
    ) -> Option<FrameTabBarDisplayRowRender> {
        let row_request = DisplayRowSourceRequestPolicy::new(
            y,
            width,
            row_height,
            char_width,
            ascent,
            DisplayTabPolicy::every(8),
            GlyphRowRole::TabBar,
        );
        let render_request = DisplayRowLispStringRenderRequest::from_base_face_policy(
            row_request,
            face_ids,
            tab_bar_face,
            rendered_text,
        );
        let mut render_context =
            DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        let mut renderer = DisplayRowRenderer::new(&mut self.font_metrics);
        let rendered = render_request.render_with_context(&mut renderer, &mut render_context)?;
        if rendered.row.glyphs[GlyphArea::Text.index()].is_empty() {
            return Some(FrameTabBarDisplayRowRender::Empty);
        }
        let measured = MeasuredDisplayRow::new(
            DisplayRowOwner::FrameChrome {
                kind: FrameChromeKind::TabBar,
            },
            row_index,
            Rect::new(0.0, y, width, height),
            rendered,
            DisplayRowBoundsPolicy::MeasureContent,
        );
        install_measured_frame_chrome_row(
            &mut self.matrix_builder,
            &mut self.pending_frame_chrome_rows,
            &measured,
        );
        Some(FrameTabBarDisplayRowRender::Measured(measured))
    }

    pub(crate) fn render_inactive_minibuffer_window(
        &mut self,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        request: InactiveMinibufferDisplayRowRequest<'_>,
    ) {
        let cols = (request.text_width / request.char_width.max(1.0))
            .ceil()
            .max(1.0) as usize;
        self.matrix_builder.begin_window_with_text_bounds(
            request.window_id,
            1,
            cols,
            request.window_bounds,
            request.text_bounds,
            request.selected,
        );
        let row_request = DisplayRowSourceRequestPolicy::new(
            request.window_bounds.y,
            request.text_width,
            request.row_height,
            request.char_width,
            request.ascent,
            DisplayTabPolicy::every(8),
            GlyphRowRole::Minibuffer,
        );
        let render_request = DisplayRowLispStringRenderRequest::from_base_face_policy(
            row_request,
            face_ids,
            request.base_face,
            Value::string(""),
        );
        let mut render_context =
            DisplayRowRenderContext::new(face_resolver, display_host, face_ids);
        let mut renderer = DisplayRowRenderer::new(&mut self.font_metrics);
        let rendered = render_request
            .render_with_context(&mut renderer, &mut render_context)
            .expect("empty Lisp string should render an inactive minibuffer row");
        install_rendered_display_row(&mut self.matrix_builder, &rendered, 0);
        self.matrix_builder.end_window();
    }

    pub(crate) fn render_echo_minibuffer_window(
        &mut self,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        request: EchoMinibufferDisplayRowsRequest<'_>,
    ) {
        let rows = self.render_minibuffer_echo_rows(
            request.window_bounds.y,
            request.text_width,
            request.char_width,
            request.ascent,
            request.row_height,
            request.base_face,
            face_resolver,
            display_host,
            request.message,
            request.max_rows,
            request.truncate_lines,
            request.reserve_right_special_col,
            face_ids,
        );
        let max_rows = rows.len().clamp(1, request.max_rows.max(1));
        let cols = (request.text_width / request.char_width.max(1.0))
            .ceil()
            .max(1.0) as usize;
        self.matrix_builder.begin_window_with_text_bounds(
            request.window_id,
            max_rows,
            cols,
            request.window_bounds,
            request.text_bounds,
            request.selected,
        );
        for (row_index, rendered) in rows.iter().enumerate() {
            install_rendered_display_row(&mut self.matrix_builder, rendered, row_index);
        }
        self.matrix_builder.end_window();
    }

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
        default_resolved: &ResolvedFace,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        echo_message: Value,
        max_rows: usize,
        truncate_lines: bool,
        reserve_right_special_col: bool,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> Vec<RenderedDisplayRow> {
        let base_face = default_resolved.clone();
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
        let session_request = DisplayRowLispStringSourceSessionRequest::from_base_face(
            echo_message,
            face_ids,
            &base_face,
        );
        let Some(mut source_session) = DisplayRowLispStringSourceSession::new(session_request)
        else {
            return empty_minibuffer_echo_row(y, ascent, row_height);
        };
        let mut renderer = DisplayRowRenderer::new(&mut self.font_metrics);
        let mut render_context =
            DisplayRowRenderContext::new(face_resolver, display_host, face_ids);

        let mut rows = Vec::new();
        let max_rows = max_rows.max(1);
        while rows.len() < max_rows {
            let request = source_session.row_request(
                DisplayRowSourceRequestPolicy::new(
                    y + rows.len() as f32 * row_height,
                    wrap_width,
                    row_height,
                    char_w,
                    ascent,
                    DisplayTabPolicy::every(8),
                    GlyphRowRole::Minibuffer,
                ),
                &base_face,
            );
            let Some(result) = source_session.render_next_row_with_context(
                &mut renderer,
                request,
                &mut render_context,
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
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
