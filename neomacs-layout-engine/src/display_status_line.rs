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
use crate::display_item::{
    DisplayItem, DisplayItemKind, DisplayTextRun, RenderFaceRef, SourceSpan,
};
use crate::display_row::{
    DisplayRowBoundsPolicy, DisplayRowLispStringRenderRequest, DisplayRowLispStringSourceSession,
    DisplayRowLispStringSourceSessionRequest, DisplayRowLispStringSourceSessionRowRequest,
    DisplayRowOwner, DisplayRowRenderExecutor, DisplayRowRenderStop, DisplayRowSourceRequestPolicy,
    FrameChromeKind, MeasuredDisplayRow, RenderedDisplayRow, WindowChromeKind,
    install_measured_frame_chrome_row, install_measured_window_display_row,
    install_rendered_display_row,
};
pub(crate) use crate::display_row::{
    DisplayRowFace, DisplayRowFaceRealizer, DisplayRowOutputProgress,
};
use crate::display_row_builder::{
    DisplayRowLayout, DisplayRowWriter, DisplayTabPolicy, display_row_text_glyph_count,
    display_row_text_is_empty, new_display_row,
};
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::types::WindowParams;
#[cfg(test)]
use neomacs_display_protocol::face::BoxType;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::GlyphRow;
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
    let row = new_display_row(&DisplayRowLayout {
        role: GlyphRowRole::Minibuffer,
        y_px: y,
        width_px: 1.0,
        height_px: row_height.max(1.0),
        ascent_px: ascent.max(0.0).min(row_height.max(1.0)),
        char_width_px: 1.0,
        tab_policy: DisplayTabPolicy::every(8),
        base_face: RenderFaceRef::FaceId(0),
        symbol_values: std::collections::HashMap::new(),
    });
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

fn append_synthetic_minibuffer_text(
    row: &mut GlyphRow,
    text: impl Into<String>,
    face_id: u32,
    y: f32,
    width: f32,
    char_width: f32,
    ascent: f32,
    row_height: f32,
    source_offset: usize,
) {
    let text = text.into();
    let char_len = text.chars().count();
    if char_len == 0 {
        return;
    }
    let layout = DisplayRowLayout {
        role: GlyphRowRole::Minibuffer,
        y_px: y,
        width_px: width.max(1.0),
        height_px: row_height.max(1.0),
        ascent_px: ascent.max(0.0).min(row_height.max(1.0)),
        char_width_px: char_width.max(1.0),
        tab_policy: DisplayTabPolicy::every(8),
        base_face: RenderFaceRef::FaceId(face_id),
        symbol_values: std::collections::HashMap::new(),
    };
    let item = DisplayItem::new(
        SourceSpan::synthetic(0, source_offset, source_offset + char_len),
        RenderFaceRef::FaceId(face_id),
        DisplayItemKind::TextRun(DisplayTextRun::new(text)),
    );
    DisplayRowWriter::new(&layout, row).push_item(item);
}

pub(crate) enum FrameTabBarDisplayRowRender {
    Empty,
    Measured(MeasuredDisplayRow),
}

pub(crate) struct FrameTabBarDisplayRowRequest<'face> {
    pub(crate) row_index: u32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) row_height: f32,
    pub(crate) base_face: &'face ResolvedFace,
    pub(crate) text: Value,
}

impl<'face> FrameTabBarDisplayRowRequest<'face> {
    fn lisp_string_row_request(&self) -> ChromeLispStringRowRequest<'face> {
        ChromeLispStringRowRequest::new(
            self.y,
            self.width,
            self.row_height,
            self.char_width,
            self.ascent,
            DisplayTabPolicy::every(8),
            GlyphRowRole::TabBar,
            self.base_face,
            self.text,
        )
    }

    fn render_request(
        &self,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayRowLispStringRenderRequest<'face> {
        self.lisp_string_row_request().render_request(face_ids)
    }

    fn bounds(&self) -> Rect {
        Rect::new(0.0, self.y, self.width, self.height)
    }
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

struct ChromeLispStringRowRequest<'face> {
    y: f32,
    width: f32,
    row_height: f32,
    char_width: f32,
    ascent: f32,
    tab_policy: DisplayTabPolicy,
    role: GlyphRowRole,
    base_face: &'face ResolvedFace,
    text: Value,
    symbol_values: std::collections::HashMap<String, Value>,
}

impl<'face> ChromeLispStringRowRequest<'face> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        y: f32,
        width: f32,
        row_height: f32,
        char_width: f32,
        ascent: f32,
        tab_policy: DisplayTabPolicy,
        role: GlyphRowRole,
        base_face: &'face ResolvedFace,
        text: Value,
    ) -> Self {
        Self {
            y,
            width,
            row_height,
            char_width,
            ascent,
            tab_policy,
            role,
            base_face,
            text,
            symbol_values: std::collections::HashMap::new(),
        }
    }

    fn with_symbol_values(
        mut self,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Self {
        self.symbol_values = symbol_values;
        self
    }

    fn into_render_request_parts(
        self,
    ) -> (DisplayRowSourceRequestPolicy, &'face ResolvedFace, Value) {
        let Self {
            y,
            width,
            row_height,
            char_width,
            ascent,
            tab_policy,
            role,
            base_face,
            text,
            symbol_values,
        } = self;
        let policy = DisplayRowSourceRequestPolicy::new(
            y, width, row_height, char_width, ascent, tab_policy, role,
        )
        .with_symbol_values(symbol_values);
        (policy, base_face, text)
    }

    #[cfg(test)]
    fn into_source_request_policy(self) -> DisplayRowSourceRequestPolicy {
        self.into_render_request_parts().0
    }

    fn render_request(
        self,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayRowLispStringRenderRequest<'face> {
        let (row_request, base_face, text) = self.into_render_request_parts();
        DisplayRowLispStringRenderRequest::from_base_face_policy(
            row_request,
            face_ids,
            base_face,
            text,
        )
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

pub(crate) struct WindowChromeRowsRenderRequest<'face, 'params> {
    pub(crate) params: &'params WindowParams,
    pub(crate) tab_line_face: Option<&'face ResolvedFace>,
    pub(crate) header_line_face: Option<&'face ResolvedFace>,
    pub(crate) mode_line_face: Option<&'face ResolvedFace>,
    pub(crate) tab_line_height: f32,
    pub(crate) header_line_height: f32,
    pub(crate) mode_line_height: f32,
    pub(crate) mode_line_matrix_row: usize,
    pub(crate) reserve_right_border_col: bool,
    pub(crate) char_width: f32,
    pub(crate) font_ascent: f32,
    pub(crate) buffer_name: &'params str,
}

impl<'face, 'params> WindowChromeRowsRenderRequest<'face, 'params> {
    fn target_cols(&self) -> usize {
        window_chrome_target_cols(
            self.params.bounds.width,
            self.char_width,
            self.reserve_right_border_col,
        )
    }
}

struct WindowChromeDisplayRowRenderParts<'face> {
    output: ChromeRowOutput,
    owner: DisplayRowOwner,
    matrix_row: u32,
    bounds: Rect,
    render_request: DisplayRowLispStringRenderRequest<'face>,
}

impl<'face> WindowChromeDisplayRowRequest<'face> {
    fn lisp_string_row_request(&self) -> ChromeLispStringRowRequest<'face> {
        ChromeLispStringRowRequest::new(
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
            self.char_width,
            self.ascent,
            self.tab_policy.clone(),
            window_chrome_glyph_row_role(self.kind),
            self.base_face,
            self.text.value(),
        )
    }

    fn into_render_parts(
        self,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> WindowChromeDisplayRowRenderParts<'face> {
        let render_request = self
            .lisp_string_row_request()
            .with_symbol_values(self.symbol_values)
            .render_request(face_ids);
        WindowChromeDisplayRowRenderParts {
            output: self.output,
            owner: DisplayRowOwner::WindowChrome {
                window_id: self.window_id,
                kind: self.kind,
            },
            matrix_row: self.matrix_row.min(u32::MAX as usize) as u32,
            bounds: self.bounds,
            render_request,
        }
    }
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

impl<'face> InactiveMinibufferDisplayRowRequest<'face> {
    fn lisp_string_row_request(&self) -> ChromeLispStringRowRequest<'face> {
        ChromeLispStringRowRequest::new(
            self.window_bounds.y,
            self.text_width,
            self.row_height,
            self.char_width,
            self.ascent,
            DisplayTabPolicy::every(8),
            GlyphRowRole::Minibuffer,
            self.base_face,
            Value::string(""),
        )
    }

    fn render_request(
        &self,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayRowLispStringRenderRequest<'face> {
        self.lisp_string_row_request().render_request(face_ids)
    }
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

pub(crate) struct EchoMinibufferRowsRenderRequest<'face> {
    pub(crate) y: f32,
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

struct EchoMinibufferDisplayRowsRenderParts<'face> {
    window_id: u64,
    window_bounds: Rect,
    text_bounds: Rect,
    selected: bool,
    text_width: f32,
    char_width: f32,
    max_rows: usize,
    rows_request: EchoMinibufferRowsRenderRequest<'face>,
}

impl<'face> EchoMinibufferDisplayRowsRequest<'face> {
    fn into_render_parts(self) -> EchoMinibufferDisplayRowsRenderParts<'face> {
        EchoMinibufferDisplayRowsRenderParts {
            window_id: self.window_id,
            window_bounds: self.window_bounds,
            text_bounds: self.text_bounds,
            selected: self.selected,
            text_width: self.text_width,
            char_width: self.char_width,
            max_rows: self.max_rows,
            rows_request: EchoMinibufferRowsRenderRequest {
                y: self.window_bounds.y,
                text_width: self.text_width,
                char_width: self.char_width,
                ascent: self.ascent,
                row_height: self.row_height,
                base_face: self.base_face,
                message: self.message,
                max_rows: self.max_rows,
                truncate_lines: self.truncate_lines,
                reserve_right_special_col: self.reserve_right_special_col,
            },
        }
    }
}

impl<'face> EchoMinibufferRowsRenderRequest<'face> {
    fn max_rows(&self) -> usize {
        self.max_rows.max(1)
    }

    fn reserve_width(&self, char_width: f32) -> f32 {
        if self.reserve_right_special_col {
            char_width.max(1.0)
        } else {
            0.0
        }
    }

    fn wrap_width(&self, char_width: f32) -> f32 {
        if self.truncate_lines {
            self.text_width
        } else {
            (self.text_width - self.reserve_width(char_width)).max(char_width.max(1.0))
        }
    }

    fn matrix_cols(&self) -> usize {
        (self.text_width / self.char_width.max(1.0)).ceil().max(1.0) as usize
    }

    fn source_row_request(
        &self,
        row_index: usize,
        wrap_width: f32,
    ) -> EchoMinibufferSourceRowRequest<'face> {
        EchoMinibufferSourceRowRequest::new(
            row_index,
            self.y,
            wrap_width,
            self.row_height,
            self.char_width,
            self.ascent,
            self.base_face,
        )
    }
}

struct EchoMinibufferSourceRowRequest<'face> {
    row_index: usize,
    y: f32,
    wrap_width: f32,
    row_height: f32,
    char_width: f32,
    ascent: f32,
    base_face: &'face ResolvedFace,
}

impl<'face> EchoMinibufferSourceRowRequest<'face> {
    fn new(
        row_index: usize,
        y: f32,
        wrap_width: f32,
        row_height: f32,
        char_width: f32,
        ascent: f32,
        base_face: &'face ResolvedFace,
    ) -> Self {
        Self {
            row_index,
            y,
            wrap_width,
            row_height,
            char_width,
            ascent,
            base_face,
        }
    }

    fn source_request_policy(&self) -> DisplayRowSourceRequestPolicy {
        DisplayRowSourceRequestPolicy::new(
            self.y + self.row_index as f32 * self.row_height,
            self.wrap_width,
            self.row_height,
            self.char_width,
            self.ascent,
            DisplayTabPolicy::every(8),
            GlyphRowRole::Minibuffer,
        )
    }

    fn source_session_row_request(
        self,
        source_session: &DisplayRowLispStringSourceSession,
    ) -> DisplayRowLispStringSourceSessionRowRequest<'face> {
        source_session.row_request(self.source_request_policy(), self.base_face)
    }
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

fn window_chrome_target_cols(width: f32, char_width: f32, reserve_right_border_col: bool) -> usize {
    ((width / char_width.max(1.0)).round().max(1.0) as usize)
        .saturating_sub(usize::from(reserve_right_border_col))
        .max(1)
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
        let parts = request.into_render_parts(face_ids);
        let mut builder = std::mem::replace(&mut self.matrix_builder, GlyphMatrixBuilder::new());
        output_emitter.begin_chrome_progress(evaluator, parts.output);
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut self.font_metrics,
            face_resolver,
            evaluator.display_host.as_deref(),
            face_ids,
        );
        let rendered_row = render_executor.render_lisp_string_request(parts.render_request);
        let measured_row = rendered_row.map(|rendered| {
            MeasuredDisplayRow::new(
                parts.owner,
                parts.matrix_row,
                parts.bounds,
                rendered,
                DisplayRowBoundsPolicy::PreserveAllocatedMinimum,
            )
        });
        if let Some(ref measured_row) = measured_row {
            install_measured_window_display_row(&mut builder, measured_row);
            output_emitter.emit_chrome_progress(
                evaluator,
                parts.output,
                measured_row.output_progress(),
            );
        }
        self.matrix_builder = builder;
        if let Some(ref measured_row) = measured_row {
            output_emitter.finish_chrome_progress(measured_row.output_progress());
        }
        measured_row
    }

    pub(crate) fn render_window_chrome_display_rows(
        &mut self,
        evaluator: &mut Context,
        output_emitter: &mut WindowOutputEmitter,
        face_resolver: &FaceResolver,
        face_ids: &mut FrameFaceIdAllocator,
        request: WindowChromeRowsRenderRequest<'_, '_>,
    ) {
        let params = request.params;
        let mut status_line_symbol_values = std::collections::HashMap::new();
        if let Some(buffer) = evaluator.buffer_manager().get(BufferId(params.buffer_id))
            && let Some(value) = buffer.buffer_local_value("header-line-indent-width")
        {
            status_line_symbol_values.insert("header-line-indent-width".to_string(), value);
        }
        let chrome_tab_policy = DisplayTabPolicy::from_tab_width_and_stops(
            0.0,
            params.tab_width,
            &params.tab_stop_list,
        );
        let target_cols = request.target_cols();

        if params.tab_line_height > 0.0 {
            let tab_line_y = params.bounds.y;
            let tab_line_text = eval_status_line_format_value(
                evaluator,
                "tab-line-format",
                params.window_id,
                params.buffer_id,
                target_cols,
            )
            .unwrap_or_else(|| Value::string(""));
            self.render_window_chrome_display_row(
                evaluator,
                output_emitter,
                face_resolver,
                face_ids,
                WindowChromeDisplayRowRequest {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::TabLine,
                    matrix_row: 0,
                    output: ChromeRowOutput {
                        row: 0,
                        y: tab_line_y,
                    },
                    bounds: Rect::new(
                        params.bounds.x,
                        tab_line_y,
                        params.bounds.width,
                        request.tab_line_height,
                    ),
                    char_width: request.char_width,
                    ascent: request.font_ascent,
                    tab_policy: chrome_tab_policy.clone(),
                    base_face: request
                        .tab_line_face
                        .expect("tab-line face should exist when tab-line height is positive"),
                    symbol_values: status_line_symbol_values.clone(),
                    text: WindowChromeDisplayText::new(tab_line_text, params.selected),
                },
            );
        }

        if params.header_line_height > 0.0 {
            let header_line_y = params.bounds.y + request.tab_line_height;
            let header_line_text = eval_status_line_format_value(
                evaluator,
                "header-line-format",
                params.window_id,
                params.buffer_id,
                target_cols,
            )
            .unwrap_or_else(|| Value::string(""));
            self.render_window_chrome_display_row(
                evaluator,
                output_emitter,
                face_resolver,
                face_ids,
                WindowChromeDisplayRowRequest {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::HeaderLine,
                    matrix_row: usize::from(request.tab_line_height > 0.0),
                    output: ChromeRowOutput {
                        row: i64::from(request.tab_line_height > 0.0),
                        y: header_line_y,
                    },
                    bounds: Rect::new(
                        params.bounds.x,
                        header_line_y,
                        params.bounds.width,
                        request.header_line_height,
                    ),
                    char_width: request.char_width,
                    ascent: request.font_ascent,
                    tab_policy: chrome_tab_policy.clone(),
                    base_face: request.header_line_face.expect(
                        "header-line face should exist when header-line height is positive",
                    ),
                    symbol_values: status_line_symbol_values.clone(),
                    text: WindowChromeDisplayText::new(header_line_text, params.selected),
                },
            );
        }

        if params.mode_line_height > 0.0 {
            let mode_line_y = params.bounds.y + params.bounds.height - request.mode_line_height;
            let mode_line_text = {
                let result = eval_status_line_format_value(
                    evaluator,
                    "mode-line-format",
                    params.window_id,
                    params.buffer_id,
                    target_cols,
                )
                .unwrap_or_else(|| Value::string(format!(" {} ", request.buffer_name)));
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
            self.render_window_chrome_display_row(
                evaluator,
                output_emitter,
                face_resolver,
                face_ids,
                WindowChromeDisplayRowRequest {
                    window_id: params.window_id as u64,
                    kind: WindowChromeKind::ModeLine,
                    matrix_row: request.mode_line_matrix_row,
                    output: ChromeRowOutput {
                        row: request.mode_line_matrix_row as i64,
                        y: mode_line_y,
                    },
                    bounds: Rect::new(
                        params.bounds.x,
                        mode_line_y,
                        params.bounds.width,
                        request.mode_line_height,
                    ),
                    char_width: request.char_width,
                    ascent: request.font_ascent,
                    tab_policy: chrome_tab_policy,
                    base_face: request
                        .mode_line_face
                        .expect("mode-line face should exist when mode-line height is positive"),
                    symbol_values: status_line_symbol_values,
                    text: WindowChromeDisplayText::new(mode_line_text, params.selected),
                },
            );
        }
    }

    pub(crate) fn render_frame_tab_bar_display_row(
        &mut self,
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        request: FrameTabBarDisplayRowRequest<'_>,
    ) -> Option<FrameTabBarDisplayRowRender> {
        let render_request = request.render_request(face_ids);
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut self.font_metrics,
            face_resolver,
            display_host,
            face_ids,
        );
        let rendered = render_executor.render_lisp_string_request(render_request)?;
        if display_row_text_is_empty(&rendered.row) {
            return Some(FrameTabBarDisplayRowRender::Empty);
        }
        let measured = MeasuredDisplayRow::new(
            DisplayRowOwner::FrameChrome {
                kind: FrameChromeKind::TabBar,
            },
            request.row_index,
            request.bounds(),
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
        let render_request = request.render_request(face_ids);
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut self.font_metrics,
            face_resolver,
            display_host,
            face_ids,
        );
        let rendered = render_executor
            .render_lisp_string_request(render_request)
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
        let parts = request.into_render_parts();
        let rows = self.render_minibuffer_echo_rows(
            face_resolver,
            display_host,
            face_ids,
            parts.rows_request,
        );
        let max_rows = rows.len().clamp(1, parts.max_rows.max(1));
        let cols = (parts.text_width / parts.char_width.max(1.0))
            .ceil()
            .max(1.0) as usize;
        self.matrix_builder.begin_window_with_text_bounds(
            parts.window_id,
            max_rows,
            cols,
            parts.window_bounds,
            parts.text_bounds,
            parts.selected,
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
        face_resolver: &FaceResolver,
        display_host: Option<&dyn DisplayHost>,
        face_ids: &mut FrameFaceIdAllocator,
        request: EchoMinibufferRowsRenderRequest<'_>,
    ) -> Vec<RenderedDisplayRow> {
        let base_face = request.base_face.clone();
        let row_face = self.realize_display_row_face(
            0,
            &base_face,
            request.char_width,
            request.ascent,
            request.row_height,
        );
        let base_render_face = row_face.render_face();
        let char_width = self.display_row_char_width(&row_face, request.char_width);
        let wrap_width = request.wrap_width(char_width);
        let matrix_cols = request.matrix_cols();
        let special_col = matrix_cols.saturating_sub(1);
        let session_request = DisplayRowLispStringSourceSessionRequest::from_base_face(
            request.message,
            face_ids,
            &base_face,
        );
        let Some(mut source_session) = DisplayRowLispStringSourceSession::new(session_request)
        else {
            return empty_minibuffer_echo_row(request.y, request.ascent, request.row_height);
        };
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut self.font_metrics,
            face_resolver,
            display_host,
            face_ids,
        );

        let mut rows = Vec::new();
        let max_rows = request.max_rows();
        while rows.len() < max_rows {
            let row_request = request
                .source_row_request(rows.len(), wrap_width)
                .source_session_row_request(&source_session);
            let Some(result) =
                render_executor.render_lisp_string_session_row(&mut source_session, row_request)
            else {
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
            if request.reserve_right_special_col && stop == DisplayRowRenderStop::Clipped {
                let ch = if request.truncate_lines { '$' } else { '\\' };
                let current_cols = display_row_text_glyph_count(&rendered.row);
                if current_cols < special_col {
                    append_synthetic_minibuffer_text(
                        &mut rendered.row,
                        " ".repeat(special_col - current_cols),
                        special_face_id,
                        rendered.progress.y,
                        request.text_width,
                        char_width,
                        request.ascent,
                        request.row_height,
                        current_cols,
                    );
                }
                append_synthetic_minibuffer_text(
                    &mut rendered.row,
                    ch.to_string(),
                    special_face_id,
                    rendered.progress.y,
                    request.text_width,
                    char_width,
                    request.ascent,
                    request.row_height,
                    special_col,
                );
                rendered.progress.end_x = request.text_width.max(0.0);
                rendered.progress.end_col = matrix_cols as i64;
            }
            rows.push(rendered);
            match stop {
                DisplayRowRenderStop::SourceExhausted => break,
                DisplayRowRenderStop::RowBreak => {}
                DisplayRowRenderStop::Clipped => {
                    if request.truncate_lines {
                        break;
                    }
                }
            }
        }
        if rows.is_empty() {
            return empty_minibuffer_echo_row(request.y, request.ascent, request.row_height);
        }
        rows
    }
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
