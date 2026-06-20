//! Display-walker chrome row rendering.
//!
//! Mode-line, header-line, tab-line, tab-bar, and minibuffer echo rows share
//! the face realization helpers defined here. The shared row renderer and
//! property harvester live in `display_row`; this module retains the status-line
//! filename because it grew from the older mode-line-only path.
//!
//! History: this module started as a divergent
//! parallel implementation of display-line rendering that did not
//! process display properties and dropped doom-modeline's
//! (space :align-to ...) forms. Steps 3.3' through 3.6 of the
//! display-engine unification plan merged it into the backend
//! trait and renamed the file to reflect its new role.

use super::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace, buffer_local_value};
use super::window_output::{
    ChromeRowOutput, ChromeRowProgress, DisplayProgressSink, TextWindowOutputTarget,
    WindowOutputEmitter,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_origin::DisplayOrigin;
use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_rendered_row_output_install::install_measured_frame_chrome_display_row;
use crate::display_row::{
    DisplayRowBoundsPolicy, DisplayRowFallbackMetrics, DisplayRowLispStringSourceRenderRequest,
    DisplayRowOwner, DisplayRowRenderExecutor, DisplayRowSourceFragmentRenderRequest,
    DisplayRowSourceRequestPolicy, DisplayRowSourceState, FrameChromeKind, MeasuredDisplayRow,
    RenderedDisplayRow, WindowChromeKind,
};
pub(crate) use crate::display_row::{DisplayRowFaceRealizer, DisplayRowOutputProgress};
use crate::display_row_builder::{DisplayTabPolicy, display_row_text_is_empty};
use crate::display_source::DisplayItemSource;
use crate::font_metrics::FontMetricsService;
use crate::types::WindowParams;
#[cfg(test)]
use neomacs_display_protocol::face::BoxType;
use neomacs_display_protocol::glyph_matrix::{FrameChromeRow, GlyphRow};
use neomacs_display_protocol::types::Rect;
use neomacs_display_protocol::ui_types::TabBarItem;
use neovm_core::buffer::BufferId;
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::emacs_core::keymap::{KeymapMarker, is_list_keymap};
use neovm_core::emacs_core::value::list_to_vec;
use neovm_core::window::WindowId;
use strum::{EnumString, IntoStaticStr};

pub(crate) enum FrameTabBarDisplayRowRender {
    Empty,
    Measured(MeasuredDisplayRow),
}

pub(crate) struct FrameChromeOutputTarget<'a> {
    output_builder: &'a mut DisplayOutputBuilder,
}

impl<'a> FrameChromeOutputTarget<'a> {
    pub(crate) fn from_builder(output_builder: &'a mut DisplayOutputBuilder) -> Self {
        Self { output_builder }
    }

    fn builder(&mut self) -> &mut DisplayOutputBuilder {
        self.output_builder
    }

    fn install_measured_frame_chrome_display_row(
        &mut self,
        pending_frame_chrome_rows: &mut Vec<FrameChromeRow>,
        measured: &MeasuredDisplayRow,
    ) {
        install_measured_frame_chrome_display_row(
            self.builder(),
            pending_frame_chrome_rows,
            measured,
        );
    }
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

    pub(crate) fn reborrow(&mut self) -> ChromeRowRenderServices<'_, 'face> {
        ChromeRowRenderServices {
            font_metrics: self.font_metrics,
            face_resolver: self.face_resolver,
            face_ids: self.face_ids,
        }
    }

    pub(crate) fn face_resolver(&self) -> &'face FaceResolver {
        self.face_resolver
    }

    pub(crate) fn face_ids(&mut self) -> &mut FrameFaceIdAllocator {
        self.face_ids
    }

    fn render_lisp_string_source_request(
        &mut self,
        request: DisplayRowLispStringSourceRenderRequest<'_>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<RenderedDisplayRow> {
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut *self.font_metrics,
            self.face_resolver,
            display_host,
            &mut *self.face_ids,
        );
        render_executor.render_lisp_string_source_request(request)
    }

    pub(crate) fn render_item_source_fragment_into_row(
        &mut self,
        request: DisplayRowSourceFragmentRenderRequest<'_>,
        row: &mut GlyphRow,
        source: &mut impl DisplayItemSource,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<crate::display_row::DisplayRowRenderIntoRowResult> {
        let mut render_executor = DisplayRowRenderExecutor::new(
            &mut *self.font_metrics,
            self.face_resolver,
            None,
            &mut *self.face_ids,
        );
        render_executor.render_item_source_fragment_into_row(request, row, source, source_state)
    }
}

pub(crate) struct FrameTabBarDisplayRowRequest<'face> {
    pub(crate) row_index: u32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) metrics: DisplayRowFallbackMetrics,
    pub(crate) base_face: &'face ResolvedFace,
    pub(crate) text: Value,
}

impl<'face> FrameTabBarDisplayRowRequest<'face> {
    fn lisp_string_row_request(&self) -> ChromeLispStringRowRequest<'face> {
        ChromeLispStringRowRequest::new(
            self.y,
            self.width,
            self.metrics,
            DisplayTabPolicy::every(8),
            DisplayOrigin::TabBar,
            self.base_face,
            self.text,
        )
    }

    fn lisp_string_source_request(
        &self,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> DisplayRowLispStringSourceRenderRequest<'face> {
        self.lisp_string_row_request().render_request(face_ids)
    }

    fn bounds(&self) -> Rect {
        Rect::new(0.0, self.y, self.width, self.height)
    }

    pub(crate) fn render(
        self,
        state: &mut FrameTabBarDisplayRowRenderState<'_, '_, 'face>,
    ) -> Option<FrameTabBarDisplayRowRender> {
        let rendered = self
            .into_chrome_render_request(state.render_services.face_ids())
            .render_row(&mut state.render_services, state.display_host)?;
        if rendered.text_is_empty() {
            return Some(FrameTabBarDisplayRowRender::Empty);
        }
        let measured = rendered.measure();
        state
            .output
            .install_measured_frame_chrome_display_row(state.pending_frame_chrome_rows, &measured);
        Some(FrameTabBarDisplayRowRender::Measured(measured))
    }

    fn into_chrome_render_request(
        self,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> ChromeDisplayRowRenderRequest<'face> {
        ChromeDisplayRowRenderRequest {
            owner: DisplayRowOwner::FrameChrome {
                kind: FrameChromeKind::TabBar,
            },
            display_row_index: self.row_index,
            bounds: self.bounds(),
            bounds_policy: DisplayRowBoundsPolicy::MeasureContent,
            render_request: self.lisp_string_source_request(face_ids),
        }
    }
}

pub(crate) struct FrameTabBarDisplayRowRenderState<'emit, 'output, 'face> {
    output: FrameChromeOutputTarget<'emit>,
    pending_frame_chrome_rows: &'emit mut Vec<FrameChromeRow>,
    render_services: ChromeRowRenderServices<'emit, 'face>,
    display_host: Option<&'emit dyn DisplayHost>,
    _output: std::marker::PhantomData<&'output mut ()>,
}

impl<'emit, 'output, 'face> FrameTabBarDisplayRowRenderState<'emit, 'output, 'face> {
    pub(crate) fn new(
        output: FrameChromeOutputTarget<'emit>,
        pending_frame_chrome_rows: &'emit mut Vec<FrameChromeRow>,
        render_services: ChromeRowRenderServices<'emit, 'face>,
        display_host: Option<&'emit dyn DisplayHost>,
    ) -> Self {
        Self {
            output,
            pending_frame_chrome_rows,
            render_services,
            display_host,
            _output: std::marker::PhantomData,
        }
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
    metrics: DisplayRowFallbackMetrics,
    tab_policy: DisplayTabPolicy,
    origin: DisplayOrigin,
    base_face: &'face ResolvedFace,
    text: Value,
    symbol_values: std::collections::HashMap<String, Value>,
}

impl<'face> ChromeLispStringRowRequest<'face> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        y: f32,
        width: f32,
        metrics: DisplayRowFallbackMetrics,
        tab_policy: DisplayTabPolicy,
        origin: DisplayOrigin,
        base_face: &'face ResolvedFace,
        text: Value,
    ) -> Self {
        Self {
            y,
            width,
            metrics,
            tab_policy,
            origin,
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
            metrics,
            tab_policy,
            origin,
            base_face,
            text,
            symbol_values,
        } = self;
        let policy = DisplayRowSourceRequestPolicy::from_origin(
            y,
            width,
            metrics.row_height(),
            metrics.char_width(),
            metrics.ascent(),
            tab_policy,
            origin,
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
    ) -> DisplayRowLispStringSourceRenderRequest<'face> {
        let (row_request, base_face, text) = self.into_render_request_parts();
        let row_request = row_request.source_request_from_base_face(face_ids, base_face);
        DisplayRowLispStringSourceRenderRequest::from_value(row_request, text)
    }
}

pub(crate) struct WindowChromeDisplayRowRequest<'face> {
    pub(crate) window_id: u64,
    pub(crate) kind: WindowChromeKind,
    pub(crate) selected: bool,
    pub(crate) display_row_index: usize,
    pub(crate) output: ChromeRowOutput,
    pub(crate) bounds: Rect,
    pub(crate) metrics: DisplayRowFallbackMetrics,
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
    pub(crate) mode_line_display_row: usize,
    pub(crate) reserve_right_border_col: bool,
    pub(crate) metrics: DisplayRowFallbackMetrics,
    pub(crate) buffer_name: &'params str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WindowChromeTargetColumns {
    width: f32,
    char_width: f32,
    reserve_right_border_col: bool,
}

impl WindowChromeTargetColumns {
    pub(crate) fn new(width: f32, char_width: f32, reserve_right_border_col: bool) -> Self {
        Self {
            width,
            char_width,
            reserve_right_border_col,
        }
    }

    pub(crate) fn columns(self) -> usize {
        ((self.width / self.char_width.max(1.0)).round().max(1.0) as usize)
            .saturating_sub(usize::from(self.reserve_right_border_col))
            .max(1)
    }
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
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> Self {
        let mode_line_face = (params.mode_line_height > 0.0).then(|| {
            face_resolver.default_base_face_for_origin_without_buffer(&DisplayOrigin::ModeLine {
                selected: params.selected,
            })
        });
        let header_line_face = (params.header_line_height > 0.0).then(|| {
            face_resolver.default_base_face_for_origin_without_buffer(&DisplayOrigin::HeaderLine {
                selected: params.selected,
            })
        });
        let tab_line_face = (params.tab_line_height > 0.0).then(|| {
            face_resolver.default_base_face_for_origin_without_buffer(&DisplayOrigin::TabLine)
        });

        let mode_line_height = mode_line_face.as_ref().map_or(0.0, |face| {
            window_chrome_row_height_for_face(
                font_metrics,
                face,
                fallback_metrics,
            )
        });
        let header_line_height = header_line_face.as_ref().map_or(0.0, |face| {
            window_chrome_row_height_for_face(
                font_metrics,
                face,
                fallback_metrics,
            )
        });
        let tab_line_height = tab_line_face.as_ref().map_or(0.0, |face| {
            window_chrome_row_height_for_face(
                font_metrics,
                face,
                fallback_metrics,
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
        mode_line_display_row: usize,
        reserve_right_border_col: bool,
        metrics: DisplayRowFallbackMetrics,
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
            mode_line_display_row,
            reserve_right_border_col,
            metrics,
            buffer_name,
        }
    }
}

impl<'face, 'params> WindowChromeRowsRenderRequest<'face, 'params> {
    fn target_columns(&self) -> WindowChromeTargetColumns {
        WindowChromeTargetColumns::new(
            self.params.bounds.width,
            self.metrics.char_width(),
            self.reserve_right_border_col,
        )
    }

    fn target_cols(&self) -> usize {
        self.target_columns().columns()
    }

    pub(crate) fn render(self, state: &mut WindowChromeRowsRenderState<'_, '_, 'face>) {
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
                selected: params.selected,
                display_row_index: 0,
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
                metrics: self.metrics,
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
                selected: params.selected,
                display_row_index: usize::from(self.tab_line_height > 0.0),
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
                metrics: self.metrics,
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
                selected: params.selected,
                display_row_index: self.mode_line_display_row,
                output: ChromeRowOutput {
                    row: self.mode_line_display_row as i64,
                    y: mode_line_y,
                },
                bounds: Rect::new(
                    params.bounds.x,
                    mode_line_y,
                    params.bounds.width,
                    self.mode_line_height,
                ),
                metrics: self.metrics,
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

struct ChromeDisplayRowRenderRequest<'face> {
    owner: DisplayRowOwner,
    display_row_index: u32,
    bounds: Rect,
    bounds_policy: DisplayRowBoundsPolicy,
    render_request: DisplayRowLispStringSourceRenderRequest<'face>,
}

struct ChromeDisplayRowRenderedRequest {
    owner: DisplayRowOwner,
    display_row_index: u32,
    bounds: Rect,
    bounds_policy: DisplayRowBoundsPolicy,
    rendered: RenderedDisplayRow,
}

impl<'face> ChromeDisplayRowRenderRequest<'face> {
    fn render_row(
        self,
        render_services: &mut ChromeRowRenderServices<'_, 'face>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<ChromeDisplayRowRenderedRequest> {
        let rendered =
            render_services.render_lisp_string_source_request(self.render_request, display_host)?;
        Some(ChromeDisplayRowRenderedRequest {
            owner: self.owner,
            display_row_index: self.display_row_index,
            bounds: self.bounds,
            rendered,
            bounds_policy: self.bounds_policy,
        })
    }
}

impl ChromeDisplayRowRenderedRequest {
    fn text_is_empty(&self) -> bool {
        display_row_text_is_empty(&self.rendered.row)
    }

    fn measure(self) -> MeasuredDisplayRow {
        MeasuredDisplayRow::new(
            self.owner,
            self.display_row_index,
            self.bounds,
            self.rendered,
            self.bounds_policy,
        )
    }
}

struct WindowChromeDisplayRowRenderRequest<'face> {
    output: ChromeRowOutput,
    row: ChromeDisplayRowRenderRequest<'face>,
}

impl<'face> WindowChromeDisplayRowRenderRequest<'face> {
    fn render_measured(
        self,
        render_services: &mut ChromeRowRenderServices<'_, 'face>,
        display_host: Option<&dyn DisplayHost>,
    ) -> Option<WindowChromeDisplayRowRender> {
        let measured = self
            .row
            .render_row(render_services, display_host)?
            .measure();
        Some(WindowChromeDisplayRowRender {
            output: self.output,
            measured,
        })
    }

    fn render_and_apply(
        self,
        state: &mut WindowChromeRowsRenderState<'_, '_, 'face>,
    ) -> Option<MeasuredDisplayRow> {
        let rendered = self.render_measured(
            &mut state.render_services,
            state.evaluator.display_host.as_deref(),
        )?;
        let progress = rendered.measured.output_progress();
        state
            .output
            .install_measured_window_display_row(&rendered.measured);
        state.output_emitter.emit_chrome_progress(
            state.evaluator,
            ChromeRowProgress::new(rendered.output, progress),
        );
        Some(rendered.measured)
    }
}

struct WindowChromeDisplayRowRender {
    output: ChromeRowOutput,
    measured: MeasuredDisplayRow,
}

impl<'face> WindowChromeDisplayRowRequest<'face> {
    fn lisp_string_row_request(&self) -> ChromeLispStringRowRequest<'face> {
        ChromeLispStringRowRequest::new(
            self.bounds.y,
            self.bounds.width,
            DisplayRowFallbackMetrics::from_default_face_extents(
                self.metrics.char_width(),
                self.bounds.height,
                self.metrics.ascent(),
            ),
            self.tab_policy.clone(),
            window_chrome_display_origin(self.kind, self.selected),
            self.base_face,
            self.text.value(),
        )
    }

    fn into_render_request(
        self,
        face_ids: &mut FrameFaceIdAllocator,
    ) -> WindowChromeDisplayRowRenderRequest<'face> {
        let render_request = self
            .lisp_string_row_request()
            .with_symbol_values(self.symbol_values)
            .render_request(face_ids);
        let row = ChromeDisplayRowRenderRequest {
            owner: DisplayRowOwner::WindowChrome {
                window_id: self.window_id,
                kind: self.kind,
            },
            display_row_index: self.display_row_index.min(u32::MAX as usize) as u32,
            bounds: self.bounds,
            bounds_policy: DisplayRowBoundsPolicy::PreserveAllocatedMinimum,
            render_request,
        };
        WindowChromeDisplayRowRenderRequest {
            output: self.output,
            row,
        }
    }
}

pub(crate) struct WindowChromeRowsRenderState<'state, 'services, 'face> {
    output: TextWindowOutputTarget<'state>,
    output_emitter: &'state mut WindowOutputEmitter,
    evaluator: &'state mut Context,
    render_services: ChromeRowRenderServices<'services, 'face>,
}

impl<'state, 'services, 'face> WindowChromeRowsRenderState<'state, 'services, 'face> {
    pub(crate) fn new(
        output: TextWindowOutputTarget<'state>,
        output_emitter: &'state mut WindowOutputEmitter,
        evaluator: &'state mut Context,
        render_services: ChromeRowRenderServices<'services, 'face>,
    ) -> Self {
        Self {
            output,
            output_emitter,
            evaluator,
            render_services,
        }
    }

    fn render_display_row(
        &mut self,
        request: WindowChromeDisplayRowRequest<'face>,
    ) -> Option<MeasuredDisplayRow> {
        request
            .into_render_request(self.render_services.face_ids())
            .render_and_apply(self)
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

    /// Mirror GNU `resize_mini_window` (src/xdisp.c:13395-13406).
    ///
    /// With `resize-mini-windows` = `grow-only`, the mini-window shrinks back
    /// only when `height < old_height && (exact_p || BEGV == ZV)`
    /// (xdisp.c:13401): i.e. when its buffer is empty, OR when an exact resize
    /// was requested. The exact case is GNU's `resize_echo_area_exactly`
    /// (xdisp.c:13228-13245), which passes `exact_p = (minibuf_level == 0)` and
    /// is run after every command from `command_loop_1` (keyboard.c:1344) — so
    /// a finished command with no active minibuffer shrinks even a NON-EMPTY
    /// shorter message to fit. With `resize-mini-windows` = `t`, always shrink.
    pub(crate) fn should_shrink(self, exact: bool, visible_region_empty: bool) -> bool {
        match self {
            Self::Disabled => false,
            Self::GrowOnly => exact || visible_region_empty,
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
    pub(crate) items: Vec<TabBarItem>,
}

struct TabBarDisplayBuildRequest {
    frame_id: u64,
}

impl TabBarDisplayBuildRequest {
    fn new(frame_id: u64) -> Self {
        Self { frame_id }
    }

    fn build(self, evaluator: &mut Context, gc_roots: &ScratchGcRootScope) -> Option<BuiltTabBar> {
        evaluator.setup_thread_locals();
        if !evaluator.obarray().fboundp("tab-bar-make-keymap-1") {
            return None;
        }

        let restore = TabBarDisplaySelectionRestore::capture(evaluator, gc_roots);
        let result = self
            .select_frame(evaluator)
            .and_then(|()| Self::make_keymap(evaluator))
            .and_then(TabBarDisplaySource::from_keymap)
            .and_then(|source| source.into_built_tab_bar(evaluator));
        if let Some(tab_bar) = &result {
            gc_roots.root(tab_bar.text);
        }
        restore.apply(evaluator);
        result
    }

    fn select_frame(self, evaluator: &mut Context) -> Option<()> {
        evaluator
            .eval_form(Value::list(vec![
                Value::symbol("select-frame"),
                Value::make_frame(self.frame_id),
                Value::NIL,
            ]))
            .ok()
            .map(|_| ())
    }

    fn make_keymap(evaluator: &mut Context) -> Option<Value> {
        evaluator
            .eval_form(Value::list(vec![Value::symbol("tab-bar-make-keymap-1")]))
            .ok()
    }
}

struct TabBarDisplaySelectionRestore {
    frame: Option<Value>,
    window: Option<Value>,
    buffer_id: Option<BufferId>,
}

impl TabBarDisplaySelectionRestore {
    fn capture(evaluator: &mut Context, gc_roots: &ScratchGcRootScope) -> Self {
        let frame = evaluator
            .eval_form(Value::list(vec![Value::symbol("selected-frame")]))
            .ok();
        if let Some(frame) = frame {
            gc_roots.root(frame);
        }
        let window = evaluator
            .eval_form(Value::list(vec![Value::symbol("selected-window")]))
            .ok();
        if let Some(window) = window {
            gc_roots.root(window);
        }
        let buffer_id = evaluator
            .buffer_manager()
            .current_buffer()
            .map(|buffer| buffer.id());
        Self {
            frame,
            window,
            buffer_id,
        }
    }

    fn apply(self, evaluator: &mut Context) {
        if let Some(frame) = self.frame {
            let _ = evaluator.eval_form(Value::list(vec![
                Value::symbol("select-frame"),
                frame,
                Value::NIL,
            ]));
        }
        if let Some(window) = self.window {
            let _ = evaluator.eval_form(Value::list(vec![
                Value::symbol("select-window"),
                window,
                Value::NIL,
            ]));
        }
        if let Some(buffer_id) = self.buffer_id
            && evaluator.buffer_manager().get(buffer_id).is_some()
        {
            evaluator.buffer_manager_mut().set_current(buffer_id);
        }
    }
}

struct TabBarDisplaySource {
    captions: Vec<Value>,
    items: Vec<TabBarItem>,
}

impl TabBarDisplaySource {
    fn from_keymap(keymap: Value) -> Option<Self> {
        let entries = list_to_vec(&keymap)?;
        let mut captions = Vec::new();
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
                captions.push(caption);
                items.push(TabBarItem {
                    index: items.len() as u32,
                    label,
                    help: String::new(),
                    enabled: true,
                    selected: false,
                    is_separator: false,
                });
            }
        }
        (!captions.is_empty()).then_some(Self { captions, items })
    }

    fn into_built_tab_bar(self, evaluator: &mut Context) -> Option<BuiltTabBar> {
        let mut concat_form = Vec::with_capacity(self.captions.len() + 1);
        concat_form.push(Value::symbol("concat"));
        concat_form.extend(self.captions);
        let text = evaluator.eval_form(Value::list(concat_form)).ok()?;
        text.as_runtime_string_owned()
            .is_some_and(|text| !text.is_empty())
            .then_some(BuiltTabBar {
                text,
                items: self.items,
            })
    }
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
    TabBarDisplayBuildRequest::new(frame_id).build(evaluator, gc_roots)
}

pub(crate) fn max_mini_window_lines(evaluator: &Context, frame_rows: f32) -> f32 {
    let raw = evaluator
        .obarray()
        .symbol_value("max-mini-window-height")
        .copied()
        .unwrap_or_else(|| Value::make_float(0.25));
    max_mini_window_lines_from_value(raw, frame_rows)
}

pub(crate) fn max_mini_window_lines_for_buffer<B: LayoutBufferView>(
    evaluator: &Context,
    buffer: &B,
    frame_rows: f32,
) -> f32 {
    let raw = buffer_local_value(buffer, "max-mini-window-height")
        .or_else(|| {
            evaluator
                .obarray()
                .symbol_value("max-mini-window-height")
                .copied()
        })
        .unwrap_or_else(|| Value::make_float(0.25));
    max_mini_window_lines_from_value(raw, frame_rows)
}

pub(crate) fn max_mini_window_lines_from_value(raw: Value, frame_rows: f32) -> f32 {
    match raw.kind() {
        neovm_core::emacs_core::value::ValueKind::Float => {
            (frame_rows * raw.as_float().unwrap_or(0.25) as f32).max(1.0)
        }
        neovm_core::emacs_core::value::ValueKind::Fixnum(_) => raw.as_int().unwrap_or(1) as f32,
        _ => 1.0,
    }
}

fn window_chrome_display_origin(kind: WindowChromeKind, selected: bool) -> DisplayOrigin {
    match kind {
        WindowChromeKind::TabLine => DisplayOrigin::TabLine,
        WindowChromeKind::HeaderLine => DisplayOrigin::HeaderLine { selected },
        WindowChromeKind::ModeLine => DisplayOrigin::ModeLine { selected },
    }
}

pub(crate) fn window_chrome_row_height_for_face(
    font_metrics: &mut Option<FontMetricsService>,
    face: &ResolvedFace,
    fallback_metrics: DisplayRowFallbackMetrics,
) -> f32 {
    DisplayRowFaceRealizer::new(font_metrics).row_height_for_face(
        face,
        fallback_metrics.char_width(),
        fallback_metrics.ascent(),
        fallback_metrics.row_height(),
    )
}

#[cfg(test)]
#[path = "display_status_line_test.rs"]
mod tests;
