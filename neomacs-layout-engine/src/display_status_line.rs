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
pub(crate) use crate::display_row::{DisplayRowFaceRealizer, DisplayRowOutputProgress};
use crate::display_row_builder::{
    DisplayRowLayout, DisplayRowWriter, DisplayTabPolicy, display_row_text_glyph_count,
    display_row_text_is_empty, new_display_row,
};
use crate::font_metrics::FontMetricsService;
use crate::matrix_builder::GlyphMatrixBuilder;
use crate::types::{FrameParams, WindowParams};
#[cfg(test)]
use neomacs_display_protocol::face::BoxType;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphRow};
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

pub(crate) struct ChromeRowRenderServices<'emit, 'face> {
    font_metrics: &'emit mut Option<FontMetricsService>,
    face_resolver: &'face FaceResolver,
    face_ids: &'emit mut FrameFaceIdAllocator,
}

impl<'emit, 'face> ChromeRowRenderServices<'emit, 'face> {
    pub(crate) fn new(
        font_metrics: &'emit mut Option<FontMetricsService>,
        face_resolver: &'face FaceResolver,
        face_ids: &'emit mut FrameFaceIdAllocator,
    ) -> Self {
        Self {
            font_metrics,
            face_resolver,
            face_ids,
        }
    }

    fn face_ids(&mut self) -> &mut FrameFaceIdAllocator {
        self.face_ids
    }

    fn face_realizer(&mut self) -> DisplayRowFaceRealizer<'_> {
        DisplayRowFaceRealizer::new(&mut *self.font_metrics)
    }

    fn render_lisp_string_request(
        &mut self,
        request: DisplayRowLispStringRenderRequest<'_>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<RenderedDisplayRow> {
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut *self.font_metrics,
            self.face_resolver,
            display_host,
            &mut *self.face_ids,
        );
        render_executor.render_lisp_string_request(request)
    }

    fn render_lisp_string_session_row(
        &mut self,
        session: &mut DisplayRowLispStringSourceSession,
        request: DisplayRowLispStringSourceSessionRowRequest<'_>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<crate::display_row::DisplayRowRenderResult> {
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut *self.font_metrics,
            self.face_resolver,
            display_host,
            &mut *self.face_ids,
        );
        render_executor.render_lisp_string_session_row(session, request)
    }
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

    pub(crate) fn render(
        self,
        state: &mut FrameTabBarDisplayRowRenderState<'_, '_>,
    ) -> Option<FrameTabBarDisplayRowRender> {
        let render_request = self.render_request(state.render_services.face_ids());
        let rendered = state
            .render_services
            .render_lisp_string_request(render_request, state.display_host)?;
        if display_row_text_is_empty(&rendered.row) {
            return Some(FrameTabBarDisplayRowRender::Empty);
        }
        let measured = MeasuredDisplayRow::new(
            DisplayRowOwner::FrameChrome {
                kind: FrameChromeKind::TabBar,
            },
            self.row_index,
            self.bounds(),
            rendered,
            DisplayRowBoundsPolicy::MeasureContent,
        );
        install_measured_frame_chrome_row(
            &mut *state.builder,
            &mut *state.pending_frame_chrome_rows,
            &measured,
        );
        Some(FrameTabBarDisplayRowRender::Measured(measured))
    }
}

pub(crate) struct FrameTabBarDisplayRowRenderState<'emit, 'face> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) pending_frame_chrome_rows: &'emit mut Vec<FrameChromeRow>,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
    pub(crate) display_host: Option<&'emit dyn DisplayHost>,
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

pub(crate) struct WindowChromeRowsPlan {
    tab_line_face: Option<ResolvedFace>,
    header_line_face: Option<ResolvedFace>,
    mode_line_face: Option<ResolvedFace>,
    tab_line_height: f32,
    header_line_height: f32,
    mode_line_height: f32,
}

impl WindowChromeRowsPlan {
    pub(crate) fn new(
        params: &WindowParams,
        face_resolver: &FaceResolver,
        font_metrics: &mut Option<FontMetricsService>,
        char_width: f32,
        fallback_ascent: f32,
        fallback_row_height: f32,
    ) -> Self {
        let mode_line_face = (params.mode_line_height > 0.0).then(|| {
            face_resolver.resolve_named_face(if params.selected {
                "mode-line-active"
            } else {
                "mode-line-inactive"
            })
        });
        let header_line_face = (params.header_line_height > 0.0).then(|| {
            face_resolver.resolve_named_face(if params.selected {
                "header-line-active"
            } else {
                "header-line-inactive"
            })
        });
        let tab_line_face =
            (params.tab_line_height > 0.0).then(|| face_resolver.resolve_named_face("tab-line"));

        let mode_line_height = mode_line_face.as_ref().map_or(0.0, |face| {
            window_chrome_row_height_for_face(
                font_metrics,
                face,
                char_width,
                fallback_ascent,
                fallback_row_height,
            )
        });
        let header_line_height = header_line_face.as_ref().map_or(0.0, |face| {
            window_chrome_row_height_for_face(
                font_metrics,
                face,
                char_width,
                fallback_ascent,
                fallback_row_height,
            )
        });
        let tab_line_height = tab_line_face.as_ref().map_or(0.0, |face| {
            window_chrome_row_height_for_face(
                font_metrics,
                face,
                char_width,
                fallback_ascent,
                fallback_row_height,
            )
        });

        Self {
            tab_line_face,
            header_line_face,
            mode_line_face,
            tab_line_height,
            header_line_height,
            mode_line_height,
        }
    }

    pub(crate) fn mode_line_height(&self) -> f32 {
        self.mode_line_height
    }

    pub(crate) fn header_line_height(&self) -> f32 {
        self.header_line_height
    }

    pub(crate) fn tab_line_height(&self) -> f32 {
        self.tab_line_height
    }

    pub(crate) fn render_request<'face, 'params>(
        &'face self,
        params: &'params WindowParams,
        mode_line_matrix_row: usize,
        reserve_right_border_col: bool,
        char_width: f32,
        font_ascent: f32,
        buffer_name: &'params str,
    ) -> WindowChromeRowsRenderRequest<'face, 'params> {
        WindowChromeRowsRenderRequest {
            params,
            tab_line_face: self.tab_line_face.as_ref(),
            header_line_face: self.header_line_face.as_ref(),
            mode_line_face: self.mode_line_face.as_ref(),
            tab_line_height: self.tab_line_height,
            header_line_height: self.header_line_height,
            mode_line_height: self.mode_line_height,
            mode_line_matrix_row,
            reserve_right_border_col,
            char_width,
            font_ascent,
            buffer_name,
        }
    }
}

impl<'face, 'params> WindowChromeRowsRenderRequest<'face, 'params> {
    fn target_cols(&self) -> usize {
        window_chrome_target_cols(
            self.params.bounds.width,
            self.char_width,
            self.reserve_right_border_col,
        )
    }

    pub(crate) fn render(self, state: &mut WindowChromeRowsRenderState<'_, '_>) {
        let params = self.params;
        let mut status_line_symbol_values = std::collections::HashMap::new();
        if let Some(buffer) = state
            .evaluator
            .buffer_manager()
            .get(BufferId(params.buffer_id))
            && let Some(value) = buffer.buffer_local_value("header-line-indent-width")
        {
            status_line_symbol_values.insert("header-line-indent-width".to_string(), value);
        }
        let chrome_tab_policy = DisplayTabPolicy::from_tab_width_and_stops(
            0.0,
            params.tab_width,
            &params.tab_stop_list,
        );
        let target_cols = self.target_cols();

        if params.tab_line_height > 0.0 {
            let tab_line_y = params.bounds.y;
            let tab_line_text = eval_status_line_format_value(
                state.evaluator,
                "tab-line-format",
                params.window_id,
                params.buffer_id,
                target_cols,
            )
            .unwrap_or_else(|| Value::string(""));
            state.render_display_row(WindowChromeDisplayRowRequest {
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
                    self.tab_line_height,
                ),
                char_width: self.char_width,
                ascent: self.font_ascent,
                tab_policy: chrome_tab_policy.clone(),
                base_face: self
                    .tab_line_face
                    .expect("tab-line face should exist when tab-line height is positive"),
                symbol_values: status_line_symbol_values.clone(),
                text: WindowChromeDisplayText::new(tab_line_text, params.selected),
            });
        }

        if params.header_line_height > 0.0 {
            let header_line_y = params.bounds.y + self.tab_line_height;
            let header_line_text = eval_status_line_format_value(
                state.evaluator,
                "header-line-format",
                params.window_id,
                params.buffer_id,
                target_cols,
            )
            .unwrap_or_else(|| Value::string(""));
            state.render_display_row(WindowChromeDisplayRowRequest {
                window_id: params.window_id as u64,
                kind: WindowChromeKind::HeaderLine,
                matrix_row: usize::from(self.tab_line_height > 0.0),
                output: ChromeRowOutput {
                    row: i64::from(self.tab_line_height > 0.0),
                    y: header_line_y,
                },
                bounds: Rect::new(
                    params.bounds.x,
                    header_line_y,
                    params.bounds.width,
                    self.header_line_height,
                ),
                char_width: self.char_width,
                ascent: self.font_ascent,
                tab_policy: chrome_tab_policy.clone(),
                base_face: self
                    .header_line_face
                    .expect("header-line face should exist when header-line height is positive"),
                symbol_values: status_line_symbol_values.clone(),
                text: WindowChromeDisplayText::new(header_line_text, params.selected),
            });
        }

        if params.mode_line_height > 0.0 {
            let mode_line_y = params.bounds.y + params.bounds.height - self.mode_line_height;
            let mode_line_text = {
                let result = eval_status_line_format_value(
                    state.evaluator,
                    "mode-line-format",
                    params.window_id,
                    params.buffer_id,
                    target_cols,
                )
                .unwrap_or_else(|| Value::string(format!(" {} ", self.buffer_name)));
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
            state.render_display_row(WindowChromeDisplayRowRequest {
                window_id: params.window_id as u64,
                kind: WindowChromeKind::ModeLine,
                matrix_row: self.mode_line_matrix_row,
                output: ChromeRowOutput {
                    row: self.mode_line_matrix_row as i64,
                    y: mode_line_y,
                },
                bounds: Rect::new(
                    params.bounds.x,
                    mode_line_y,
                    params.bounds.width,
                    self.mode_line_height,
                ),
                char_width: self.char_width,
                ascent: self.font_ascent,
                tab_policy: chrome_tab_policy,
                base_face: self
                    .mode_line_face
                    .expect("mode-line face should exist when mode-line height is positive"),
                symbol_values: status_line_symbol_values,
                text: WindowChromeDisplayText::new(mode_line_text, params.selected),
            });
        }
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

pub(crate) struct WindowChromeRowsRenderState<'emit, 'face> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) evaluator: &'emit mut Context,
    pub(crate) output_emitter: &'emit mut WindowOutputEmitter,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
}

impl WindowChromeRowsRenderState<'_, '_> {
    fn render_display_row(
        &mut self,
        request: WindowChromeDisplayRowRequest<'_>,
    ) -> Option<MeasuredDisplayRow> {
        let parts = request.into_render_parts(self.render_services.face_ids());
        self.output_emitter
            .begin_chrome_progress(self.evaluator, parts.output);
        let rendered_row = self.render_services.render_lisp_string_request(
            parts.render_request,
            self.evaluator.display_host.as_deref(),
        );
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
            install_measured_window_display_row(&mut *self.builder, measured_row);
            self.output_emitter.emit_chrome_progress(
                self.evaluator,
                parts.output,
                measured_row.output_progress(),
            );
            self.output_emitter
                .finish_chrome_progress(measured_row.output_progress());
        }
        measured_row
    }
}

enum MinibufferSpecialRowsRequest<'face> {
    Echo(EchoMinibufferDisplayRowsRequest<'face>),
    Inactive(InactiveMinibufferDisplayRowRequest<'face>),
}

pub(crate) struct MinibufferSpecialRowsPlan<'face> {
    request: MinibufferSpecialRowsRequest<'face>,
}

impl<'face> MinibufferSpecialRowsPlan<'face> {
    pub(crate) fn from_window(
        evaluator: &Context,
        params: &WindowParams,
        frame_params: &FrameParams,
        text_width: f32,
        char_width: f32,
        row_height: f32,
        ascent: f32,
        base_face: &'face ResolvedFace,
    ) -> Option<Self> {
        let active_minibuffer_window =
            evaluator.minibuffer_window_is_active(WindowId(params.window_id as u64));
        let echo_message = minibuffer_echo_message_for_window(
            params.is_minibuffer,
            active_minibuffer_window,
            evaluator.current_message_value(),
        );
        if let Some(message) = echo_message {
            // GNU `display_echo_area_1` displays the current message by
            // temporarily making the echo-area buffer current, calling
            // `resize_mini_window`, then redisplaying the minibuffer window.
            // GNU measures the displayed height, not just literal newlines:
            // a long one-line message grows the echo area when
            // `message-truncate-lines' is nil.
            let reserve_right_special_col =
                !frame_params.window_system && params.right_fringe_width == 0.0;
            let frame_rows = frame_params.height / row_height.max(1.0);
            let max_rows = max_mini_window_lines(evaluator, frame_rows).ceil().max(1.0) as usize;
            return Some(Self {
                request: MinibufferSpecialRowsRequest::Echo(EchoMinibufferDisplayRowsRequest {
                    window_id: params.window_id as u64,
                    window_bounds: params.bounds,
                    text_bounds: params.text_bounds,
                    selected: params.selected,
                    text_width,
                    char_width,
                    ascent,
                    row_height,
                    base_face,
                    message,
                    max_rows,
                    truncate_lines: message_truncate_lines(evaluator),
                    reserve_right_special_col,
                }),
            });
        }

        if params.is_minibuffer && !active_minibuffer_window {
            // GNU `display_echo_area` temporarily displays an echo-area
            // buffer in the minibuffer window. With no current message that
            // buffer is empty; the inactive minibuffer must not redisplay the
            // ordinary buffer attached to the window record.
            return Some(Self {
                request: MinibufferSpecialRowsRequest::Inactive(
                    InactiveMinibufferDisplayRowRequest {
                        window_id: params.window_id as u64,
                        window_bounds: params.bounds,
                        text_bounds: params.text_bounds,
                        selected: params.selected,
                        text_width,
                        row_height,
                        char_width,
                        ascent,
                        base_face,
                    },
                ),
            });
        }

        None
    }

    pub(crate) fn render_window(self, state: &mut MinibufferDisplayRenderState<'_, '_>) {
        match self.request {
            MinibufferSpecialRowsRequest::Echo(request) => request.render_window(state),
            MinibufferSpecialRowsRequest::Inactive(request) => request.render_window(state),
        }
    }
}

struct InactiveMinibufferDisplayRowRequest<'face> {
    window_id: u64,
    window_bounds: Rect,
    text_bounds: Rect,
    selected: bool,
    text_width: f32,
    row_height: f32,
    char_width: f32,
    ascent: f32,
    base_face: &'face ResolvedFace,
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

    pub(crate) fn render_window(self, state: &mut MinibufferDisplayRenderState<'_, '_>) {
        let cols = (self.text_width / self.char_width.max(1.0)).ceil().max(1.0) as usize;
        state.builder.begin_window_with_text_bounds(
            self.window_id,
            1,
            cols,
            self.window_bounds,
            self.text_bounds,
            self.selected,
        );
        let render_request = self.render_request(state.render_services.face_ids());
        let rendered = state
            .render_services
            .render_lisp_string_request(render_request, state.display_host)
            .expect("empty Lisp string should render an inactive minibuffer row");
        install_rendered_display_row(&mut *state.builder, &rendered, 0);
        state.builder.end_window();
    }
}

struct EchoMinibufferDisplayRowsRequest<'face> {
    window_id: u64,
    window_bounds: Rect,
    text_bounds: Rect,
    selected: bool,
    text_width: f32,
    char_width: f32,
    ascent: f32,
    row_height: f32,
    base_face: &'face ResolvedFace,
    message: Value,
    max_rows: usize,
    truncate_lines: bool,
    reserve_right_special_col: bool,
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

    pub(crate) fn render_window(self, state: &mut MinibufferDisplayRenderState<'_, '_>) {
        let parts = self.into_render_parts();
        let rows = parts.rows_request.render_rows(state);
        let max_rows = rows.len().clamp(1, parts.max_rows.max(1));
        let cols = (parts.text_width / parts.char_width.max(1.0))
            .ceil()
            .max(1.0) as usize;
        state.builder.begin_window_with_text_bounds(
            parts.window_id,
            max_rows,
            cols,
            parts.window_bounds,
            parts.text_bounds,
            parts.selected,
        );
        for (row_index, rendered) in rows.iter().enumerate() {
            install_rendered_display_row(&mut *state.builder, rendered, row_index);
        }
        state.builder.end_window();
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

    /// Build minibuffer echo rows through the shared display-source path.
    ///
    /// The returned rows retain their realized faces and progress metadata so
    /// callers can install them through the same path used by chrome rows.
    pub(crate) fn render_rows(
        self,
        state: &mut MinibufferDisplayRenderState<'_, '_>,
    ) -> Vec<RenderedDisplayRow> {
        let base_face = self.base_face.clone();
        let row_face = state.render_services.face_realizer().realize_face(
            0,
            &base_face,
            self.char_width,
            self.ascent,
            self.row_height,
        );
        let base_render_face = row_face.render_face();
        let char_width = state
            .render_services
            .face_realizer()
            .char_width(&row_face, self.char_width);
        let wrap_width = self.wrap_width(char_width);
        let matrix_cols = self.matrix_cols();
        let special_col = matrix_cols.saturating_sub(1);
        let session_request = DisplayRowLispStringSourceSessionRequest::from_base_face(
            self.message,
            state.render_services.face_ids(),
            &base_face,
        );
        let Some(mut source_session) = DisplayRowLispStringSourceSession::new(session_request)
        else {
            return empty_minibuffer_echo_row(self.y, self.ascent, self.row_height);
        };
        let mut rows = Vec::new();
        let max_rows = self.max_rows();
        while rows.len() < max_rows {
            let row_request = self
                .source_row_request(rows.len(), wrap_width)
                .source_session_row_request(&source_session);
            let Some(result) = state.render_services.render_lisp_string_session_row(
                &mut source_session,
                row_request,
                state.display_host,
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
            if self.reserve_right_special_col && stop == DisplayRowRenderStop::Clipped {
                let ch = if self.truncate_lines { '$' } else { '\\' };
                let current_cols = display_row_text_glyph_count(&rendered.row);
                if current_cols < special_col {
                    append_synthetic_minibuffer_text(
                        &mut rendered.row,
                        " ".repeat(special_col - current_cols),
                        special_face_id,
                        rendered.progress.y,
                        self.text_width,
                        char_width,
                        self.ascent,
                        self.row_height,
                        current_cols,
                    );
                }
                append_synthetic_minibuffer_text(
                    &mut rendered.row,
                    ch.to_string(),
                    special_face_id,
                    rendered.progress.y,
                    self.text_width,
                    char_width,
                    self.ascent,
                    self.row_height,
                    special_col,
                );
                rendered.progress.end_x = self.text_width.max(0.0);
                rendered.progress.end_col = matrix_cols as i64;
            }
            rows.push(rendered);
            match stop {
                DisplayRowRenderStop::SourceExhausted => break,
                DisplayRowRenderStop::RowBreak => {}
                DisplayRowRenderStop::Clipped => {
                    if self.truncate_lines {
                        break;
                    }
                }
            }
        }
        if rows.is_empty() {
            return empty_minibuffer_echo_row(self.y, self.ascent, self.row_height);
        }
        rows
    }
}

pub(crate) struct MinibufferDisplayRenderState<'emit, 'face> {
    pub(crate) builder: &'emit mut GlyphMatrixBuilder,
    pub(crate) render_services: ChromeRowRenderServices<'emit, 'face>,
    pub(crate) display_host: Option<&'emit dyn DisplayHost>,
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

pub(crate) fn window_chrome_row_height_for_face(
    font_metrics: &mut Option<FontMetricsService>,
    face: &ResolvedFace,
    char_width: f32,
    fallback_ascent: f32,
    fallback_row_height: f32,
) -> f32 {
    DisplayRowFaceRealizer::new(font_metrics).row_height_for_face(
        face,
        char_width,
        fallback_ascent,
        fallback_row_height,
    )
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
