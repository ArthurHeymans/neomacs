//! Source-render state facade.
//!
//! This module holds the state types that bridge the typed display-source
//! layer (`DisplayItemSource`) to the row renderer and output builder.  It
//! lives between `display_source.rs` / `display_row.rs` and the append-layer
//! helpers in `display_row_append.rs`, so that the append module does not
//! need to own the render-state facade.

use crate::display_current_row_output::{DisplayCurrentRowMutation, DisplayRowCurrentRowOutput};
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::DisplayPropertyReplacementDescriptor;
use crate::display_mock_frame::protocol_color_to_pixel;
use crate::display_origin::DisplayOrigin;
use crate::display_property::DisplayReplacementProperty;
use crate::display_row::{
    DisplayRowRenderContext, DisplayRowRenderExecutor, DisplayRowRenderer,
    DisplayRowSourceFragmentFrame, DisplayRowSourceRenderRequest,
};
use crate::display_row_append_context::DisplayRowAppendSourceRenderRequest;
use crate::display_row_builder::{DisplayRowGlyphCheckpoint, DisplayRowPosition};
use crate::display_row_face_state::{
    DisplayRowActiveFaceState, DisplayRowMeasurementPolicy, DisplayRowResolvedMeasuredFace,
};
use crate::display_row_geometry::{DisplayRowGeometryState, DisplayRowScopedValue};
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_render_policy::DisplayRowRenderPolicy;
use crate::display_row_render_state::{
    CurrentTextRowRenderOutcome, DisplayRowRenderIntoRowResult, display_row_output_end_position,
};
use crate::display_row_replacement::DisplayPropertyReplacementRowRenderRequest;
use crate::display_row_source_state::DisplayRowSourceState;
use crate::display_row_text_output::TextRowOutput;
use crate::display_row_walk_state::TrailingWhitespaceRenderState;
use crate::display_source::DisplayItemSource;
use crate::display_source_resolver::{
    ActiveDisplayStringBaseFace, DisplayDefaultFaceInstallPolicy, DisplayStringBaseFace,
    resolve_display_string_base_face,
};
use crate::display_spec::DisplayFringeSide;
use crate::display_text_output_install::TextWindowRowDecorationRequest;
use crate::font_metrics::FontMetricsService;
use crate::frame_face_arena::FrameFaceAttempt;
use crate::glyph_row_writer::push_stretch_to_area;
use crate::neovm_bridge::{FaceResolver, LayoutBufferView, ResolvedFace};
use crate::types::WindowParams;
use crate::window_output::{
    DisplayTextRowGeometryTransition, DisplayTextRowTransition, TextWindowOutputTarget,
    WindowOutputEmitter, install_text_window_row_decoration_request,
    transition_text_window_row_with_limit,
};
use neomacs_display_protocol::glyph_matrix::{
    FringeBitmapInfo, Glyph, GlyphArea, GlyphRow, GlyphType, NO_BUFFER_POSITION_CHARPOS,
};
use neomacs_display_protocol::types::Color;
use neomacs_display_protocol::types::FaceId;
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;
use neovm_core::window::DisplayRowSnapshot;

/// Current-row mutation that attaches a resolved fringe bitmap to the row's
/// left or right fringe slot.
struct SetRowFringeBitmapMutation {
    side: DisplayFringeSide,
    info: FringeBitmapInfo,
}

impl DisplayCurrentRowMutation for SetRowFringeBitmapMutation {
    type Output = ();

    fn apply(self, row: &mut GlyphRow) -> Self::Output {
        match self.side {
            DisplayFringeSide::Left => row.left_fringe_bitmap = Some(self.info),
            DisplayFringeSide::Right => row.right_fringe_bitmap = Some(self.info),
        }
    }
}

pub(crate) struct TextRowOutputRenderState<'a> {
    output: TextWindowOutputTarget<'a>,
    output_emitter: &'a mut WindowOutputEmitter,
    evaluator: &'a mut Context,
}

struct DisplayRowCurrentTextSourceState<'face, 'emit> {
    row_output: DisplayRowCurrentRowOutput<'emit>,
    evaluator: &'emit mut Context,
    font_metrics: &'emit mut Option<FontMetricsService>,
    window_system: bool,
    face_resolver: &'face FaceResolver,
    face_ids: &'emit mut FrameFaceAttempt,
}

struct DisplayRowCurrentSourceFragmentRenderState<'face, 'emit> {
    row_output: DisplayRowCurrentRowOutput<'emit>,
    font_metrics: &'emit mut Option<FontMetricsService>,
    window_system: bool,
    face_resolver: &'face FaceResolver,
    display_host: Option<&'emit dyn DisplayHost>,
    face_ids: &'emit mut FrameFaceAttempt,
}

struct DisplayRowCurrentTextSourceStepResult {
    result: DisplayRowRenderIntoRowResult,
    row_height_px: f32,
    row_ascent_px: f32,
}

struct DisplayRowCurrentSourceStepMutation<'a, 'request, 'renderer, 'face, 'host, S, P> {
    row_request: DisplayRowSourceRenderRequest<'request>,
    renderer: DisplayRowRenderer<'renderer>,
    source: &'a mut S,
    source_state: &'a mut DisplayRowSourceState,
    context: DisplayRowRenderContext<'face, 'host>,
    render_policy: &'a mut P,
}

struct DisplayRowNaturalSourceFragmentMutation<'a, 'request, 'metrics, 'face, 'host, S> {
    request: DisplayRowSourceRenderRequest<'request>,
    render_executor: &'a mut DisplayRowRenderExecutor<'metrics, 'face, 'host>,
    source: &'a mut S,
    source_state: &'a mut DisplayRowSourceState,
}

/// Geometry + face payload for one trailing `:extend` fill (GNU
/// `extend_face_to_end_of_line`). `face_id` is the extend face installed on the
/// row; `bg` is its background pixel; `(width_px, height_px, ascent_px)` are the
/// stretch geometry; `char_width` is the face's column advance used to size the
/// stretch column count.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowExtendFill {
    bg: Color,
    face_id: FaceId,
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
    char_width: f32,
}

impl RowExtendFill {
    pub(crate) fn new(
        bg: Color,
        face_id: FaceId,
        width_px: f32,
        height_px: f32,
        ascent_px: f32,
        char_width: f32,
    ) -> Self {
        Self {
            bg,
            face_id,
            width_px,
            height_px,
            ascent_px,
            char_width,
        }
    }

    /// Number of stretch columns (>=1) covering `width_px` at the face advance.
    fn width_cols(self) -> u16 {
        let cw = self.char_width.max(1.0);
        ((self.width_px / cw).ceil() as i64).clamp(1, u16::MAX as i64) as u16
    }
}

/// Mutation that appends the trailing extend-face stretch to the current row's
/// TEXT area, without emitting any output span. Mirrors GNU
/// `extend_face_to_end_of_line`: an empty row first gets a leading space glyph
/// (xdisp.c:24420) so the row `displays_text` and carries a face anchor; the
/// fill stretch is then pushed to the text-area right edge. Returns `true` when
/// a fill was applied.
struct RowExtendFillMutation {
    fill: RowExtendFill,
}

impl DisplayCurrentRowMutation for RowExtendFillMutation {
    type Output = bool;

    fn apply(self, row: &mut GlyphRow) -> Self::Output {
        // R2L safety: never fill a reversed row (the stretch would reorder to
        // the visual left). The caller already guards on reversed_p, but the
        // row is the authoritative source so we re-check here.
        if row.reversed_p {
            return false;
        }
        let text_index = GlyphArea::Text.index();
        // Empty row: push a leading space carrying the extend face so the row
        // displays text and has a face anchor (GNU xdisp.c:24420). It covers no
        // buffer position, so stamp the no-position sentinel (see GNU's
        // `glyph->charpos = -1` for the same anchor, src/xdisp.c:26021) — this
        // keeps the blank-line cursor from latching onto the fill.
        if row.glyphs[text_index].is_empty() {
            row.glyphs[text_index].push(
                Glyph::char(' ', self.fill.face_id, NO_BUFFER_POSITION_CHARPOS)
                    .with_pixel_width(self.fill.char_width.max(1.0)),
            );
            row.displays_text = true;
        }
        push_stretch_to_area(
            row,
            text_index,
            self.fill.width_cols(),
            self.fill.face_id,
            self.fill.width_px,
            self.fill.height_px,
            self.fill.ascent_px,
        );
        // The trailing fill stretch likewise maps to no buffer position; mark it
        // so cursor placement excludes it (GNU set_cursor_from_row, xdisp.c:18648).
        if let Some(last) = row.glyphs[text_index].last_mut() {
            last.charpos = NO_BUFFER_POSITION_CHARPOS;
        }
        true
    }
}

/// Re-face the current row's trailing whitespace glyphs with the
/// `trailing-whitespace` face (GNU `highlight_trailing_whitespace`, xdisp.c).
/// Walks the TEXT-area glyphs from the end backward over space/tab whitespace —
/// space `Char` glyphs and `Stretch` glyphs (tabs) — stamping each with
/// `face_id` until the first non-whitespace glyph. A `Glyph`'s background is
/// resolved from its `face_id`, so this paints the trailing run through the same
/// per-glyph background path the `region` face uses. Called only at true line
/// ends (before a real newline / at ZV), never at a visual wrap.
struct HighlightTrailingWhitespaceMutation {
    face_id: FaceId,
}

impl DisplayCurrentRowMutation for HighlightTrailingWhitespaceMutation {
    type Output = ();

    fn apply(self, row: &mut GlyphRow) -> Self::Output {
        let glyphs = &mut row.glyphs[GlyphArea::Text.index()];
        let mut start = glyphs.len();
        while start > 0 {
            let is_whitespace = match glyphs[start - 1].glyph_type {
                GlyphType::Char { ch } => ch == ' ' || ch == '\t',
                GlyphType::Stretch { .. } => true,
                _ => false,
            };
            if !is_whitespace {
                break;
            }
            start -= 1;
        }
        for glyph in &mut glyphs[start..] {
            glyph.face_id = self.face_id;
        }
    }
}

/// Geometry + faces for the `display-fill-column-indicator` glyph produced in a
/// row's trailing region (GNU `extend_face_to_end_of_line`, xdisp.c:24752): a
/// `gap` stretch pads from end-of-text to the indicator column, the indicator
/// character carries the `fill-column-indicator` face, and an optional `tail`
/// stretch continues to the right edge. On a plain row the gap is transparent
/// and there is no tail; on an `:extend`-highlighted row (region/hl-line) the
/// gap and tail carry the extend face so the whole trailing region stays
/// highlighted, and the indicator char face keeps the highlight background.
#[derive(Clone, Copy)]
struct FillColumnIndicatorFill {
    gap_px: f32,
    gap_cols: u16,
    gap_face_id: FaceId,
    indicator_char: char,
    indicator_face_id: FaceId,
    tail_px: f32,
    tail_cols: u16,
    tail_face_id: FaceId,
    char_width: f32,
    height_px: f32,
    ascent_px: f32,
}

struct FillColumnIndicatorMutation {
    fill: FillColumnIndicatorFill,
}

impl DisplayCurrentRowMutation for FillColumnIndicatorMutation {
    type Output = ();

    fn apply(self, row: &mut GlyphRow) -> Self::Output {
        // R2L rows reorder to the visual left; leave them alone (documented
        // limitation, mirroring RowExtendFillMutation).
        if row.reversed_p {
            return;
        }
        let text_index = GlyphArea::Text.index();
        let f = self.fill;
        // Pad the trailing region up to the indicator column.
        if f.gap_cols >= 1 && f.gap_px > 0.5 {
            push_stretch_to_area(
                row,
                text_index,
                f.gap_cols,
                f.gap_face_id,
                f.gap_px,
                f.height_px,
                f.ascent_px,
            );
            if let Some(last) = row.glyphs[text_index].last_mut() {
                last.charpos = NO_BUFFER_POSITION_CHARPOS;
            }
        }
        // The indicator character itself. It maps to no buffer position, so the
        // blank-line cursor never latches onto it.
        row.glyphs[text_index].push(
            Glyph::char(
                f.indicator_char,
                f.indicator_face_id,
                NO_BUFFER_POSITION_CHARPOS,
            )
            .with_pixel_width(f.char_width.max(1.0)),
        );
        // Continue the `:extend` highlight past the indicator to the right edge.
        if f.tail_cols >= 1 && f.tail_px > 0.5 {
            push_stretch_to_area(
                row,
                text_index,
                f.tail_cols,
                f.tail_face_id,
                f.tail_px,
                f.height_px,
                f.ascent_px,
            );
            if let Some(last) = row.glyphs[text_index].last_mut() {
                last.charpos = NO_BUFFER_POSITION_CHARPOS;
            }
        }
        row.displays_text = true;
    }
}

/// Mutation that rolls the current row's drawn glyphs back to a previously
/// captured `DisplayRowGlyphCheckpoint`. Used by the word-wrap break to drop the
/// partial-word glyphs that fit on the current row but belong on the next
/// continuation row. GNU keeps whole words by rewinding its iterator to the word
/// boundary; we mirror that by truncating the glyph row here while the source
/// position is rewound to the same boundary.
struct DisplayRowGlyphCheckpointRestoreMutation {
    checkpoint: DisplayRowGlyphCheckpoint,
}

impl DisplayCurrentRowMutation for DisplayRowGlyphCheckpointRestoreMutation {
    type Output = ();

    fn apply(self, row: &mut GlyphRow) -> Self::Output {
        self.checkpoint.restore(row);
    }
}

impl<S, P> DisplayCurrentRowMutation
    for DisplayRowCurrentSourceStepMutation<'_, '_, '_, '_, '_, S, P>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    type Output = Option<(DisplayRowRenderIntoRowResult, f32, f32)>;

    fn apply(self, row: &mut neomacs_display_protocol::glyph_matrix::GlyphRow) -> Self::Output {
        let mut renderer = self.renderer;
        let mut context = self.context;
        let result = self.row_request.render_fragment_step_into_row_with_policy(
            &mut renderer,
            row,
            self.source,
            self.source_state,
            &mut context,
            self.render_policy,
        )?;
        result.apply_current_row_effects_to(row);
        Some((result, row.height_px, row.ascent_px))
    }
}

impl<S> DisplayCurrentRowMutation for DisplayRowNaturalSourceFragmentMutation<'_, '_, '_, '_, '_, S>
where
    S: DisplayItemSource,
{
    type Output = Option<DisplayRowRenderIntoRowResult>;

    fn apply(self, row: &mut neomacs_display_protocol::glyph_matrix::GlyphRow) -> Self::Output {
        let result = self.render_executor.render_item_source_fragment_into_row(
            self.request,
            row,
            self.source,
            self.source_state,
        )?;
        result.apply_current_row_effects_to(row);
        Some(result)
    }
}

impl<'face, 'emit> DisplayRowCurrentTextSourceState<'face, 'emit> {
    fn new(
        row_output: DisplayRowCurrentRowOutput<'emit>,
        evaluator: &'emit mut Context,
        font_metrics: &'emit mut Option<FontMetricsService>,
        window_system: bool,
        face_resolver: &'face FaceResolver,
        face_ids: &'emit mut FrameFaceAttempt,
    ) -> Self {
        Self {
            row_output,
            evaluator,
            font_metrics,
            window_system,
            face_resolver,
            face_ids,
        }
    }

    fn render_source_with_policy<S, P>(
        &mut self,
        row_request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        render_policy: &mut P,
    ) -> Option<DisplayRowCurrentTextSourceStepResult>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        let mutation = DisplayRowCurrentSourceStepMutation {
            row_request,
            renderer: DisplayRowRenderer::new_for_frame(self.font_metrics, self.window_system),
            source,
            source_state,
            context: DisplayRowRenderContext::new(
                self.face_resolver,
                self.evaluator.display_host.as_deref(),
                self.face_ids,
            ),
            render_policy,
        };
        let (result, row_height_px, row_ascent_px) =
            self.row_output.apply_current_row_mutation(mutation)??;
        Some(DisplayRowCurrentTextSourceStepResult {
            result,
            row_height_px,
            row_ascent_px,
        })
    }
    fn measure_source_with_policy<S, P>(
        &mut self,
        row_request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        render_policy: &mut P,
    ) -> Option<DisplayRowCurrentTextSourceStepResult>
    where
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    {
        let mutation = DisplayRowCurrentSourceStepMutation {
            row_request,
            renderer: DisplayRowRenderer::new_for_frame(self.font_metrics, self.window_system),
            source,
            source_state,
            context: DisplayRowRenderContext::new(
                self.face_resolver,
                self.evaluator.display_host.as_deref(),
                self.face_ids,
            ),
            render_policy,
        };
        let (result, row_height_px, row_ascent_px) = self
            .row_output
            .apply_current_row_scratch_mutation(mutation)??;
        Some(DisplayRowCurrentTextSourceStepResult {
            result,
            row_height_px,
            row_ascent_px,
        })
    }
}

impl<'face, 'emit> DisplayRowCurrentSourceFragmentRenderState<'face, 'emit> {
    fn new(
        row_output: DisplayRowCurrentRowOutput<'emit>,
        font_metrics: &'emit mut Option<FontMetricsService>,
        window_system: bool,
        face_resolver: &'face FaceResolver,
        display_host: Option<&'emit dyn DisplayHost>,
        face_ids: &'emit mut FrameFaceAttempt,
    ) -> Self {
        Self {
            row_output,
            font_metrics,
            window_system,
            face_resolver,
            display_host,
            face_ids,
        }
    }

    fn render_natural_fragment_into_current_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let mut render_executor = DisplayRowRenderExecutor::new_for_frame(
            self.font_metrics,
            self.window_system,
            self.face_resolver,
            self.display_host,
            self.face_ids,
        );
        let result = self.row_output.apply_current_row_mutation(
            DisplayRowNaturalSourceFragmentMutation {
                request,
                render_executor: &mut render_executor,
                source,
                source_state,
            },
        )??;
        Some(result)
    }
}

impl DisplayRowCurrentTextSourceStepResult {
    fn into_measure_outcome(self) -> CurrentTextRowRenderOutcome {
        let (progress, source_slots, _faces, stop) = self.result.into_current_row_parts();
        let end = display_row_output_end_position(progress);
        CurrentTextRowRenderOutcome::new(
            stop,
            source_slots,
            end,
            self.row_height_px,
            self.row_ascent_px,
        )
    }
}

fn render_display_item_source_into_current_text_row<S, P>(
    state: &mut DisplayRowCurrentTextSourceState<'_, '_>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    request: DisplayRowSourceRenderRequest<'_>,
    render_policy: &mut P,
) -> Option<DisplayRowCurrentTextSourceStepResult>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    state.render_source_with_policy(request, source, source_state, render_policy)
}

fn measure_display_item_source_against_current_text_row<S, P>(
    state: &mut DisplayRowCurrentTextSourceState<'_, '_>,
    source: &mut S,
    source_state: &mut DisplayRowSourceState,
    request: DisplayRowSourceRenderRequest<'_>,
    render_policy: &mut P,
) -> Option<CurrentTextRowRenderOutcome>
where
    S: DisplayItemSource,
    P: DisplayRowRenderPolicy,
{
    state
        .measure_source_with_policy(request, source, source_state, render_policy)
        .map(DisplayRowCurrentTextSourceStepResult::into_measure_outcome)
}

impl<'a> TextRowOutputRenderState<'a> {
    pub(crate) fn from_parts(
        output: TextWindowOutputTarget<'a>,
        output_emitter: &'a mut WindowOutputEmitter,
        evaluator: &'a mut Context,
    ) -> Self {
        Self {
            output,
            output_emitter,
            evaluator,
        }
    }

    pub(crate) fn reborrow(&mut self) -> TextRowOutputRenderState<'_> {
        TextRowOutputRenderState {
            output: self.output.reborrow(),
            output_emitter: self.output_emitter,
            evaluator: self.evaluator,
        }
    }

    pub(crate) fn with_output_target_parts<R>(
        self,
        f: impl FnOnce(TextWindowOutputTarget<'_>, &mut WindowOutputEmitter, &mut Context) -> R,
    ) -> R {
        f(self.output, self.output_emitter, self.evaluator)
    }

    pub(crate) fn transition_text_row_with_limit(
        self,
        transition: DisplayTextRowGeometryTransition,
        max_rows: usize,
    ) -> DisplayTextRowTransition {
        transition_text_window_row_with_limit(
            self.output,
            self.output_emitter,
            self.evaluator,
            transition,
            max_rows,
        )
    }

    pub(crate) fn install_row_decoration(self, request: TextWindowRowDecorationRequest) {
        install_text_window_row_decoration_request(self.output, request);
    }

    fn insert_resolved_face(&mut self, face_id: FaceId, face: &ResolvedFace) {
        self.output.install_resolved_face(face_id, face, None);
    }

    fn install_resolved_measured_face(&mut self, face: &DisplayRowResolvedMeasuredFace) {
        self.output.install_resolved_face(
            face.face_id(),
            face.resolved_face(),
            face.font_metrics(),
        );
    }

    fn display_host(&self) -> Option<&dyn DisplayHost> {
        self.evaluator.display_host.as_deref()
    }

    fn evaluator(&self) -> &Context {
        self.evaluator
    }

    fn current_row_output(&mut self) -> DisplayRowCurrentRowOutput<'_> {
        self.output.current_row_output()
    }

    fn output_emitter(&mut self) -> &mut WindowOutputEmitter {
        self.output_emitter
    }

    fn output_emitter_ref(&self) -> &WindowOutputEmitter {
        self.output_emitter
    }

    fn output_rows(&self) -> &[DisplayRowSnapshot] {
        self.output_emitter.rows()
    }

    fn output_rows_len(&self) -> usize {
        self.output_emitter.rows().len()
    }

    fn measure_state<'emit>(
        &'emit mut self,
        font_metrics: &'emit mut Option<FontMetricsService>,
        window_system: bool,
        face_resolver: &'emit FaceResolver,
    ) -> TextRowSourceMeasureState<'emit> {
        TextRowSourceMeasureState {
            row_output: self.output.current_row_output(),
            evaluator: self.evaluator,
            font_metrics,
            window_system,
            face_resolver,
        }
    }

    fn current_text_render_state<'emit>(
        &'emit mut self,
        font_metrics: &'emit mut Option<FontMetricsService>,
        window_system: bool,
        face_resolver: &'emit FaceResolver,
        face_ids: &'emit mut FrameFaceAttempt,
    ) -> DisplayRowCurrentTextSourceState<'emit, 'emit> {
        DisplayRowCurrentTextSourceState::new(
            self.output.current_row_output(),
            self.evaluator,
            font_metrics,
            window_system,
            face_resolver,
            face_ids,
        )
    }

    fn current_source_fragment_render_state<'emit>(
        &'emit mut self,
        font_metrics: &'emit mut Option<FontMetricsService>,
        window_system: bool,
        face_resolver: &'emit FaceResolver,
        face_ids: &'emit mut FrameFaceAttempt,
    ) -> DisplayRowCurrentSourceFragmentRenderState<'emit, 'emit> {
        DisplayRowCurrentSourceFragmentRenderState::new(
            self.output.current_row_output(),
            font_metrics,
            window_system,
            face_resolver,
            self.evaluator.display_host.as_deref(),
            face_ids,
        )
    }

    /// Capture the current output row's glyph counts for a word-wrap candidate.
    fn capture_current_row_glyph_checkpoint(&self) -> DisplayRowGlyphCheckpoint {
        self.output.capture_current_row_glyph_checkpoint()
    }

    /// Truncate the current output row's drawn glyphs back to `checkpoint`,
    /// dropping the partial-word glyphs that the word-wrap break rewinds past.
    fn restore_current_row_glyph_checkpoint(&mut self, checkpoint: DisplayRowGlyphCheckpoint) {
        self.output
            .current_row_output()
            .apply_current_row_mutation(DisplayRowGlyphCheckpointRestoreMutation { checkpoint });
    }

    /// Append a trailing `:extend` fill stretch to the current row's TEXT area
    /// without emitting an output span. Returns `true` when a fill was applied.
    fn extend_current_row_face_to_end_of_line(&mut self, fill: RowExtendFill) -> bool {
        self.output
            .current_row_output()
            .apply_current_row_mutation(RowExtendFillMutation { fill })
            .unwrap_or(false)
    }

    /// Re-face the current row's trailing whitespace run with `face_id`
    /// (`show-trailing-whitespace`). See [`HighlightTrailingWhitespaceMutation`].
    fn highlight_current_row_trailing_whitespace(&mut self, face_id: FaceId) {
        self.output
            .current_row_output()
            .apply_current_row_mutation(HighlightTrailingWhitespaceMutation { face_id });
    }

    /// Produce the `display-fill-column-indicator` glyph in the current row's
    /// trailing region. See [`FillColumnIndicatorMutation`].
    fn produce_current_row_fill_column_indicator(&mut self, fill: FillColumnIndicatorFill) {
        self.output
            .current_row_output()
            .apply_current_row_mutation(FillColumnIndicatorMutation { fill });
    }

    fn finish_current_text_row_render(
        &mut self,
        output: TextRowOutput,
        result: DisplayRowCurrentTextSourceStepResult,
    ) -> CurrentTextRowRenderOutcome {
        let DisplayRowCurrentTextSourceStepResult {
            result,
            row_height_px,
            row_ascent_px,
        } = result;
        let (progress, source_slots, faces, stop) = result.into_current_row_parts();
        let end = display_row_output_end_position(progress);
        self.output.install_rendered_fragment_assets(&faces);
        let output_spans = output.spans_for_source_slots(&source_slots);
        self.output_emitter
            .emit_text_output_spans(self.evaluator, output, output_spans, end);
        CurrentTextRowRenderOutcome::new(stop, source_slots, end, row_height_px, row_ascent_px)
    }
}

pub(crate) struct TextRowSourceRenderState<'a> {
    output_render: TextRowOutputRenderState<'a>,
    font_metrics: &'a mut Option<FontMetricsService>,
    window_system: bool,
    face_resolver: &'a FaceResolver,
}

impl<'a> TextRowSourceRenderState<'a> {
    pub(crate) fn from_output_render(
        output_render: TextRowOutputRenderState<'a>,
        font_metrics: &'a mut Option<FontMetricsService>,
        window_system: bool,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            output_render,
            font_metrics,
            window_system,
            face_resolver,
        }
    }

    pub(crate) fn reborrow(&mut self) -> TextRowSourceRenderState<'_> {
        TextRowSourceRenderState {
            output_render: self.output_render.reborrow(),
            font_metrics: self.font_metrics,
            window_system: self.window_system,
            face_resolver: self.face_resolver,
        }
    }

    pub(crate) fn output_render(&mut self) -> TextRowOutputRenderState<'_> {
        self.output_render.reborrow()
    }

    pub(crate) fn measure_state(&mut self) -> TextRowSourceMeasureState<'_> {
        self.output_render
            .measure_state(self.font_metrics, self.window_system, self.face_resolver)
    }

    pub(crate) fn insert_resolved_face(&mut self, face_id: FaceId, face: &ResolvedFace) {
        self.output_render.insert_resolved_face(face_id, face);
    }

    fn install_pending_display_string_base_face(&mut self, base_face: &DisplayStringBaseFace) {
        if let Some(pending_face) = base_face.pending_face() {
            self.insert_resolved_face(pending_face.face_id(), pending_face.resolved());
        }
    }

    fn resolved_measured_face(
        &mut self,
        measurement_policy: DisplayRowMeasurementPolicy,
        face_id: FaceId,
        face: ResolvedFace,
        window_system: bool,
        fallback_char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> DisplayRowResolvedMeasuredFace {
        let metrics = if window_system {
            self.font_metrics.as_mut().map(|svc| {
                svc.font_metrics(
                    &face.font_family,
                    face.font_weight,
                    face.italic,
                    face.font_size,
                )
            })
        } else {
            None
        };
        measurement_policy.resolved_measured_face(
            face_id,
            face,
            metrics,
            fallback_char_width,
            fallback_metrics,
            self.font_metrics,
        )
    }

    pub(crate) fn resolve_and_install_measured_face(
        &mut self,
        measurement_policy: DisplayRowMeasurementPolicy,
        face_id: FaceId,
        face: ResolvedFace,
        window_system: bool,
        fallback_char_width: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
    ) -> DisplayRowActiveFaceState {
        let resolved_face = self.resolved_measured_face(
            measurement_policy,
            face_id,
            face,
            window_system,
            fallback_char_width,
            fallback_metrics,
        );
        self.output_render
            .install_resolved_measured_face(&resolved_face);
        resolved_face.into_active_face_state()
    }

    pub(crate) fn resolve_named_face(&self, face_name: &str) -> ResolvedFace {
        self.face_resolver.resolve_named_face(face_name)
    }

    /// Merge a named face over a base resolved face, GNU
    /// `merge_faces(w, <named-face>, 0, base_face_id)`: start from `base`'s full
    /// attribute set, overlay only the attributes the named face specifies
    /// (resolving its `:inherit` chain), and return the realized face. Returns
    /// `base` unchanged when the named face contributes nothing.
    pub(crate) fn merge_named_face_over(
        &self,
        base: &ResolvedFace,
        face_name: &str,
    ) -> ResolvedFace {
        self.face_resolver
            .resolve_face_value_over(base, &Value::symbol(face_name))
            .unwrap_or_else(|| base.clone())
    }

    pub(crate) fn default_face(&self) -> ResolvedFace {
        self.face_resolver.default_face().clone()
    }

    pub(crate) fn display_string_base_face<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        policy: BaseFacePolicy,
        face_ids: &mut FrameFaceAttempt,
    ) -> DisplayStringBaseFace {
        let base_face = resolve_display_string_base_face(
            buffer,
            self.face_resolver,
            origin,
            policy,
            None,
            DisplayDefaultFaceInstallPolicy::InstallDefaultFace,
            face_ids,
        );
        self.install_pending_display_string_base_face(&base_face);
        base_face
    }

    pub(crate) fn default_display_string_base_face<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        face_ids: &mut FrameFaceAttempt,
    ) -> DisplayStringBaseFace {
        self.display_string_base_face(buffer, origin, origin.default_base_face_policy(), face_ids)
    }

    pub(crate) fn display_string_base_face_for_active_row<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        policy: BaseFacePolicy,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceAttempt,
    ) -> DisplayStringBaseFace {
        let base_face = resolve_display_string_base_face(
            buffer,
            self.face_resolver,
            origin,
            policy,
            Some(ActiveDisplayStringBaseFace::new(
                active_face_state.face_id(),
                active_face_state.resolved_face(),
            )),
            DisplayDefaultFaceInstallPolicy::ReuseInstalledDefaultFace,
            face_ids,
        );
        self.install_pending_display_string_base_face(&base_face);
        base_face
    }

    pub(crate) fn default_display_string_base_face_for_active_row<B: LayoutBufferView>(
        &mut self,
        buffer: &B,
        origin: DisplayOrigin,
        active_face_state: &DisplayRowActiveFaceState,
        face_ids: &mut FrameFaceAttempt,
    ) -> DisplayStringBaseFace {
        self.display_string_base_face_for_active_row(
            buffer,
            origin,
            origin.default_base_face_policy(),
            active_face_state,
            face_ids,
        )
    }

    pub(crate) fn render_natural_fragment_into_current_row<S: DisplayItemSource>(
        &mut self,
        request: DisplayRowSourceRenderRequest<'_>,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        face_ids: &mut FrameFaceAttempt,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        self.output_render
            .current_source_fragment_render_state(
                self.font_metrics,
                self.window_system,
                self.face_resolver,
                face_ids,
            )
            .render_natural_fragment_into_current_row(request, source, source_state)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_natural_fragment_from_row_geometry_columns<S: DisplayItemSource>(
        &mut self,
        row_geometry: &DisplayRowGeometryState,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        cols: usize,
        char_width: f32,
        role: neomacs_display_protocol::frame_glyphs::GlyphRowRole,
        face_id: FaceId,
        base_face: &ResolvedFace,
        start_col: usize,
        max_col: usize,
        area: neomacs_display_protocol::glyph_matrix::GlyphArea,
        face_ids: &mut FrameFaceAttempt,
    ) -> Option<DisplayRowRenderIntoRowResult> {
        let request = DisplayRowSourceFragmentFrame::from_row_geometry_columns(
            row_geometry,
            cols,
            char_width,
            role,
            face_id,
            base_face,
        )
        .render_request_from_column_for_area(start_col, max_col, area);
        self.render_natural_fragment_into_current_row(request, source, source_state, face_ids)
    }

    pub(crate) fn render_display_item_source_into_current_text_row_and_emit<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        face_ids: &mut FrameFaceAttempt,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        request: DisplayRowAppendSourceRenderRequest<'_>,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let mut state = current_text_render_state(self, face_ids);
        let (result, output) = request.render_with_row_request(|row_request| {
            render_display_item_source_into_current_text_row(
                &mut state,
                source,
                source_state,
                row_request,
                render_policy,
            )
        });
        let result = result?;
        Some(
            self.output_render
                .finish_current_text_row_render(output, result),
        )
    }

    pub(crate) fn mark_current_text_row_truncated_left(&mut self) {
        self.output_render()
            .install_row_decoration(TextWindowRowDecorationRequest::MarkCurrentTruncatedLeft);
    }

    /// Fill the current row's background from the current pen `x` to the
    /// text-area `right_edge` with the active `:extend` face (GNU
    /// `extend_face_to_end_of_line`). No-op (returns `false`) when the row is
    /// reversed (R2L), when the fill width is non-positive, or when the extend
    /// background equals the frame background.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn extend_face_to_end_of_line(
        &mut self,
        row_extend: &DisplayRowScopedValue<(Color, FaceId)>,
        row_geometry: &DisplayRowGeometryState,
        current_x: f32,
        right_edge: f32,
        frame_background: Color,
        reversed_p: bool,
        height_px: f32,
        ascent_px: f32,
        char_width: f32,
    ) -> bool {
        if reversed_p {
            return false;
        }
        let fill_px = right_edge - current_x;
        if fill_px <= 0.0 {
            return false;
        }
        let Some(&(bg, face_id)) = row_extend.value_on(row_geometry) else {
            return false;
        };
        if bg == frame_background {
            return false;
        }
        self.output_render
            .extend_current_row_face_to_end_of_line(RowExtendFill::new(
                bg, face_id, fill_px, height_px, ascent_px, char_width,
            ))
    }

    /// Highlight the current row's trailing whitespace with the
    /// `trailing-whitespace` face when `show-trailing-whitespace` is enabled
    /// (GNU `highlight_trailing_whitespace`, xdisp.c). No-op when disabled. Must
    /// be called at a true line end (real newline / ZV), before the row extend
    /// fill and the row transition — never at a visual wrap, where the
    /// wrap-boundary whitespace is not "trailing".
    pub(crate) fn highlight_trailing_whitespace(
        &mut self,
        trailing_whitespace: &TrailingWhitespaceRenderState,
        face_ids: &mut FrameFaceAttempt,
    ) {
        if !trailing_whitespace.is_enabled() {
            return;
        }
        let face = self.resolve_named_face("trailing-whitespace");
        let face_id = crate::display_row_face_state::stable_face_id_for_resolved(face_ids, &face);
        self.insert_resolved_face(face_id, &face);
        self.output_render
            .highlight_current_row_trailing_whitespace(face_id);
    }

    /// Produce the `display-fill-column-indicator` glyph at the indicator column
    /// in the current row's trailing region, for every non-continuation text row
    /// (GNU `extend_face_to_end_of_line` / `fill_column_indicator_column`,
    /// xdisp.c). `indicator_col` is the buffer column (negative = disabled), so
    /// the indicator pixel x is `content_x + indicator_col * char_width`. Unlike
    /// GNU (whose `fill_column_indicator_column` adds `lnum_pixel_width` because
    /// its origin is the text-area left edge), neomacs's `content_x` is already
    /// the buffer-text origin — it shifts right by the line-number gutter — so
    /// the gutter width must NOT be added again (doing so double-counts it and
    /// pushes the indicator past its column when line numbers are on).
    /// No-op when disabled or when the row text already reached/passed the column
    /// (GNU begins the trailing fill at end-of-text, so a longer line covers it).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn produce_fill_column_indicator(
        &mut self,
        indicator_col: i32,
        indicator_char: char,
        text_end_col: i64,
        content_x: f32,
        char_width: f32,
        pen_x: f32,
        right_edge: f32,
        row_extend: &DisplayRowScopedValue<(Color, FaceId)>,
        row_geometry: &DisplayRowGeometryState,
        frame_background: Color,
        height_px: f32,
        ascent_px: f32,
        face_ids: &mut FrameFaceAttempt,
    ) -> bool {
        if indicator_col < 0 || char_width <= 0.0 {
            return false;
        }
        // Decide whether the indicator is visible by COLUMN, not pixels: GNU
        // draws it at grid column `indicator_col`, so it shows whenever the
        // text ends at or before that column (`text_end_col <= indicator_col`).
        // A pixel comparison is wrong here because neomacs's nominal
        // `char_width` differs from the measured per-glyph advance, so a line
        // whose length is EXACTLY the fill column overshoots the nominal grid
        // (e.g. 10 chars × 16.255 vs 10 × 16.0 ⇒ pen_x 2.5px past indicator_px)
        // and the indicator was dropped — the divergence grows with the column.
        if text_end_col > i64::from(indicator_col) {
            // The row text passed the indicator column; the indicator is covered
            // (GNU begins the trailing fill at end-of-text). Let the caller run
            // the normal `:extend` fill instead.
            return false;
        }
        let indicator_px = content_x + indicator_col as f32 * char_width;
        // Positioning stays pixel-based: when the text ends exactly at the
        // indicator column, `pen_x` may be a hair past `indicator_px`, so clamp
        // the gap to 0 and place the indicator right after the text (as GNU does).
        let gap_px = indicator_px - pen_x;
        let gap_px = gap_px.max(0.0);
        let gap_cols = (gap_px / char_width).round().clamp(0.0, u16::MAX as f32) as u16;
        // An `:extend` face (region / hl-line) fills the trailing region with its
        // background; produce the indicator INSIDE that fill so the highlight is
        // continuous, mirroring GNU's `extend_face_to_end_of_line` which merges
        // `fill-column-indicator` over the extend face at the indicator column.
        let extend = row_extend
            .value_on(row_geometry)
            .copied()
            .filter(|(bg, _)| *bg != frame_background);
        let fill = match extend {
            Some((extend_bg, extend_face_id)) => {
                let mut char_face = self.resolve_named_face("fill-column-indicator");
                char_face.bg = protocol_color_to_pixel(extend_bg);
                char_face.use_default_background = false;
                let indicator_face_id = crate::display_row_face_state::stable_face_id_for_resolved(
                    face_ids, &char_face,
                );
                self.insert_resolved_face(indicator_face_id, &char_face);
                let tail_px = (right_edge - (indicator_px + char_width)).max(0.0);
                let tail_cols = (tail_px / char_width).round().clamp(0.0, u16::MAX as f32) as u16;
                FillColumnIndicatorFill {
                    gap_px,
                    gap_cols,
                    gap_face_id: extend_face_id,
                    indicator_char,
                    indicator_face_id,
                    tail_px,
                    tail_cols,
                    tail_face_id: extend_face_id,
                    char_width,
                    height_px,
                    ascent_px,
                }
            }
            None => {
                // Plain row: the gap is transparent (fill-column-indicator has no
                // background) and there is no tail past the indicator.
                let fci = self.resolve_named_face("fill-column-indicator");
                let face_id =
                    crate::display_row_face_state::stable_face_id_for_resolved(face_ids, &fci);
                self.insert_resolved_face(face_id, &fci);
                FillColumnIndicatorFill {
                    gap_px,
                    gap_cols,
                    gap_face_id: face_id,
                    indicator_char,
                    indicator_face_id: face_id,
                    tail_px: 0.0,
                    tail_cols: 0,
                    tail_face_id: face_id,
                    char_width,
                    height_px,
                    ascent_px,
                }
            }
        };
        self.output_render
            .produce_current_row_fill_column_indicator(fill);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_display_property_replacement_row_request(
        &mut self,
        descriptor: &DisplayPropertyReplacementDescriptor,
        source_text: &[u8],
        active_face_state: &DisplayRowActiveFaceState,
        current_x: f32,
        content_x: f32,
        params: &WindowParams,
        glyph_y_offset: f32,
        fallback_metrics: DisplayRowFallbackMetrics,
        start_position: DisplayRowPosition,
    ) -> Option<DisplayPropertyReplacementRowRenderRequest> {
        DisplayPropertyReplacementRowRenderRequest::from_typed_replacement_descriptor(
            descriptor,
            source_text,
            active_face_state,
            self.font_metrics,
            current_x,
            content_x,
            params,
            self.output_render.display_host(),
            glyph_y_offset,
            fallback_metrics,
            start_position,
        )
    }

    /// Record a `(left-fringe …)` / `(right-fringe …)` fringe-bitmap descriptor
    /// on the current row, if the typed replacement is a fringe spec. The text
    /// area still shows nothing (the replacement resolves to `Empty`); this only
    /// attaches the bitmap so the frame-output bridge can draw it in the fringe.
    pub(crate) fn record_fringe_bitmap_for_descriptor(
        &mut self,
        descriptor: &DisplayPropertyReplacementDescriptor,
        face_ids: &mut FrameFaceAttempt,
        active_face_state: &DisplayRowActiveFaceState,
    ) {
        if let Some(DisplayReplacementProperty::Fringe(layout)) =
            descriptor.classification().replacement()
        {
            let layout = *layout;
            self.record_fringe_bitmap_layout(&layout, face_ids, active_face_state.face_id());
        }
    }

    /// Record a parsed fringe layout (from any source path) on the current row.
    /// `fallback_face_id` is the row's active face, used only when neither a
    /// `set-fringe-bitmap-face` override nor the spec's FACE resolves.
    ///
    /// Resolution honors GNU's `set-fringe-bitmap-face` override: the face name
    /// stored on the registry entry wins over the spec's FACE argument.
    pub(crate) fn record_fringe_bitmap_layout(
        &mut self,
        layout: &crate::display_spec::DisplayFringeLayout,
        face_ids: &mut FrameFaceAttempt,
        fallback_face_id: FaceId,
    ) {
        let layout = *layout;

        // Resolve the bitmap symbol -> registry index, and capture the registry
        // face override name (GC-safe String) before borrowing self mutably.
        let evaluator = self.output_render.evaluator();
        let (bitmap_index, registry_face_name) =
            match evaluator.fringe_bitmap_for_symbol(layout.bitmap) {
                Some((index, bitmap)) => {
                    if index > u32::from(u16::MAX) {
                        return;
                    }
                    (index as u16, bitmap.face.clone())
                }
                // No registered user bitmap (e.g. a standard built-in we don't
                // implement yet): nothing to draw.
                None => return,
            };

        // The face id: prefer the `set-fringe-bitmap-face` override, then the
        // spec's FACE, then the row's active face.
        let face_id = self.resolve_fringe_face_id(
            registry_face_name.as_deref(),
            layout.face,
            face_ids,
            fallback_face_id,
        );

        let info = FringeBitmapInfo {
            bitmap_index,
            face_id,
        };
        let side = layout.side;
        self.output_render
            .current_row_output()
            .apply_current_row_mutation(SetRowFringeBitmapMutation { side, info });
    }

    /// Resolve the face id used for a fringe bitmap. `override_name` is the
    /// `set-fringe-bitmap-face` registry override (highest priority); `spec_face`
    /// is the FACE from the display spec; the active row face is the fallback.
    fn resolve_fringe_face_id(
        &mut self,
        override_name: Option<&str>,
        spec_face: Option<Value>,
        face_ids: &mut FrameFaceAttempt,
        fallback_face_id: FaceId,
    ) -> FaceId {
        if let Some(name) = override_name {
            let resolved = self.face_resolver.resolve_named_face(name);
            let face_id =
                crate::display_row_face_state::stable_face_id_for_resolved(face_ids, &resolved);
            self.insert_resolved_face(face_id, &resolved);
            return face_id;
        }
        if let Some(face_value) = spec_face
            && let Some(resolved) = self
                .face_resolver
                .resolve_face_value_over(self.face_resolver.default_face(), &face_value)
        {
            let face_id =
                crate::display_row_face_state::stable_face_id_for_resolved(face_ids, &resolved);
            self.insert_resolved_face(face_id, &resolved);
            return face_id;
        }
        fallback_face_id
    }

    pub(crate) fn output_emitter(&mut self) -> &mut WindowOutputEmitter {
        self.output_render.output_emitter()
    }

    pub(crate) fn output_emitter_ref(&self) -> &WindowOutputEmitter {
        self.output_render.output_emitter_ref()
    }

    /// Capture the current row's drawn-glyph counts at a word-wrap candidate so
    /// the eventual break can roll the partial word off the row.
    pub(crate) fn capture_glyph_checkpoint(&self) -> DisplayRowGlyphCheckpoint {
        self.output_render.capture_current_row_glyph_checkpoint()
    }

    /// Roll the current row's drawn glyphs back to `checkpoint` when the
    /// word-wrap break rewinds to a word boundary.
    pub(crate) fn restore_glyph_checkpoint(&mut self, checkpoint: DisplayRowGlyphCheckpoint) {
        self.output_render
            .restore_current_row_glyph_checkpoint(checkpoint);
    }

    pub(crate) fn output_rows(&self) -> &[DisplayRowSnapshot] {
        self.output_render.output_rows()
    }

    pub(crate) fn output_rows_len(&self) -> usize {
        self.output_render.output_rows_len()
    }
}

fn current_text_render_state<'emit>(
    state: &'emit mut TextRowSourceRenderState<'_>,
    face_ids: &'emit mut FrameFaceAttempt,
) -> DisplayRowCurrentTextSourceState<'emit, 'emit> {
    state.output_render.current_text_render_state(
        state.font_metrics,
        state.window_system,
        state.face_resolver,
        face_ids,
    )
}

pub(crate) struct TextRowSourceMeasureState<'a> {
    row_output: DisplayRowCurrentRowOutput<'a>,
    evaluator: &'a mut Context,
    font_metrics: &'a mut Option<FontMetricsService>,
    window_system: bool,
    face_resolver: &'a FaceResolver,
}

impl<'a> TextRowSourceMeasureState<'a> {
    #[cfg(test)]
    pub(crate) fn from_current_row(
        row_output: DisplayRowCurrentRowOutput<'a>,
        evaluator: &'a mut Context,
        font_metrics: &'a mut Option<FontMetricsService>,
        face_resolver: &'a FaceResolver,
    ) -> Self {
        Self {
            row_output,
            evaluator,
            font_metrics,
            window_system: false,
            face_resolver,
        }
    }

    pub(crate) fn font_metrics(&mut self) -> &mut Option<FontMetricsService> {
        self.font_metrics
    }

    pub(crate) fn current_cluster_tail(&self) -> Option<(char, bool)> {
        self.row_output.cluster_tail()
    }

    pub(crate) fn measure_display_item_source_against_current_text_row<
        S: DisplayItemSource,
        P: DisplayRowRenderPolicy,
    >(
        &mut self,
        face_ids: &mut FrameFaceAttempt,
        source: &mut S,
        source_state: &mut DisplayRowSourceState,
        row_request: DisplayRowSourceRenderRequest<'_>,
        render_policy: &mut P,
    ) -> Option<CurrentTextRowRenderOutcome> {
        let mut state = current_text_measure_state(self, face_ids);
        measure_display_item_source_against_current_text_row(
            &mut state,
            source,
            source_state,
            row_request,
            render_policy,
        )
    }
}

fn current_text_measure_state<'emit>(
    state: &'emit mut TextRowSourceMeasureState<'_>,
    face_ids: &'emit mut FrameFaceAttempt,
) -> DisplayRowCurrentTextSourceState<'emit, 'emit> {
    DisplayRowCurrentTextSourceState::new(
        state.row_output.reborrow(),
        state.evaluator,
        state.font_metrics,
        state.window_system,
        state.face_resolver,
        face_ids,
    )
}

#[cfg(test)]
#[path = "display_row_extend_fill_test.rs"]
mod extend_fill_tests;

#[cfg(test)]
#[path = "display_row_trailing_whitespace_test.rs"]
mod trailing_whitespace_tests;

#[cfg(test)]
#[path = "display_row_fill_column_indicator_test.rs"]
mod fill_column_indicator_tests;
