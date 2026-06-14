use super::*;
use crate::display_cursor::CursorCaptureState;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{
    DisplayImageItem, DisplayItemKind, DisplayLength, DisplayMediaReplacement,
    DisplaySourceMappedText, DisplaySourcePosition, DisplayStretch, DisplayStretchWidth,
    DisplayVideoItem, DisplayXwidgetItem, GlyphlessMethod, RenderFaceRef,
};
use crate::display_origin::{DisplayOrigin, DisplayPropertySource, OverlayStringKind};
use crate::display_property::{
    DisplayMediaReplacementProperty, DisplayPropertyClassification, DisplayReplacementProperty,
    classify_display_property,
};
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowCurrentTextRenderState, DisplayRowFallbackMetrics,
    DisplayRowGeometry, DisplayRowItemSourceRenderRequest, DisplayRowMeasuredFaceMetrics,
    DisplayRowMeasurementPolicy, DisplayRowRenderBounds, DisplayRowRenderPolicy,
    DisplayRowRenderer, DisplayRowSourceRequestPolicy, DisplayRowSourceState,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowGlyphSlot,
    DisplayRowItemMeasurement, DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::{
    DisplayRowBoundaryTarget, DisplayRowFlagKind, DisplayRowFlags, DisplayRowGeometryDefaults,
    DisplayRowGeometryState, DisplayRowHitRange, DisplayRowLimit, DisplayRowStartMarker,
    DisplayRowVisibilityLimit, DisplayRowYPositions,
};
use crate::display_row_walk_state::{
    ActiveDisplayPropertySpan, BufferTextRowOverflowDecision, FaceScanCheckpoint,
    HitRowRangeTracker, LineNumberRenderState, WordWrapBreakCandidate, WordWrapRenderState,
};
use crate::display_text_run_measurement::{DisplayTextRunAdvance, DisplayTextRunMeasurement};
use crate::neovm_bridge::{
    FaceResolver, LayoutBufferSnapshot, OverlayDisplayString, RustBufferAccess,
};
use crate::window_output::TextMatrixRowTransition;
use neomacs_display_protocol::effect_config::EffectsConfig;
use neomacs_display_protocol::face::BasicFaceId;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphArea, GlyphType};
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, EmacsByteRange, LispCharPos1};
use neovm_core::emacs_core::eval::{
    DisplayHost, GuiFrameHostRequest, ImageResolveRequest, ResolvedImage,
};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;
use std::sync::{Arc, Mutex};

fn write_char_to_current_row_with_width(
    builder: &mut crate::matrix_builder::GlyphMatrixBuilder,
    ch: char,
    face_id: u32,
    charpos: usize,
    pixel_width: f32,
) {
    builder
        .with_current_row_mut(|row| {
            crate::glyph_row_writer::push_char_to_row(row, ch, face_id, charpos, pixel_width);
        })
        .expect("current row");
}

fn emitted_row(
    row: i64,
    y: i64,
    height: i64,
    start_lisp: i64,
    end_lisp: i64,
) -> neovm_core::window::DisplayRowSnapshot {
    neovm_core::window::DisplayRowSnapshot {
        row,
        y,
        height,
        start_x: 0,
        start_col: 0,
        end_x: 0,
        end_col: 0,
        start_buffer_pos: Some(LispCharPos1::new(start_lisp)),
        end_buffer_pos: Some(LispCharPos1::new(end_lisp)),
    }
}

struct RecordingAppendImageHost {
    requests: Arc<Mutex<Vec<ImageResolveRequest>>>,
}

struct RowTransitionTestContext {
    eval: Context,
    output_emitter: crate::window_output::WindowOutputEmitter,
    builder: crate::matrix_builder::GlyphMatrixBuilder,
    defaults: DisplayRowGeometryDefaults,
    geometry: DisplayRowGeometryState,
    row_y_positions: DisplayRowYPositions,
    hit_rows: Vec<crate::hit_test::HitRow>,
    row_flags: DisplayRowFlags,
    row_limit: DisplayRowLimit,
}

impl RowTransitionTestContext {
    fn new(frame_name: &str) -> Self {
        let mut eval = Context::new();
        let buf_id = eval
            .buffer_manager()
            .current_buffer()
            .expect("current buffer")
            .id();
        let frame_id = eval
            .frame_manager_mut()
            .create_frame(frame_name, 320, 120, buf_id);
        let window_id = eval
            .frame_manager()
            .get(frame_id)
            .expect("frame")
            .selected_window;
        let mut output_emitter =
            crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
        output_emitter.begin_update(&mut eval);
        output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

        let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
        builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 48.0), true);
        builder.begin_row(0, GlyphRowRole::Text);
        let defaults = DisplayRowGeometryDefaults::new(0.0, 16.0, 12.0);
        let geometry = defaults.initial_state();
        let max_rows = 4;

        Self {
            eval,
            output_emitter,
            builder,
            defaults,
            geometry,
            row_y_positions: DisplayRowYPositions::with_capacity_and_first_row(max_rows, 0.0),
            hit_rows: Vec::new(),
            row_flags: DisplayRowFlags::new(max_rows),
            row_limit: DisplayRowLimit { max_rows },
        }
    }
}

#[test]
fn buffer_line_number_margin_render_request_renders_and_consumes_pending_margin() {
    let mut context = RowTransitionTestContext::new("line-number-margin-render-request");
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(7);
    let mut line_numbers = LineNumberRenderState::new(true, 12, 9);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;

    assert!(
        BufferLineNumberMarginRenderRequest::new(1, false, 0, 4, 4).render_pending(
            &mut line_numbers,
            &face_resolver,
            &mut face_ids,
            &mut context.builder,
            &context.geometry,
            &mut face_scan,
            8.0,
        )
    );

    context.builder.end_row();
    context.builder.end_window();
    let state = context.builder.finish(20, 1, 8.0, 16.0);
    let margin = &state.window_matrices[0].matrix.rows[0].glyphs[GlyphArea::LeftMargin as usize];

    assert_eq!(margin.len(), 4);
    assert_eq!(margin[0].glyph_type, GlyphType::Stretch { width_cols: 1 });
    assert_eq!(margin[1].glyph_type, GlyphType::Char { ch: '1' });
    assert_eq!(margin[2].glyph_type, GlyphType::Char { ch: '2' });
    assert_eq!(margin[3].glyph_type, GlyphType::Stretch { width_cols: 1 });
    assert!(
        margin
            .iter()
            .all(|glyph| glyph.face_id == BasicFaceId::SENTINEL)
    );
    assert_eq!(face_scan, FaceScanCheckpoint::initial());
    assert!(!line_numbers.should_render());
}

impl DisplayHost for RecordingAppendImageHost {
    fn realize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn resize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn resolve_image(
        &self,
        _request: ImageResolveRequest,
    ) -> Result<Option<ResolvedImage>, String> {
        panic!("append display source rendering must use nonblocking request_image");
    }

    fn request_image(&self, request: ImageResolveRequest) -> Result<Option<ResolvedImage>, String> {
        self.requests
            .lock()
            .expect("image requests lock")
            .push(request);
        Ok(Some(ResolvedImage {
            image_id: 42,
            width: 64,
            height: 32,
            dimensions_known: true,
        }))
    }
}

#[test]
fn display_row_append_metrics_builds_from_measured_face_metrics() {
    let metrics = DisplayRowAppendMetrics::from_measured_face_metrics(
        DisplayRowMeasuredFaceMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
            space_width: 8.0,
        },
        16.0,
    );

    assert_eq!(
        metrics,
        DisplayRowAppendMetrics {
            height: 18.0,
            ascent: 13.0,
            char_width: 7.5,
            space_width: 8.0,
            default_row_height: 16.0,
        }
    );
}

#[test]
fn display_row_append_metrics_builds_from_active_face_state() {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base = resolver.default_face().clone();
    let mut font_metrics = None;
    let measured = DisplayRowMeasurementPolicy::for_frame(false).measured_face(
        7,
        &base,
        None,
        7.5,
        DisplayRowFallbackMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
        },
        &mut font_metrics,
    );
    let active_face = DisplayRowActiveFaceState::new(base, measured);

    let metrics = DisplayRowAppendMetrics::from_active_face_state(&active_face, 16.0);

    assert_eq!(
        metrics,
        DisplayRowAppendMetrics {
            height: 18.0,
            ascent: 13.0,
            char_width: 7.5,
            space_width: 8.0,
            default_row_height: 16.0,
        }
    );
}

#[test]
fn display_row_append_metrics_builds_display_box_from_active_face_state() {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base = resolver.default_face().clone();
    let mut font_metrics = None;
    let measured = DisplayRowMeasurementPolicy::for_frame(false).measured_face(
        7,
        &base,
        None,
        7.5,
        DisplayRowFallbackMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
        },
        &mut font_metrics,
    );
    let active_face = DisplayRowActiveFaceState::new(base, measured);

    let metrics =
        DisplayRowAppendMetrics::display_box_from_active_face_state(&active_face, 42.0, 31.0, 16.0);

    assert_eq!(
        metrics,
        DisplayRowAppendMetrics {
            height: 42.0,
            ascent: 31.0,
            char_width: 7.5,
            space_width: 8.0,
            default_row_height: 16.0,
        }
    );
}

#[test]
fn buffer_current_face_resolution_context_skips_before_checkpoint() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let default_face = face_resolver.default_face().clone();
    let mut font_metrics = None;
    let measurement_policy = DisplayRowMeasurementPolicy::for_frame(false);
    let measured = measurement_policy.measured_face(
        7,
        &default_face,
        None,
        8.0,
        DisplayRowFallbackMetrics::from_default_face_extents(8.0, 16.0, 12.0),
        &mut font_metrics,
    );
    let mut active_face = DisplayRowActiveFaceState::new(default_face.clone(), measured);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;
    let height_span = ActiveDisplayPropertySpan::inactive();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let mut row_geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut row_extend = DisplayRowScopedValue::inactive();
    let mut box_face = BoxFaceRowState::inactive();

    let resolved = BufferCurrentFaceResolutionContext::new(
        &buffer,
        &face_resolver,
        measurement_policy,
        &default_face,
        8.0,
        12.0,
        16.0,
        8.0,
        16.0,
        12.0,
        false,
    )
    .resolve_at_checkpoint(
        &mut BufferCurrentFaceResolutionState::new(
            &mut face_scan,
            &height_span,
            &mut font_metrics,
            &mut face_ids,
            &mut builder,
            &mut active_face,
            &mut row_geometry,
            &mut row_extend,
            &mut box_face,
            0.0,
        ),
        1,
    );

    assert!(!resolved);
    assert_eq!(active_face.face_id(), 7);
    assert_eq!(face_ids.allocate(), 20);
}

#[test]
fn buffer_current_face_resolution_context_resolves_due_face() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let default_face = face_resolver.default_face().clone();
    let mut font_metrics = None;
    let measurement_policy = DisplayRowMeasurementPolicy::for_frame(false);
    let measured = measurement_policy.measured_face(
        7,
        &default_face,
        None,
        8.0,
        DisplayRowFallbackMetrics::from_default_face_extents(8.0, 16.0, 12.0),
        &mut font_metrics,
    );
    let mut active_face = DisplayRowActiveFaceState::new(default_face.clone(), measured);
    let mut face_scan = FaceScanCheckpoint::initial();
    let height_span = ActiveDisplayPropertySpan::inactive();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let mut row_geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 8.0, 6.0);
    let mut row_extend = DisplayRowScopedValue::inactive();
    let mut box_face = BoxFaceRowState::inactive();

    let resolved = BufferCurrentFaceResolutionContext::new(
        &buffer,
        &face_resolver,
        measurement_policy,
        &default_face,
        8.0,
        12.0,
        16.0,
        8.0,
        16.0,
        12.0,
        false,
    )
    .resolve_at_checkpoint(
        &mut BufferCurrentFaceResolutionState::new(
            &mut face_scan,
            &height_span,
            &mut font_metrics,
            &mut face_ids,
            &mut builder,
            &mut active_face,
            &mut row_geometry,
            &mut row_extend,
            &mut box_face,
            4.0,
        ),
        0,
    );

    assert!(resolved);
    assert_eq!(active_face.face_id(), 20);
    assert_eq!(active_face.metrics().row_height, 16.0);
    assert_eq!(row_geometry.height(), 16.0);
}

#[test]
fn display_row_boundary_transition_request_records_hit_and_emits_next_row() {
    let mut ctx = RowTransitionTestContext::new("boundary-transition-request");

    let transition = DisplayRowBoundaryTransitionRequest::new(
        DisplayRowBoundaryTarget::visual_wrap(
            DisplayRowHitRange {
                charpos_start: 3,
                charpos_end: 9,
            },
            ctx.defaults,
            0,
            6,
            48.0,
            ctx.row_y_positions.recording(),
        ),
        4,
    )
    .emit(
        &mut ctx.geometry,
        &mut ctx.hit_rows,
        &mut ctx.builder,
        &mut ctx.output_emitter,
        &mut ctx.eval,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(ctx.geometry.row(), 1);
    assert_eq!(ctx.hit_rows.len(), 1);
    assert_eq!(ctx.hit_rows[0].charpos_start, 3);
    assert_eq!(ctx.hit_rows[0].charpos_end, 9);
    assert_eq!(ctx.row_y_positions.recorded(), &[0.0, 16.0]);
}

#[test]
fn display_row_line_break_transition_request_records_hit_spacing_and_emits_next_row() {
    let mut ctx = RowTransitionTestContext::new("line-break-transition-request");

    let transition = DisplayRowLineBreakTransitionRequest::new(
        DisplayRowHitRange {
            charpos_start: 3,
            charpos_end: 9,
        },
        ctx.defaults,
        0,
        6,
        48.0,
        4.0,
        ctx.row_y_positions.recording(),
        4,
    )
    .emit(
        &mut ctx.geometry,
        &mut ctx.hit_rows,
        &mut ctx.builder,
        &mut ctx.output_emitter,
        &mut ctx.eval,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(ctx.geometry.row(), 1);
    assert_eq!(ctx.hit_rows.len(), 1);
    assert_eq!(ctx.hit_rows[0].charpos_start, 3);
    assert_eq!(ctx.hit_rows[0].charpos_end, 9);
    assert_eq!(ctx.row_y_positions.recorded(), &[0.0, 20.0]);
}

#[test]
fn display_row_transition_request_context_builds_line_break_and_overflow_requests() {
    let mut line_ctx = RowTransitionTestContext::new("transition-context-line-break");

    let transition = DisplayRowTransitionRequestContext::new(
        line_ctx.defaults,
        0,
        line_ctx.row_y_positions.recording(),
        4,
    )
    .line_break(
        DisplayRowLineBreakTransitionPlan::line_break(),
        DisplayRowHitRange {
            charpos_start: 3,
            charpos_end: 9,
        },
        DisplayRowPosition { x_px: 48.0, col: 6 },
        4.0,
    )
    .emit(
        &mut line_ctx.geometry,
        &mut line_ctx.hit_rows,
        &mut line_ctx.builder,
        &mut line_ctx.output_emitter,
        &mut line_ctx.eval,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(line_ctx.geometry.row(), 1);
    assert_eq!(line_ctx.hit_rows.len(), 1);
    assert_eq!(line_ctx.hit_rows[0].charpos_start, 3);
    assert_eq!(line_ctx.hit_rows[0].charpos_end, 9);
    assert_eq!(line_ctx.row_y_positions.recorded(), &[0.0, 20.0]);

    let mut wrap_ctx = RowTransitionTestContext::new("transition-context-overflow");
    let BufferTextSourceCharOverflowAction::CharacterWrap { transition } =
        BufferTextSourceCharOverflowAction::for_decision(
            BufferTextRowOverflowDecision::CharacterWrap,
        )
    else {
        panic!("expected character wrap transition");
    };

    let transition = DisplayRowTransitionRequestContext::new(
        wrap_ctx.defaults,
        0,
        wrap_ctx.row_y_positions.recording(),
        4,
    )
    .overflow(
        transition,
        DisplayRowHitRange {
            charpos_start: 4,
            charpos_end: 10,
        },
        DisplayRowPosition { x_px: 56.0, col: 7 },
    )
    .emit(
        &mut wrap_ctx.geometry,
        &mut wrap_ctx.row_flags,
        wrap_ctx.row_limit,
        &mut wrap_ctx.hit_rows,
        &mut wrap_ctx.builder,
        &mut wrap_ctx.output_emitter,
        &mut wrap_ctx.eval,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(wrap_ctx.geometry.row(), 1);
    assert_eq!(wrap_ctx.hit_rows.len(), 1);
    assert_eq!(wrap_ctx.hit_rows[0].charpos_start, 4);
    assert_eq!(wrap_ctx.hit_rows[0].charpos_end, 10);
    assert!(wrap_ctx.row_flags.is_set(0, DisplayRowFlagKind::Continued));
    assert!(
        wrap_ctx
            .row_flags
            .is_set(1, DisplayRowFlagKind::Continuation)
    );
    assert_eq!(wrap_ctx.row_y_positions.recorded(), &[0.0, 16.0]);
}

#[test]
fn display_row_text_window_transition_context_emits_line_break_and_overflow() {
    let mut line_ctx = RowTransitionTestContext::new("text-window-transition-line-break");

    let row_limit = line_ctx.row_limit;
    let transition = DisplayRowTextWindowEmitContext::new(
        line_ctx.defaults,
        0,
        &mut line_ctx.row_y_positions,
        4,
        &mut line_ctx.geometry,
        &mut line_ctx.row_flags,
        row_limit,
        &mut line_ctx.hit_rows,
        &mut line_ctx.builder,
        &mut line_ctx.output_emitter,
        &mut line_ctx.eval,
    )
    .emit_line_break(
        DisplayRowLineBreakTransitionPlan::line_break(),
        DisplayRowHitRange {
            charpos_start: 1,
            charpos_end: 5,
        },
        DisplayRowPosition { x_px: 32.0, col: 4 },
        2.0,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(line_ctx.geometry.row(), 1);
    assert_eq!(line_ctx.hit_rows.len(), 1);
    assert_eq!(line_ctx.hit_rows[0].charpos_start, 1);
    assert_eq!(line_ctx.hit_rows[0].charpos_end, 5);
    assert_eq!(line_ctx.row_y_positions.recorded(), &[0.0, 18.0]);

    let mut overflow_ctx = RowTransitionTestContext::new("text-window-transition-overflow");
    let BufferTextSourceCharOverflowAction::CharacterWrap { transition } =
        BufferTextSourceCharOverflowAction::for_decision(
            BufferTextRowOverflowDecision::CharacterWrap,
        )
    else {
        panic!("expected character wrap transition");
    };

    let row_limit = overflow_ctx.row_limit;
    let transition = DisplayRowTextWindowEmitContext::new(
        overflow_ctx.defaults,
        0,
        &mut overflow_ctx.row_y_positions,
        4,
        &mut overflow_ctx.geometry,
        &mut overflow_ctx.row_flags,
        row_limit,
        &mut overflow_ctx.hit_rows,
        &mut overflow_ctx.builder,
        &mut overflow_ctx.output_emitter,
        &mut overflow_ctx.eval,
    )
    .emit_overflow(
        transition,
        DisplayRowHitRange {
            charpos_start: 2,
            charpos_end: 8,
        },
        DisplayRowPosition { x_px: 64.0, col: 8 },
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(overflow_ctx.geometry.row(), 1);
    assert_eq!(overflow_ctx.hit_rows.len(), 1);
    assert_eq!(overflow_ctx.hit_rows[0].charpos_start, 2);
    assert_eq!(overflow_ctx.hit_rows[0].charpos_end, 8);
    assert!(
        overflow_ctx
            .row_flags
            .is_set(0, DisplayRowFlagKind::Continued)
    );
    assert!(
        overflow_ctx
            .row_flags
            .is_set(1, DisplayRowFlagKind::Continuation)
    );
    assert_eq!(overflow_ctx.row_y_positions.recorded(), &[0.0, 16.0]);
}

#[test]
fn display_row_text_window_emit_context_applies_line_break_render_state_after_transition() {
    let mut ctx = RowTransitionTestContext::new("text-window-transition-line-state");
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut line_numbers = LineNumberRenderState::new(true, 4, 9);
    let mut hscroll_skip = HorizontalScrollSkipState::new(true, 4);
    hscroll_skip.consume_columns(4);
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    trailing_whitespace.track_rendered_char(' ', ctx.geometry.start_marker_at_x(8.0));
    let mut col = 6;

    let row_limit = ctx.row_limit;
    let transition = DisplayRowTextWindowEmitContext::new(
        ctx.defaults,
        0,
        &mut ctx.row_y_positions,
        4,
        &mut ctx.geometry,
        &mut ctx.row_flags,
        row_limit,
        &mut ctx.hit_rows,
        &mut ctx.builder,
        &mut ctx.output_emitter,
        &mut ctx.eval,
    )
    .emit_line_break_then_row_start(
        DisplayRowLineBreakTransitionPlan::hidden_line_break(),
        DisplayRowHitRange {
            charpos_start: 1,
            charpos_end: 5,
        },
        DisplayRowPosition { x_px: 32.0, col },
        2.0,
        DisplayRowTransitionRenderState::new(
            &mut prefix_request,
            true,
            &mut line_numbers,
            &mut hscroll_skip,
            &mut word_wrap,
            &mut trailing_whitespace,
        ),
        &mut col,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(col, 0);
    assert_eq!(prefix_request, DisplayRowPrefixRequest::Line);
    assert_eq!(line_numbers.current_line(), 5);
    assert!(hscroll_skip.should_skip());
    assert_eq!(
        trailing_whitespace.start_marker(),
        DisplayRowStartMarker::Inactive
    );
}

#[test]
fn display_row_text_window_emit_context_applies_overflow_render_state_after_transition() {
    let mut ctx = RowTransitionTestContext::new("text-window-transition-overflow-state");
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut line_numbers = LineNumberRenderState::new(true, 4, 9);
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    trailing_whitespace.track_rendered_char(' ', ctx.geometry.start_marker_at_x(8.0));
    let mut col = 6;
    let BufferTextSourceCharOverflowAction::CharacterWrap { transition } =
        BufferTextSourceCharOverflowAction::for_decision(
            BufferTextRowOverflowDecision::CharacterWrap,
        )
    else {
        panic!("expected character wrap transition");
    };

    let row_limit = ctx.row_limit;
    let row_transition = DisplayRowTextWindowEmitContext::new(
        ctx.defaults,
        0,
        &mut ctx.row_y_positions,
        4,
        &mut ctx.geometry,
        &mut ctx.row_flags,
        row_limit,
        &mut ctx.hit_rows,
        &mut ctx.builder,
        &mut ctx.output_emitter,
        &mut ctx.eval,
    )
    .emit_overflow_then_row_start(
        transition,
        DisplayRowHitRange {
            charpos_start: 2,
            charpos_end: 8,
        },
        DisplayRowPosition { x_px: 64.0, col },
        DisplayRowTransitionRenderState::new(
            &mut prefix_request,
            true,
            &mut line_numbers,
            &mut hscroll_skip,
            &mut word_wrap,
            &mut trailing_whitespace,
        ),
        &mut col,
    );

    assert_eq!(row_transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(col, 0);
    assert_eq!(prefix_request, DisplayRowPrefixRequest::Wrap);
    assert_eq!(line_numbers.current_line(), 4);
    assert!(!hscroll_skip.should_skip());
    assert!(!word_wrap.has_candidate());
    assert_eq!(
        trailing_whitespace.start_marker(),
        DisplayRowStartMarker::Inactive
    );
}

#[test]
fn display_row_transition_render_state_applies_row_start_line_break_policy() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut line_numbers = LineNumberRenderState::new(true, 4, 9);
    let mut hscroll_skip = HorizontalScrollSkipState::new(true, 4);
    hscroll_skip.consume_columns(4);
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    word_wrap.record_candidate(
        'a',
        0,
        4,
        2,
        (Some(LispCharPos1::new(1)), Some(LispCharPos1::new(1))),
    );
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    trailing_whitespace.track_rendered_char(' ', geometry.start_marker_at_x(8.0));
    let mut col = 7;

    DisplayRowTransitionRenderState::new(
        &mut prefix_request,
        true,
        &mut line_numbers,
        &mut hscroll_skip,
        &mut word_wrap,
        &mut trailing_whitespace,
    )
    .apply_line_break_row_start(
        DisplayRowLineBreakTransitionPlan::hidden_line_break(),
        &mut col,
    );

    assert_eq!(col, 0);
    assert_eq!(prefix_request, DisplayRowPrefixRequest::Line);
    assert_eq!(line_numbers.current_line(), 5);
    assert!(hscroll_skip.should_skip());
    assert!(!word_wrap.has_candidate());
    assert_eq!(
        trailing_whitespace.start_marker(),
        DisplayRowStartMarker::Inactive
    );
}

#[test]
fn buffer_hscroll_skip_source_char_preserves_line_break_action() {
    let mut byte_idx = 0;
    let mut charpos = 10;
    let mut hscroll_skip = HorizontalScrollSkipState::new(true, 4);

    let action = BufferHscrollSkipSourceChar::consume_from_text(
        b"\nnext",
        &mut byte_idx,
        &mut charpos,
        &mut hscroll_skip,
        8,
    )
    .expect("hscroll skip action");

    assert_eq!(
        action,
        BufferHscrollSkipAction::LineBreak {
            ch_start_byte_idx: 0,
            charpos: 11
        }
    );
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 11);
    assert!(hscroll_skip.should_skip());
}

#[test]
fn buffer_hscroll_skip_source_char_consumes_tab_to_next_stop() {
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut hscroll_skip = HorizontalScrollSkipState::new(true, 4);

    let action = BufferHscrollSkipSourceChar::consume_from_text(
        b"\tabc",
        &mut byte_idx,
        &mut charpos,
        &mut hscroll_skip,
        8,
    )
    .expect("hscroll skip action");

    assert_eq!(
        action,
        BufferHscrollSkipAction::Text {
            ch_start_byte_idx: 0,
            charpos: 1,
            show_left_truncation: true
        }
    );
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 1);
    assert!(!hscroll_skip.should_skip());
}

#[test]
fn buffer_hscroll_skip_source_char_consumes_wide_char_columns() {
    let mut byte_idx = 0;
    let mut charpos = 3;
    let mut hscroll_skip = HorizontalScrollSkipState::new(true, 2);

    let action = BufferHscrollSkipSourceChar::consume_from_text(
        "界x".as_bytes(),
        &mut byte_idx,
        &mut charpos,
        &mut hscroll_skip,
        8,
    )
    .expect("hscroll skip action");

    assert_eq!(
        action,
        BufferHscrollSkipAction::Text {
            ch_start_byte_idx: 0,
            charpos: 4,
            show_left_truncation: true
        }
    );
    assert_eq!(byte_idx, "界".len());
    assert_eq!(charpos, 4);
    assert!(!hscroll_skip.should_skip());
}

#[test]
fn buffer_hscroll_skip_source_char_keeps_marker_pending_while_still_skipping() {
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut hscroll_skip = HorizontalScrollSkipState::new(true, 3);

    let action = BufferHscrollSkipSourceChar::consume_from_text(
        b"abc",
        &mut byte_idx,
        &mut charpos,
        &mut hscroll_skip,
        8,
    )
    .expect("hscroll skip action");

    assert_eq!(
        action,
        BufferHscrollSkipAction::Text {
            ch_start_byte_idx: 0,
            charpos: 1,
            show_left_truncation: false
        }
    );
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 1);
    assert!(hscroll_skip.should_skip());
}

#[test]
fn buffer_hscroll_skip_render_request_appends_left_truncation_marker() {
    let mut context = RowTransitionTestContext::new("hscroll-render-request-marker");
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut hscroll_skip = HorizontalScrollSkipState::new(true, 4);
    let mut row_extend = DisplayRowScopedValue::inactive();
    let mut x = 0.0;
    let mut col = 0;
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut line_numbers = LineNumberRenderState::new(false, 0, 0);
    let mut word_wrap = WordWrapRenderState::new(false);
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(false, 0);
    let mut hit_row_range = HitRowRangeTracker::new(0);
    let mut cursor_info = CursorCaptureState::new();
    let mut font_metrics = None;
    let row_limit = context.row_limit;

    let continuation = BufferHscrollSkipRenderRequest::new(
        b"\tabc",
        8,
        0.0,
        &surface,
        &active_face,
        12.0,
        16.0,
        8.0,
        &face_resolver,
        99,
        false,
        context.defaults,
        0,
        4,
        row_limit,
    )
    .render_next_and_apply(BufferHscrollSkipRenderState {
        byte_idx: &mut byte_idx,
        charpos: &mut charpos,
        hscroll_skip: &mut hscroll_skip,
        row_extend: &mut row_extend,
        output_emitter: &mut context.output_emitter,
        x: &mut x,
        col: &mut col,
        prefix_request: &mut prefix_request,
        line_numbers: &mut line_numbers,
        word_wrap: &mut word_wrap,
        trailing_whitespace: &mut trailing_whitespace,
        row_geometry: &mut context.geometry,
        row_flags: &mut context.row_flags,
        hit_rows: &mut context.hit_rows,
        hit_row_range: &mut hit_row_range,
        cursor_info: &mut cursor_info,
        row_y_positions: &mut context.row_y_positions,
        builder: &mut context.builder,
        evaluator: &mut context.eval,
        font_metrics: &mut font_metrics,
    });

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 1);
    assert!(!hscroll_skip.should_skip());
    assert_eq!(x, 8.0);
    assert_eq!(col, 1);
    context
        .builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 1);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: '$' }));
            assert!(row.truncated_left);
        })
        .expect("current row");
}

#[test]
fn buffer_hscroll_skip_action_applies_line_break_transition_state() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut context = RowTransitionTestContext::new("hscroll-line-break-state");
    let action = BufferHscrollSkipAction::LineBreak {
        ch_start_byte_idx: 3,
        charpos: 12,
    };
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        geometry.current_row_marker(),
        (Color::from_pixel(0x112233), 17),
    );
    let mut x = 80.0;

    action.apply_line_break_before_row_transition(
        &mut row_extend,
        &mut context.output_emitter,
        &mut x,
        4.0,
    );

    assert_eq!(x, 4.0);
    assert_eq!(row_extend.value_on(&geometry), None);

    let mut hit_row_range = HitRowRangeTracker::new(7);
    let hit_range = action
        .line_break_hit_range(&mut hit_row_range)
        .expect("line break hit range");

    assert_eq!(hit_range.charpos_start, 7);
    assert_eq!(hit_range.charpos_end, 12);
    assert_eq!(hit_row_range.start(), 12);
}

#[test]
fn buffer_hscroll_skip_action_captures_line_break_cursor() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferHscrollSkipAction::LineBreak {
        ch_start_byte_idx: 3,
        charpos: 12,
    };
    let mut cursor = CursorCaptureState::new();

    action.capture_line_break_cursor_if_point(
        &mut cursor,
        &active_face,
        &geometry,
        12,
        32.0,
        4,
        16.0,
    );

    let captured = cursor.as_ref().expect("cursor captured");
    assert_eq!(captured.x, 32.0);
    assert_eq!(captured.byte_idx, 3);
    assert_eq!(captured.col, 4);
    assert_eq!(captured.slot_width, Some(8.0));
}

#[test]
fn buffer_hscroll_skip_action_applies_after_line_break_transition() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferHscrollSkipAction::LineBreak {
        ch_start_byte_idx: 3,
        charpos: 12,
    };
    let mut cursor = CursorCaptureState::new();

    let continuation = action.apply_after_line_break_row_transition(
        TextMatrixRowTransition::BeganNextRow,
        &mut cursor,
        &active_face,
        &geometry,
        12,
        32.0,
        4,
        16.0,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert!(cursor.as_ref().is_some());
}

#[test]
fn buffer_hscroll_skip_action_skips_after_state_when_transition_exhausted() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferHscrollSkipAction::LineBreak {
        ch_start_byte_idx: 3,
        charpos: 12,
    };
    let mut cursor = CursorCaptureState::new();

    let continuation = action.apply_after_line_break_row_transition(
        TextMatrixRowTransition::ExhaustedRows,
        &mut cursor,
        &active_face,
        &geometry,
        12,
        32.0,
        4,
        16.0,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Exhausted);
    assert!(cursor.as_ref().is_none());
}

#[test]
fn buffer_hscroll_skip_action_captures_text_cursor() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferHscrollSkipAction::Text {
        ch_start_byte_idx: 5,
        charpos: 9,
        show_left_truncation: false,
    };
    let mut cursor = CursorCaptureState::new();

    action.capture_text_cursor_if_point(&mut cursor, &active_face, &geometry, 9, 24.0, 3);

    let captured = cursor.as_ref().expect("cursor captured");
    assert_eq!(captured.x, 24.0);
    assert_eq!(captured.byte_idx, 5);
    assert_eq!(captured.col, 3);
    assert_eq!(captured.slot_width, Some(8.0));
}

#[test]
fn buffer_hscroll_skip_action_appends_left_truncation_marker_and_marks_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("hscroll-left-truncation-marker", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut x = 0.0;
    let mut col = 0;
    let mut render_state = BufferSyntheticTextRenderState::new(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &mut font_metrics,
        &face_resolver,
        &mut x,
        &mut col,
    );
    let action = BufferHscrollSkipAction::Text {
        ch_start_byte_idx: 5,
        charpos: 9,
        show_left_truncation: true,
    };

    action.append_left_truncation_marker_to_text_row_and_apply(
        BufferSyntheticTextRenderContext::new(&surface, &active_face, 0.0, 16.0, 12.0, 8.0),
        &geometry,
        &mut render_state,
        &face_resolver,
        0.0,
    );

    assert_eq!(x, 8.0);
    assert_eq!(col, 1);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 1);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: '$' }));
            assert!(row.truncated_left);
        })
        .expect("current row");
}

#[test]
fn buffer_invisible_text_scan_context_skips_when_checkpoint_not_reached() {
    let buffer_text = b"visible";
    let mut checkpoints = TextPropertyScanCheckpoints::new(5);
    let mut byte_idx = 2;
    let mut charpos = 2;
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);

    let action = BufferInvisibleTextScanContext::new(buffer_text, 7, 2, true)
        .consume_at_checkpoint(&snapshot, &mut checkpoints, &mut byte_idx, &mut charpos);

    assert_eq!(action, BufferInvisibleTextScanAction::Unchecked);
    assert_eq!(byte_idx, 2);
    assert_eq!(charpos, 2);
}

#[test]
fn buffer_invisible_text_scan_context_records_visible_boundary() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("visible hidden");
        let _ = eval
            .buffer_manager_mut()
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                EmacsByteRange::from_usize(8, 14),
                Value::symbol("invisible"),
                Value::T,
            );
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let mut checkpoints = TextPropertyScanCheckpoints::new(0);
    let mut byte_idx = 0;
    let mut charpos = 0;

    let action = BufferInvisibleTextScanContext::new("visible hidden".as_bytes(), 14, 0, true)
        .consume_at_checkpoint(&snapshot, &mut checkpoints, &mut byte_idx, &mut charpos);

    assert_eq!(
        action,
        BufferInvisibleTextScanAction::Visible { next_visible: 8 }
    );
    assert_eq!(byte_idx, 0);
    assert_eq!(charpos, 0);
    assert!(!checkpoints.should_check_invisible(7));
    assert!(checkpoints.should_check_invisible(8));
}

#[test]
fn buffer_invisible_text_scan_context_skips_hidden_region_and_reports_point() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("visible hidden visible");
        let _ = eval
            .buffer_manager_mut()
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                EmacsByteRange::from_usize(8, 14),
                Value::symbol("invisible"),
                Value::T,
            );
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let mut checkpoints = TextPropertyScanCheckpoints::new(8);
    let mut byte_idx = 8;
    let mut charpos = 8;

    let action =
        BufferInvisibleTextScanContext::new("visible hidden visible".as_bytes(), 22, 10, true)
            .consume_at_checkpoint(&snapshot, &mut checkpoints, &mut byte_idx, &mut charpos);

    let BufferInvisibleTextScanAction::Hidden(hidden) = action else {
        panic!("expected hidden region");
    };
    assert_eq!(hidden.start_byte_idx(), 8);
    assert_eq!(hidden.start_charpos(), 8);
    assert_eq!(hidden.skip_to(), 14);
    assert_eq!(hidden.next_visible(), 14);
    assert!(hidden.point_in_hidden_region());
    assert!(!hidden.ellipsis());
    assert_eq!(byte_idx, 14);
    assert_eq!(charpos, 14);
    assert!(!checkpoints.should_check_invisible(13));
    assert!(checkpoints.should_check_invisible(14));
}

#[test]
fn buffer_invisible_text_scan_context_reports_ellipsis_policy() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("folded rest");
        buffer.set_buffer_local(
            "buffer-invisibility-spec",
            Value::list(vec![Value::cons(Value::symbol("outline"), Value::T)]),
        );
        let _ = eval
            .buffer_manager_mut()
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                EmacsByteRange::from_usize(0, 6),
                Value::symbol("invisible"),
                Value::symbol("outline"),
            );
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let mut checkpoints = TextPropertyScanCheckpoints::new(0);
    let mut byte_idx = 0;
    let mut charpos = 0;

    let action = BufferInvisibleTextScanContext::new("folded rest".as_bytes(), 11, 9, true)
        .consume_at_checkpoint(&snapshot, &mut checkpoints, &mut byte_idx, &mut charpos);

    let BufferInvisibleTextScanAction::Hidden(hidden) = action else {
        panic!("expected hidden region");
    };
    assert_eq!(hidden.skip_to(), 6);
    assert!(!hidden.point_in_hidden_region());
    assert!(hidden.ellipsis());
    assert_eq!(byte_idx, 6);
    assert_eq!(charpos, 6);
}

#[test]
fn buffer_invisible_text_skip_captures_cursor_at_hidden_span_start() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(2, 24.0, 0.0, 16.0, 12.0);
    let hidden = BufferInvisibleTextSkip::new(5, 8, 14, 14, true, false);
    let mut cursor = CursorCaptureState::new();

    hidden.capture_cursor_if_point(&mut cursor, &active_face, &geometry, 40.0, 5);

    let captured = cursor.as_ref().expect("cursor captured");
    assert_eq!(captured.x, 40.0);
    assert_eq!(captured.y, 24.0);
    assert_eq!(captured.byte_idx, 5);
    assert_eq!(captured.col, 5);
    assert_eq!(captured.matrix_row, 2);
    assert_eq!(captured.slot_width, Some(8.0));
}

#[test]
fn buffer_invisible_text_skip_keeps_cursor_missing_when_point_is_visible() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(2, 24.0, 0.0, 16.0, 12.0);
    let hidden = BufferInvisibleTextSkip::new(5, 8, 14, 14, false, false);
    let mut cursor = CursorCaptureState::new();

    hidden.capture_cursor_if_point(&mut cursor, &active_face, &geometry, 40.0, 5);

    assert!(cursor.as_ref().is_none());
}

#[test]
fn buffer_invisible_text_skip_builds_active_ellipsis_request() {
    let hidden = BufferInvisibleTextSkip::new(5, 8, 14, 14, false, true);
    let position = DisplayRowPosition { x_px: 16.0, col: 2 };

    let request = hidden
        .ellipsis_append_request(position)
        .expect("ellipsis request");
    let (request_position, source, face) = request.into_parts();

    assert_eq!(request_position, position);
    assert_eq!(source.source_id(), SYNTHETIC_SOURCE_INVISIBLE_ELLIPSIS);
    assert_eq!(source.into_text().as_ref(), "...");
    assert!(matches!(face, SyntheticTextAppendFace::ActiveFace));
}

#[test]
fn buffer_invisible_text_skip_omits_ellipsis_request_without_policy() {
    let hidden = BufferInvisibleTextSkip::new(5, 8, 14, 14, false, false);

    assert!(
        hidden
            .ellipsis_append_request(DisplayRowPosition { x_px: 16.0, col: 2 })
            .is_none()
    );
}

#[test]
fn buffer_invisible_text_render_request_appends_ellipsis_and_captures_cursor() {
    let mut context = RowTransitionTestContext::new("invisible-text-render-request");
    let buf_id = context
        .eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = context
            .eval
            .buffer_manager_mut()
            .get_mut(buf_id)
            .expect("buffer");
        buffer.insert("folded rest");
        buffer.set_buffer_local(
            "buffer-invisibility-spec",
            Value::list(vec![Value::cons(Value::symbol("outline"), Value::T)]),
        );
        let _ = context
            .eval
            .buffer_manager_mut()
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                EmacsByteRange::from_usize(0, 6),
                Value::symbol("invisible"),
                Value::symbol("outline"),
            );
    }
    let snapshot = current_buffer_snapshot(&context.eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let overlay_context =
        BufferOverlayStringTextRowRenderContext::new(false, 1, &surface, 16.0, 12.0, 0.0, 0, 4);
    let mut checkpoints = TextPropertyScanCheckpoints::new(0);
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut x = 0.0;
    let mut col = 0;
    let mut cursor_info = CursorCaptureState::new();
    let mut hit_row_range = HitRowRangeTracker::new(0);
    let mut face_ids = FrameFaceIdAllocator::new(7);
    let mut font_metrics = None;

    let outcome = BufferInvisibleTextRenderRequest::new(
        b"folded rest",
        11,
        2,
        &surface,
        overlay_context,
        &active_face,
        0.0,
        12.0,
        16.0,
        8.0,
    )
    .render_at_checkpoint_and_apply(
        &snapshot,
        BufferInvisibleTextRenderRequestState {
            checkpoints: &mut checkpoints,
            byte_idx: &mut byte_idx,
            charpos: &mut charpos,
            output_emitter: &mut context.output_emitter,
            x: &mut x,
            col: &mut col,
            row_geometry: &mut context.geometry,
            cursor_info: &mut cursor_info,
            hit_rows: &mut context.hit_rows,
            hit_row_range: &mut hit_row_range,
            row_y_positions: &mut context.row_y_positions,
            face_ids: &mut face_ids,
            builder: &mut context.builder,
            evaluator: &mut context.eval,
            font_metrics: &mut font_metrics,
            face_resolver: &face_resolver,
        },
    );

    assert_eq!(
        outcome,
        BufferInvisibleTextRenderOutcome::ContinueBufferWalk
    );
    assert_eq!(byte_idx, 6);
    assert_eq!(charpos, 6);
    assert_eq!(x, 24.0);
    assert_eq!(col, 3);
    assert!(cursor_info.captured().is_some());
    context
        .builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 3);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: '.' }));
            assert!(matches!(text[1].glyph_type, GlyphType::Char { ch: '.' }));
            assert!(matches!(text[2].glyph_type, GlyphType::Char { ch: '.' }));
        })
        .expect("current row");
}

#[test]
fn buffer_selective_display_context_skips_carriage_return_tail_to_newline() {
    let text = b"a\rb\nc";
    let context = BufferSelectiveDisplayContext::new(text, 1, 8);
    let mut byte_idx = 2;
    let mut charpos = 1;

    assert!(context.hides_carriage_return_tail('\r'));
    let action = context.skip_rest_of_line_after_carriage_return(&mut byte_idx, &mut charpos);

    assert_eq!(
        action,
        BufferSelectiveDisplayLineTailAction::LineBreak { charpos: 4 }
    );
    assert!(action.is_line_break());
    assert_eq!(action.charpos(), Some(4));
    assert_eq!(byte_idx, 4);
    assert_eq!(charpos, 4);
}

#[test]
fn buffer_selective_display_context_reports_carriage_return_tail_marker() {
    let context = BufferSelectiveDisplayContext::new(b"a\rb", 1, 8);

    assert_eq!(
        context.carriage_return_tail_marker('\r'),
        Some(BufferSelectiveDisplayLineTailMarker)
    );
    assert_eq!(context.carriage_return_tail_marker('x'), None);
}

#[test]
fn buffer_selective_display_line_tail_marker_builds_active_ellipsis_request() {
    let marker = BufferSelectiveDisplayLineTailMarker;
    let position = DisplayRowPosition { x_px: 24.0, col: 3 };

    let request = marker.ellipsis_append_request(position);
    let (request_position, source, face) = request.into_parts();

    assert_eq!(request_position, position);
    assert_eq!(source.source_id(), SYNTHETIC_SOURCE_SELECTIVE_ELLIPSIS);
    assert_eq!(source.into_text().as_ref(), "...");
    assert!(matches!(face, SyntheticTextAppendFace::ActiveFace));
}

#[test]
fn buffer_selective_display_context_reports_exhausted_carriage_return_tail() {
    let text = b"a\rhidden";
    let context = BufferSelectiveDisplayContext::new(text, 1, 8);
    let mut byte_idx = 2;
    let mut charpos = 1;

    let action = context.skip_rest_of_line_after_carriage_return(&mut byte_idx, &mut charpos);

    assert_eq!(action, BufferSelectiveDisplayLineTailAction::Exhausted);
    assert!(!action.is_line_break());
    assert_eq!(action.charpos(), None);
    assert_eq!(byte_idx, text.len());
    assert_eq!(charpos, 8);
}

#[test]
fn buffer_selective_display_context_skips_hidden_indented_lines() {
    let text = b"  hidden\n\talso\n visible\n";
    let context = BufferSelectiveDisplayContext::new(text, 1, 4);
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut line_numbers = LineNumberRenderState::new(true, 7, 9);

    assert!(context.hides_indented_lines_after_line_break(byte_idx));
    let hidden_lines =
        context.skip_hidden_indented_lines_after_line_break(&mut byte_idx, &mut charpos);
    hidden_lines.apply_to_line_numbers(&mut line_numbers);

    assert_eq!(hidden_lines.hidden_line_count(), 2);
    assert_eq!(byte_idx, b"  hidden\n\talso\n".len());
    assert_eq!(charpos, b"  hidden\n\talso\n".len() as i64);
    assert_eq!(line_numbers.current_line(), 9);
}

#[test]
fn buffer_selective_display_context_applies_hidden_indented_lines_after_line_break() {
    let text = b"  hidden\n\talso\n visible\n";
    let context = BufferSelectiveDisplayContext::new(text, 1, 4);
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut line_numbers = LineNumberRenderState::new(true, 7, 9);

    let hidden_lines = context.apply_hidden_indented_lines_after_line_break(
        &mut byte_idx,
        &mut charpos,
        &mut line_numbers,
    );

    assert_eq!(hidden_lines.hidden_line_count(), 2);
    assert_eq!(byte_idx, b"  hidden\n\talso\n".len());
    assert_eq!(charpos, b"  hidden\n\talso\n".len() as i64);
    assert_eq!(line_numbers.current_line(), 9);
}

#[test]
fn buffer_selective_display_context_apply_hidden_indented_lines_noops_when_disabled() {
    let text = b"  visible\n";
    let context = BufferSelectiveDisplayContext::new(text, 0, 4);
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut line_numbers = LineNumberRenderState::new(true, 7, 9);

    let hidden_lines = context.apply_hidden_indented_lines_after_line_break(
        &mut byte_idx,
        &mut charpos,
        &mut line_numbers,
    );

    assert_eq!(hidden_lines.hidden_line_count(), 0);
    assert_eq!(line_numbers.current_line(), 7);
    assert_eq!(byte_idx, 0);
    assert_eq!(charpos, 0);
}

#[test]
fn buffer_selective_display_context_keeps_visible_indented_line() {
    let text = b" visible\n";
    let context = BufferSelectiveDisplayContext::new(text, 1, 4);
    let mut byte_idx = 0;
    let mut charpos = 0;

    let hidden_lines =
        context.skip_hidden_indented_lines_after_line_break(&mut byte_idx, &mut charpos);

    assert_eq!(hidden_lines.hidden_line_count(), 0);
    assert_eq!(byte_idx, 0);
    assert_eq!(charpos, 0);
}

#[test]
fn buffer_text_decoded_source_char_consumes_multibyte_source_coordinates() {
    let text = "a界b".as_bytes();
    let mut byte_idx = "a".len();
    let charpos = 4;

    let source_char = BufferTextDecodedSourceChar::consume_from_text(text, &mut byte_idx, charpos)
        .expect("decoded source char");

    assert_eq!(source_char.ch(), '界');
    assert_eq!(source_char.start_byte_idx(), 1);
    assert_eq!(source_char.start_charpos(), 4);
    assert_eq!(byte_idx, "a界".len());
}

#[test]
fn buffer_text_decoded_source_char_records_word_wrap_candidate() {
    let context = RowTransitionTestContext::new("decoded-source-char-word-wrap");
    let text = b" a";
    let mut byte_idx = 1;
    let source_char = BufferTextDecodedSourceChar::consume_from_text(text, &mut byte_idx, 6)
        .expect("decoded source char");
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');

    source_char.record_word_wrap_candidate(&mut word_wrap, &context.output_emitter);

    let candidate = word_wrap.candidate();
    assert!(candidate.is_available());
    assert_eq!(candidate.byte_idx(), 1);
    assert_eq!(candidate.charpos(), 6);
    assert_eq!(candidate.display_point_count(), 0);
}

#[test]
fn buffer_text_decoded_source_char_builds_line_break_action() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("a\nb");
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let mut byte_idx = 1;
    let source_char =
        BufferTextDecodedSourceChar::consume_from_text("a\nb".as_bytes(), &mut byte_idx, 1)
            .expect("decoded newline");

    let action =
        BufferTextLineBreakSourceAction::for_decoded_newline(&snapshot, source_char, 16.0, 5.0);

    assert_eq!(source_char.ch(), '\n');
    assert!(action.point_matches(1));
    assert_eq!(action.next_charpos(), 2);
    assert_eq!(action.line_spacing(), 5.0);
}

#[test]
fn buffer_text_line_break_source_action_uses_extra_line_spacing() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("a\nb");
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);

    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 1, 1, 16.0, 5.0);

    assert!(action.point_matches(1));
    assert!(!action.point_matches(2));
    assert_eq!(action.next_charpos(), 2);
    assert_eq!(action.line_spacing(), 5.0);
}

#[test]
fn buffer_text_line_break_source_action_prefers_text_property_spacing() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("a\nb");
        let _ = eval
            .buffer_manager_mut()
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                EmacsByteRange::from_usize(1, 2),
                Value::symbol("line-spacing"),
                Value::fixnum(7),
            );
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);

    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 1, 1, 16.0, 5.0);

    assert_eq!(action.next_charpos(), 2);
    assert_eq!(action.line_spacing(), 7.0);
}

#[test]
fn buffer_text_line_break_source_action_builds_row_end_cursor_info() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);

    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 4, 12, 16.0, 0.0);
    let cursor = action.cursor_info(&active_face, &geometry, 32.0, 4);

    assert_eq!(cursor.x, 32.0);
    assert_eq!(cursor.byte_idx, 12);
    assert_eq!(cursor.col, 4);
    assert_eq!(cursor.slot_width, Some(8.0));
    assert!(!cursor.stretch_like);
}

#[test]
fn buffer_text_line_break_source_action_captures_cursor_when_point_matches() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 4, 12, 16.0, 0.0);
    let mut cursor = CursorCaptureState::new();

    action.capture_cursor_if_point(&mut cursor, &active_face, &geometry, 4, 32.0, 4);

    let captured = cursor.as_ref().expect("cursor captured");
    assert_eq!(captured.x, 32.0);
    assert_eq!(captured.byte_idx, 12);
    assert_eq!(captured.col, 4);
    assert_eq!(captured.slot_width, Some(8.0));
}

#[test]
fn buffer_text_line_break_source_action_keeps_cursor_missing_when_point_differs() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 4, 12, 16.0, 0.0);
    let mut cursor = CursorCaptureState::new();

    action.capture_cursor_if_point(&mut cursor, &active_face, &geometry, 5, 32.0, 4);

    assert!(cursor.as_ref().is_none());
}

#[test]
fn buffer_text_line_break_source_action_applies_row_transition_state() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let mut context = RowTransitionTestContext::new("line-break-source-state");
    let geometry = context.geometry;
    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 4, 12, 16.0, 0.0);
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    trailing_whitespace.track_rendered_char(' ', geometry.start_marker_at_x(24.0));
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        geometry.current_row_marker(),
        (Color::from_pixel(0x112233), 17),
    );
    let mut box_face = BoxFaceRowState::inactive();
    box_face.activate(geometry.current_row_marker(), 8.0);
    let mut x = 40.0;
    let mut charpos = 4;

    action.apply_before_row_transition(
        &geometry,
        &mut trailing_whitespace,
        &mut row_extend,
        &mut box_face,
        &mut context.output_emitter,
        2.0,
        &mut x,
        &mut charpos,
    );

    assert_eq!(x, 2.0);
    assert_eq!(charpos, 5);
    assert_eq!(trailing_whitespace.highlight_start_x(&geometry), None);
    assert_eq!(row_extend.value_on(&geometry), None);
    assert_eq!(box_face.row(), geometry.current_row_marker());
    assert_eq!(box_face.start_x(), Some(2.0));
}

#[test]
fn buffer_text_line_break_source_action_syncs_after_transition() {
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(3);

    BufferTextLineBreakSourceAction::sync_after_row_transition(
        14,
        &mut charpos,
        &mut hit_row_range,
    );

    assert_eq!(charpos, 14);
    assert_eq!(hit_row_range.start(), 14);
}

#[test]
fn buffer_text_line_break_source_action_applies_after_transition() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let geometry = DisplayRowGeometryState::new(1, 16.0, 0.0, 16.0, 12.0);
    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 4, 12, 16.0, 0.0);
    let mut box_face = BoxFaceRowState::inactive();
    box_face.activate(geometry.current_row_marker(), 8.0);
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(3);

    let continuation = action.apply_after_line_break_row_transition(
        TextMatrixRowTransition::BeganNextRow,
        14,
        &mut charpos,
        &mut hit_row_range,
        &geometry,
        &mut box_face,
        2.0,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert_eq!(charpos, 14);
    assert_eq!(hit_row_range.start(), 14);
    assert_eq!(box_face.row(), geometry.current_row_marker());
    assert_eq!(box_face.start_x(), Some(2.0));
}

#[test]
fn buffer_text_line_break_source_action_skips_after_state_when_transition_exhausted() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let geometry = DisplayRowGeometryState::new(1, 16.0, 0.0, 16.0, 12.0);
    let action = BufferTextLineBreakSourceAction::for_newline(&snapshot, 4, 12, 16.0, 0.0);
    let mut box_face = BoxFaceRowState::inactive();
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(3);

    let continuation = action.apply_after_line_break_row_transition(
        TextMatrixRowTransition::ExhaustedRows,
        14,
        &mut charpos,
        &mut hit_row_range,
        &geometry,
        &mut box_face,
        2.0,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Exhausted);
    assert_eq!(charpos, 9);
    assert_eq!(hit_row_range.start(), 3);
    assert_eq!(box_face.start_x(), None);
}

#[test]
fn buffer_text_line_break_render_request_emits_row_transition_and_syncs_position() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("\nnext");
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let mut context = RowTransitionTestContext::new("line-break-render-request");
    let active_face = test_active_face_state(9, 8.0);
    let text = b"\nnext";
    let mut byte_idx = 0;
    let source_char = BufferTextDecodedSourceChar::consume_from_text(text, &mut byte_idx, 0)
        .expect("decoded newline");
    let mut charpos = 0;
    let mut cursor_info = CursorCaptureState::new();
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    trailing_whitespace.track_rendered_char(' ', context.geometry.start_marker_at_x(24.0));
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        context.geometry.current_row_marker(),
        (Color::from_pixel(0x112233), 17),
    );
    let mut box_face = BoxFaceRowState::inactive();
    box_face.activate(context.geometry.current_row_marker(), 8.0);
    let mut x = 40.0;
    let mut col = 5;
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut line_numbers = LineNumberRenderState::new(true, 1, 0);
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut word_wrap = WordWrapRenderState::new(false);
    let mut hit_row_range = HitRowRangeTracker::new(0);
    let row_limit = context.row_limit;

    let continuation = BufferTextLineBreakRenderRequest::new(
        source_char,
        text,
        0,
        0,
        8,
        &active_face,
        0,
        16.0,
        0.0,
        0.0,
        false,
        context.defaults,
        0,
        4,
        row_limit,
    )
    .render_and_apply(
        &snapshot,
        BufferTextLineBreakRenderState {
            byte_idx: &mut byte_idx,
            charpos: &mut charpos,
            cursor_info: &mut cursor_info,
            row_geometry: &mut context.geometry,
            trailing_whitespace: &mut trailing_whitespace,
            row_extend: &mut row_extend,
            box_face: &mut box_face,
            output_emitter: &mut context.output_emitter,
            x: &mut x,
            col: &mut col,
            prefix_request: &mut prefix_request,
            line_numbers: &mut line_numbers,
            hscroll_skip: &mut hscroll_skip,
            word_wrap: &mut word_wrap,
            row_flags: &mut context.row_flags,
            hit_rows: &mut context.hit_rows,
            hit_row_range: &mut hit_row_range,
            row_y_positions: &mut context.row_y_positions,
            builder: &mut context.builder,
            evaluator: &mut context.eval,
        },
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 1);
    assert_eq!(x, 0.0);
    assert_eq!(col, 0);
    assert_eq!(hit_row_range.start(), 1);
    assert_eq!(context.row_y_positions.recorded(), &[0.0, 16.0]);
    assert_eq!(
        trailing_whitespace.highlight_start_x(&context.geometry),
        None
    );
    assert_eq!(row_extend.value_on(&context.geometry), None);
    assert_eq!(box_face.row(), context.geometry.current_row_marker());
    assert_eq!(box_face.start_x(), Some(0.0));
    assert!(cursor_info.as_ref().is_some());
}

#[test]
fn buffer_selective_display_line_tail_action_syncs_after_hidden_line_break_transition() {
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(3);

    BufferSelectiveDisplayLineTailAction::sync_after_hidden_line_break_transition(
        14,
        &mut charpos,
        &mut hit_row_range,
    );

    assert_eq!(charpos, 14);
    assert_eq!(hit_row_range.start(), 14);
}

#[test]
fn buffer_selective_display_line_tail_action_applies_after_hidden_line_break_transition() {
    let action = BufferSelectiveDisplayLineTailAction::LineBreak { charpos: 12 };
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(3);

    let continuation = action.apply_after_hidden_line_break_transition(
        TextMatrixRowTransition::BeganNextRow,
        14,
        &mut charpos,
        &mut hit_row_range,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert_eq!(charpos, 14);
    assert_eq!(hit_row_range.start(), 14);
}

#[test]
fn buffer_selective_display_line_tail_action_skips_after_state_when_transition_exhausted() {
    let action = BufferSelectiveDisplayLineTailAction::LineBreak { charpos: 12 };
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(3);

    let continuation = action.apply_after_hidden_line_break_transition(
        TextMatrixRowTransition::ExhaustedRows,
        14,
        &mut charpos,
        &mut hit_row_range,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Exhausted);
    assert_eq!(charpos, 9);
    assert_eq!(hit_row_range.start(), 3);
}

#[test]
fn buffer_selective_display_tail_render_request_appends_marker_and_transitions_row() {
    let mut context = RowTransitionTestContext::new("selective-display-tail-request");
    let buf_id = context
        .eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = context
            .eval
            .buffer_manager_mut()
            .get_mut(buf_id)
            .expect("buffer");
        buffer.insert("a\rb\nc");
    }
    let snapshot = current_buffer_snapshot(&context.eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let text = b"a\rb\nc";
    let mut byte_idx = 1;
    let decoded_source_char =
        BufferTextDecodedSourceChar::consume_from_text(text, &mut byte_idx, 1)
            .expect("decoded carriage return");
    let mut charpos = 1;
    let mut col = 0;
    let mut row_extend = DisplayRowScopedValue::inactive();
    let mut box_face = BoxFaceRowState::inactive();
    let mut x = 0.0;
    let mut line_numbers = LineNumberRenderState::new(false, 0, 0);
    let mut hit_row_range = HitRowRangeTracker::new(1);
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut word_wrap = WordWrapRenderState::new(false);
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(false, 0);
    let mut font_metrics = None;

    let outcome = BufferSelectiveDisplayTailRenderRequest::new(
        decoded_source_char,
        text,
        0,
        1,
        8,
        &surface,
        &active_face,
        0.0,
        12.0,
        16.0,
        8.0,
        0.0,
        false,
        context.defaults,
        0,
        4,
        context.row_limit,
    )
    .render_if_needed_and_apply(
        &snapshot,
        BufferSelectiveDisplayTailRenderState {
            byte_idx: &mut byte_idx,
            charpos: &mut charpos,
            col: &mut col,
            output_emitter: &mut context.output_emitter,
            row_extend: &mut row_extend,
            box_face: &mut box_face,
            x: &mut x,
            line_numbers: &mut line_numbers,
            row_geometry: &mut context.geometry,
            row_flags: &mut context.row_flags,
            hit_rows: &mut context.hit_rows,
            hit_row_range: &mut hit_row_range,
            builder: &mut context.builder,
            evaluator: &mut context.eval,
            prefix_request: &mut prefix_request,
            hscroll_skip: &mut hscroll_skip,
            word_wrap: &mut word_wrap,
            trailing_whitespace: &mut trailing_whitespace,
            row_y_positions: &mut context.row_y_positions,
            font_metrics: &mut font_metrics,
            face_resolver: &face_resolver,
        },
    );

    assert_eq!(
        outcome,
        BufferSelectiveDisplayTailRenderOutcome::ContinueBufferWalk
    );
    assert_eq!(byte_idx, 4);
    assert_eq!(charpos, 4);
    assert_eq!(hit_row_range.start(), 4);
    assert_eq!(context.geometry.row(), 1);
    assert_eq!(x, 0.0);
    assert_eq!(col, 0);
    assert_eq!(context.hit_rows.len(), 1);
    assert_eq!(context.hit_rows[0].charpos_start, 1);
    assert_eq!(context.hit_rows[0].charpos_end, 4);
}

#[test]
fn buffer_text_truncation_skip_action_consumes_decoded_char_and_reaches_newline() {
    let text = b"abc\nnext";
    let mut byte_idx = 1;
    let mut charpos = 0;

    let action = BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
        text,
        &mut byte_idx,
        &mut charpos,
    );

    assert!(action.reached_line_break());
    assert_eq!(action.charpos(), 4);
    assert_eq!(byte_idx, 4);
    assert_eq!(charpos, 4);
}

#[test]
fn buffer_text_truncation_skip_action_consumes_to_text_end_without_newline() {
    let text = b"abc";
    let mut byte_idx = 1;
    let mut charpos = 0;

    let action = BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
        text,
        &mut byte_idx,
        &mut charpos,
    );

    assert!(!action.reached_line_break());
    assert_eq!(action.charpos(), 3);
    assert_eq!(byte_idx, 3);
    assert_eq!(charpos, 3);
}

#[test]
fn buffer_text_truncation_skip_action_counts_multibyte_chars() {
    let text = "a界b\n".as_bytes();
    let mut byte_idx = "a".len();
    let mut charpos = 0;

    let action = BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
        text,
        &mut byte_idx,
        &mut charpos,
    );

    assert!(action.reached_line_break());
    assert_eq!(action.charpos(), 4);
    assert_eq!(byte_idx, text.len());
    assert_eq!(charpos, 4);
}

#[test]
fn buffer_text_truncation_skip_action_applies_transition_state() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let text = b"abc\nnext";
    let mut byte_idx = 1;
    let mut charpos = 0;
    let action = BufferTextTruncationSkipAction::consume_decoded_char_and_rest_of_line(
        text,
        &mut byte_idx,
        &mut charpos,
    );
    let mut line_numbers = LineNumberRenderState::new(true, 5, 8);
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        geometry.current_row_marker(),
        (Color::from_pixel(0x112233), 17),
    );
    let mut x = 80.0;

    action.apply_before_row_transition(&mut line_numbers, &mut row_extend, &mut x, 3.0);

    assert_eq!(line_numbers.current_line(), 6);
    assert_eq!(x, 3.0);
    assert_eq!(row_extend.value_on(&geometry), None);
}

#[test]
fn buffer_text_truncation_skip_action_syncs_after_transition() {
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(2);

    BufferTextTruncationSkipAction::sync_after_row_transition(14, &mut charpos, &mut hit_row_range);

    assert_eq!(charpos, 14);
    assert_eq!(hit_row_range.start(), 14);
}

#[test]
fn buffer_text_truncation_skip_action_reports_transition_continuation() {
    let action = BufferTextTruncationSkipAction {
        charpos: 12,
        reached_line_break: false,
    };

    assert_eq!(
        action.transition_continuation(TextMatrixRowTransition::BeganNextRow),
        DisplayRowTransitionContinuation::Continue
    );
    assert_eq!(
        action.transition_continuation(TextMatrixRowTransition::ExhaustedRows),
        DisplayRowTransitionContinuation::Exhausted
    );
}

#[test]
fn buffer_text_truncation_skip_action_syncs_after_visible_transition() {
    let action = BufferTextTruncationSkipAction {
        charpos: 12,
        reached_line_break: true,
    };
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(2);

    let continuation = action.sync_after_row_transition_if_visible(
        TextMatrixRowTransition::BeganNextRow,
        14,
        &mut charpos,
        &mut hit_row_range,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert_eq!(charpos, 14);
    assert_eq!(hit_row_range.start(), 14);
}

#[test]
fn buffer_text_truncation_skip_action_skips_sync_when_transition_exhausted() {
    let action = BufferTextTruncationSkipAction {
        charpos: 12,
        reached_line_break: true,
    };
    let mut charpos = 9;
    let mut hit_row_range = HitRowRangeTracker::new(2);

    let continuation = action.sync_after_row_transition_if_visible(
        TextMatrixRowTransition::ExhaustedRows,
        14,
        &mut charpos,
        &mut hit_row_range,
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Exhausted);
    assert_eq!(charpos, 9);
    assert_eq!(hit_row_range.start(), 2);
}

#[test]
fn buffer_text_word_wrap_source_action_rewinds_source_state() {
    let mut break_candidate = WordWrapBreakCandidate::default();
    break_candidate.record(
        7,
        12,
        3,
        (Some(LispCharPos1::new(10)), Some(LispCharPos1::new(12))),
    );
    let action = BufferTextWordWrapSourceAction::new(break_candidate);
    let mut byte_idx = 20;
    let mut charpos = 30;
    let mut col = 9;

    action.rewind_source_state(&mut byte_idx, &mut charpos, &mut col);

    assert_eq!(action.byte_idx(), 7);
    assert_eq!(action.charpos(), 12);
    assert_eq!(byte_idx, 7);
    assert_eq!(charpos, 12);
    assert_eq!(col, 0);
}

#[test]
fn buffer_text_word_wrap_source_action_applies_transition_state() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut context = RowTransitionTestContext::new("word-wrap-source-state");
    let mut break_candidate = WordWrapBreakCandidate::default();
    break_candidate.record(
        7,
        12,
        3,
        (Some(LispCharPos1::new(10)), Some(LispCharPos1::new(12))),
    );
    let action = BufferTextWordWrapSourceAction::new(break_candidate);
    let mut byte_idx = 20;
    let mut charpos = 30;
    let mut col = 9;
    let mut x = 88.0;
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        geometry.current_row_marker(),
        (Color::from_pixel(0x112233), 17),
    );

    action.apply_before_row_transition(
        &mut context.output_emitter,
        &mut byte_idx,
        &mut charpos,
        &mut col,
        &mut row_extend,
        &mut x,
        2.0,
    );

    assert_eq!(byte_idx, 7);
    assert_eq!(charpos, 12);
    assert_eq!(col, 0);
    assert_eq!(x, 2.0);
    assert_eq!(row_extend.value_on(&geometry), None);

    let mut hit_row_range = HitRowRangeTracker::new(4);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;
    let mut final_charpos = 30;
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut line_numbers = LineNumberRenderState::new(true, 4, 9);
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut wrap_state = WordWrapRenderState::new(true);
    wrap_state.allow_after_current_char(' ');
    wrap_state.record_candidate(
        'a',
        0,
        4,
        2,
        (Some(LispCharPos1::new(1)), Some(LispCharPos1::new(1))),
    );
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    trailing_whitespace.track_rendered_char(' ', geometry.start_marker_at_x(8.0));
    let BufferTextSourceCharOverflowAction::WordWrap { transition, .. } =
        BufferTextSourceCharOverflowAction::for_decision(BufferTextRowOverflowDecision::WordWrap {
            break_candidate,
        })
    else {
        panic!("expected word wrap transition");
    };

    let continuation = action.apply_after_row_transition_and_prefix(
        TextMatrixRowTransition::BeganNextRow,
        transition,
        &mut final_charpos,
        &mut hit_row_range,
        &mut face_scan,
        &geometry,
        DisplayRowVisibilityLimit {
            max_rows: 2,
            bottom_y: 64.0,
        },
        DisplayRowTransitionRenderState::new(
            &mut prefix_request,
            true,
            &mut line_numbers,
            &mut hscroll_skip,
            &mut wrap_state,
            &mut trailing_whitespace,
        ),
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert_eq!(final_charpos, 12);
    assert_eq!(hit_row_range.start(), 12);
    assert!(face_scan.should_resolve_at(0));
    assert_eq!(prefix_request, DisplayRowPrefixRequest::Wrap);
    assert!(!wrap_state.has_candidate());
    assert_eq!(
        trailing_whitespace.start_marker(),
        DisplayRowStartMarker::Inactive
    );
}

#[test]
fn buffer_text_special_wrap_source_action_applies_transition_state() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferTextSpecialWrapSourceAction::new(21);
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        geometry.current_row_marker(),
        (Color::from_pixel(0x445566), 21),
    );
    let mut x = 88.0;

    action.apply_before_row_transition(&mut row_extend, &mut x, 3.0);

    assert_eq!(action.charpos(), 21);
    assert_eq!(x, 3.0);
    assert_eq!(row_extend.value_on(&geometry), None);

    let mut hit_row_range = HitRowRangeTracker::new(6);
    let hit_range = action.hit_range_and_advance(&mut hit_row_range);

    assert_eq!(hit_range.charpos_start, 6);
    assert_eq!(hit_range.charpos_end, 21);
    assert_eq!(hit_row_range.start(), 21);
    assert_eq!(
        action.transition_continuation(
            TextMatrixRowTransition::BeganNextRow,
            &geometry,
            DisplayRowVisibilityLimit {
                max_rows: 2,
                bottom_y: 64.0,
            },
        ),
        DisplayRowTransitionContinuation::Continue
    );
}

#[test]
fn buffer_text_special_overflow_render_request_wraps_then_keeps_prepared_append() {
    let mut context = RowTransitionTestContext::new("special-overflow-wrap-request");
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("a");
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let prepared_append = BufferTextSpecialSourceCharPreparedAppend {
        kind: BufferTextSourceSpecialDisplayKind::Control,
        append_plan: BufferTextSpecialSourceCharAppendPlan {
            source_item: BufferTextSourceItemRequest::new(
                BufferTextSourceRange::single_char(CharPos0::new(21)),
                BufferTextSourceAppendItem::ControlChar { ch: '\n' },
            ),
            position: DisplayRowPosition {
                x_px: 80.0,
                col: 10,
            },
        },
        measured_width_px: Some(8.0),
    };
    let text = b"a";
    let mut byte_idx = 0;
    let mut charpos = 21;
    let mut col = 10;
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        context.geometry.current_row_marker(),
        (Color::from_pixel(0x445566), 21),
    );
    let mut x = 80.0;
    let mut line_numbers = LineNumberRenderState::new(false, 0, 0);
    let mut hit_row_range = HitRowRangeTracker::new(6);
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut word_wrap = WordWrapRenderState::new(false);
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(false, 0);
    let row_limit = context.row_limit;

    let outcome = BufferTextSpecialOverflowRenderRequest::new(
        &prepared_append,
        text,
        0,
        x,
        80.0,
        false,
        DisplayRowVisibilityLimit {
            max_rows: 4,
            bottom_y: 64.0,
        },
        0.0,
        false,
        context.defaults,
        0,
        4,
        row_limit,
    )
    .render_if_needed_and_apply(
        &snapshot,
        BufferTextSpecialOverflowRenderState {
            byte_idx: &mut byte_idx,
            charpos: &mut charpos,
            col: &mut col,
            output_emitter: &mut context.output_emitter,
            row_extend: &mut row_extend,
            x: &mut x,
            line_numbers: &mut line_numbers,
            row_geometry: &mut context.geometry,
            row_flags: &mut context.row_flags,
            hit_rows: &mut context.hit_rows,
            hit_row_range: &mut hit_row_range,
            builder: &mut context.builder,
            evaluator: &mut context.eval,
            prefix_request: &mut prefix_request,
            hscroll_skip: &mut hscroll_skip,
            word_wrap: &mut word_wrap,
            trailing_whitespace: &mut trailing_whitespace,
            row_y_positions: &mut context.row_y_positions,
        },
    );

    assert_eq!(
        outcome,
        BufferTextSpecialOverflowRenderOutcome::AppendPrepared(
            DisplayRowTransitionContinuation::Continue
        )
    );
    assert_eq!(byte_idx, 0);
    assert_eq!(charpos, 21);
    assert_eq!(hit_row_range.start(), 21);
    assert_eq!(x, 0.0);
    assert_eq!(col, 0);
    assert_eq!(row_extend.value_on(&context.geometry), None);
}

#[test]
fn buffer_text_character_wrap_source_action_rewinds_to_current_char_start() {
    let action = BufferTextCharacterWrapSourceAction::new(13, 21);
    let mut byte_idx = 17;
    let mut charpos = 22;

    action.rewind_source_state(&mut byte_idx, &mut charpos);

    assert_eq!(byte_idx, 13);
    assert_eq!(charpos, 21);
}

#[test]
fn buffer_text_character_wrap_source_action_rewinds_decoded_source_char() {
    let text = "a界b".as_bytes();
    let mut byte_idx = "a".len();
    let source_char = BufferTextDecodedSourceChar::consume_from_text(text, &mut byte_idx, 9)
        .expect("decoded source char");
    let action = BufferTextCharacterWrapSourceAction::from_decoded_char(source_char);
    let mut rewind_byte_idx = byte_idx;
    let mut rewind_charpos = 10;

    action.rewind_source_state(&mut rewind_byte_idx, &mut rewind_charpos);

    assert_eq!(rewind_byte_idx, "a".len());
    assert_eq!(rewind_charpos, 9);
}

#[test]
fn buffer_text_character_wrap_source_action_applies_transition_state() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferTextCharacterWrapSourceAction::new(13, 21);
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        geometry.current_row_marker(),
        (Color::from_pixel(0x445566), 21),
    );
    let mut x = 88.0;

    action.apply_before_row_transition(&mut row_extend, &mut x, 3.0);

    assert_eq!(x, 3.0);
    assert_eq!(row_extend.value_on(&geometry), None);

    let mut byte_idx = 17;
    let mut charpos = 22;
    let mut hit_row_range = HitRowRangeTracker::new(6);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;

    let continuation = action.apply_after_visible_row_transition(
        TextMatrixRowTransition::BeganNextRow,
        &mut byte_idx,
        &mut charpos,
        &mut hit_row_range,
        &mut face_scan,
        &geometry,
        DisplayRowVisibilityLimit {
            max_rows: 2,
            bottom_y: 64.0,
        },
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Continue);
    assert_eq!(byte_idx, 13);
    assert_eq!(charpos, 21);
    assert_eq!(hit_row_range.start(), 21);
    assert!(face_scan.should_resolve_at(0));
}

#[test]
fn buffer_text_character_wrap_source_action_skips_state_when_transition_exhausted() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let action = BufferTextCharacterWrapSourceAction::new(13, 21);
    let mut byte_idx = 17;
    let mut charpos = 22;
    let mut hit_row_range = HitRowRangeTracker::new(6);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;

    let continuation = action.apply_after_visible_row_transition(
        TextMatrixRowTransition::ExhaustedRows,
        &mut byte_idx,
        &mut charpos,
        &mut hit_row_range,
        &mut face_scan,
        &geometry,
        DisplayRowVisibilityLimit {
            max_rows: 2,
            bottom_y: 64.0,
        },
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Exhausted);
    assert_eq!(byte_idx, 17);
    assert_eq!(charpos, 22);
    assert_eq!(hit_row_range.start(), 6);
    assert!(!face_scan.should_resolve_at(0));
}

#[test]
fn buffer_text_character_wrap_source_action_reports_hidden_after_state_sync() {
    let geometry = DisplayRowGeometryState::new(0, 64.0, 0.0, 16.0, 12.0);
    let action = BufferTextCharacterWrapSourceAction::new(13, 21);
    let mut byte_idx = 17;
    let mut charpos = 22;
    let mut hit_row_range = HitRowRangeTracker::new(6);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;

    let continuation = action.apply_after_visible_row_transition(
        TextMatrixRowTransition::BeganNextRow,
        &mut byte_idx,
        &mut charpos,
        &mut hit_row_range,
        &mut face_scan,
        &geometry,
        DisplayRowVisibilityLimit {
            max_rows: 2,
            bottom_y: 64.0,
        },
    );

    assert_eq!(continuation, DisplayRowTransitionContinuation::Hidden);
    assert_eq!(byte_idx, 13);
    assert_eq!(charpos, 21);
    assert_eq!(hit_row_range.start(), 21);
    assert!(face_scan.should_resolve_at(0));
}

#[test]
fn buffer_text_overflow_render_request_handles_character_wrap_transition() {
    let mut context = RowTransitionTestContext::new("text-overflow-character-wrap-request");
    let text = b"a";
    let mut byte_idx = 0;
    let decoded_source_char =
        BufferTextDecodedSourceChar::consume_from_text(text, &mut byte_idx, 21)
            .expect("decoded source char");
    let prepared_append = BufferTextSourceCharPreparedAppend {
        plan: BufferTextSourceCharAppendPlan {
            source_text: BufferTextSourceTextRequest::new(
                BufferTextSourceRange::single_char(CharPos0::new(21)),
                ResolvedBufferTextSourceAdvance::resolved(8.0),
            ),
            position: DisplayRowPosition {
                x_px: 80.0,
                col: 10,
            },
        },
    };
    let mut charpos = 21;
    let mut col = 10;
    let mut row_extend = DisplayRowScopedValue::inactive();
    row_extend.activate(
        context.geometry.current_row_marker(),
        (Color::from_pixel(0x445566), 21),
    );
    let mut x = 80.0;
    let mut line_numbers = LineNumberRenderState::new(false, 0, 0);
    let mut hit_row_range = HitRowRangeTracker::new(6);
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut word_wrap = WordWrapRenderState::new(false);
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(false, 0);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;
    let row_limit = context.row_limit;

    let outcome = BufferTextOverflowRenderRequest::new(
        prepared_append,
        decoded_source_char,
        'a',
        80.0,
        false,
        word_wrap,
        DisplayRowVisibilityLimit {
            max_rows: 4,
            bottom_y: 64.0,
        },
        0.0,
        false,
        context.defaults,
        0,
        4,
        row_limit,
    )
    .render_if_needed_and_apply(
        text,
        BufferTextOverflowRenderState {
            byte_idx: &mut byte_idx,
            charpos: &mut charpos,
            col: &mut col,
            output_emitter: &mut context.output_emitter,
            row_extend: &mut row_extend,
            x: &mut x,
            line_numbers: &mut line_numbers,
            row_geometry: &mut context.geometry,
            row_flags: &mut context.row_flags,
            hit_rows: &mut context.hit_rows,
            hit_row_range: &mut hit_row_range,
            builder: &mut context.builder,
            evaluator: &mut context.eval,
            prefix_request: &mut prefix_request,
            hscroll_skip: &mut hscroll_skip,
            word_wrap: &mut word_wrap,
            trailing_whitespace: &mut trailing_whitespace,
            face_scan: &mut face_scan,
            row_y_positions: &mut context.row_y_positions,
        },
    );

    assert_eq!(
        outcome,
        BufferTextOverflowRenderOutcome::Transition(DisplayRowTransitionContinuation::Continue)
    );
    assert_eq!(byte_idx, 0);
    assert_eq!(charpos, 21);
    assert_eq!(hit_row_range.start(), 21);
    assert_eq!(x, 0.0);
    assert!(face_scan.should_resolve_at(0));
    assert_eq!(row_extend.value_on(&context.geometry), None);
}

#[test]
fn display_row_transition_render_state_applies_overflow_wrap_policy() {
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut line_numbers = LineNumberRenderState::new(true, 4, 9);
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    word_wrap.record_candidate(
        'a',
        0,
        4,
        2,
        (Some(LispCharPos1::new(1)), Some(LispCharPos1::new(1))),
    );
    let break_candidate = word_wrap.candidate();
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    trailing_whitespace.track_rendered_char(' ', geometry.start_marker_at_x(8.0));
    let col = 7;
    let BufferTextSourceCharOverflowAction::WordWrap { transition, .. } =
        BufferTextSourceCharOverflowAction::for_decision(BufferTextRowOverflowDecision::WordWrap {
            break_candidate,
        })
    else {
        panic!("expected word wrap transition");
    };

    DisplayRowTransitionRenderState::new(
        &mut prefix_request,
        true,
        &mut line_numbers,
        &mut hscroll_skip,
        &mut word_wrap,
        &mut trailing_whitespace,
    )
    .apply_overflow_prefix(transition);

    assert_eq!(col, 7);
    assert_eq!(prefix_request, DisplayRowPrefixRequest::Wrap);
    assert_eq!(line_numbers.current_line(), 4);
    assert!(!hscroll_skip.should_skip());
    assert!(!word_wrap.has_candidate());
    assert_eq!(
        trailing_whitespace.start_marker(),
        DisplayRowStartMarker::Inactive
    );
}

#[test]
fn display_row_overflow_transition_request_marks_truncated_row_and_emits_boundary() {
    let mut ctx = RowTransitionTestContext::new("overflow-truncation-request");

    let transition = DisplayRowOverflowTransitionRequest::truncation(
        DisplayRowHitRange {
            charpos_start: 3,
            charpos_end: 9,
        },
        ctx.defaults,
        0,
        6,
        48.0,
        ctx.row_y_positions.recording(),
        4,
    )
    .emit(
        &mut ctx.geometry,
        &mut ctx.row_flags,
        ctx.row_limit,
        &mut ctx.hit_rows,
        &mut ctx.builder,
        &mut ctx.output_emitter,
        &mut ctx.eval,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(ctx.geometry.row(), 1);
    assert_eq!(ctx.hit_rows.len(), 1);
    assert_eq!(ctx.hit_rows[0].charpos_start, 3);
    assert_eq!(ctx.hit_rows[0].charpos_end, 9);
    assert!(ctx.row_flags.is_set(0, DisplayRowFlagKind::Truncated));
    assert!(!ctx.row_flags.is_set(0, DisplayRowFlagKind::Continued));
    assert!(!ctx.row_flags.is_set(1, DisplayRowFlagKind::Continuation));
    assert_eq!(ctx.row_y_positions.recorded(), &[0.0, 16.0]);
}

#[test]
fn display_row_overflow_transition_request_marks_visual_wrap_rows_and_emits_boundary() {
    let mut ctx = RowTransitionTestContext::new("overflow-visual-wrap-request");

    let transition = DisplayRowOverflowTransitionRequest::visual_wrap(
        DisplayRowHitRange {
            charpos_start: 3,
            charpos_end: 9,
        },
        ctx.defaults,
        0,
        6,
        48.0,
        ctx.row_y_positions.recording(),
        4,
    )
    .emit(
        &mut ctx.geometry,
        &mut ctx.row_flags,
        ctx.row_limit,
        &mut ctx.hit_rows,
        &mut ctx.builder,
        &mut ctx.output_emitter,
        &mut ctx.eval,
    );

    assert_eq!(transition, TextMatrixRowTransition::BeganNextRow);
    assert_eq!(ctx.geometry.row(), 1);
    assert_eq!(ctx.hit_rows.len(), 1);
    assert_eq!(ctx.hit_rows[0].charpos_start, 3);
    assert_eq!(ctx.hit_rows[0].charpos_end, 9);
    assert!(ctx.row_flags.is_set(0, DisplayRowFlagKind::Continued));
    assert!(ctx.row_flags.is_set(1, DisplayRowFlagKind::Continuation));
    assert!(!ctx.row_flags.is_set(0, DisplayRowFlagKind::Truncated));
    assert_eq!(ctx.row_y_positions.recorded(), &[0.0, 16.0]);
}

fn test_active_face_state(face_id: u32, char_width: f32) -> DisplayRowActiveFaceState {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut base = resolver.default_face().clone();
    base.font_char_width = char_width;
    let mut font_metrics = None;
    let measured = DisplayRowMeasurementPolicy::for_frame(false).measured_face(
        face_id,
        &base,
        None,
        char_width,
        DisplayRowFallbackMetrics {
            char_width,
            row_height: 18.0,
            ascent: 13.0,
        },
        &mut font_metrics,
    );
    DisplayRowActiveFaceState::new(base, measured)
}

fn test_append_frame(
    char_width: f32,
    space_width: f32,
    tab_policy: DisplayTabPolicy,
) -> DisplayRowAppendFrame {
    test_append_frame_at(
        0,
        0.0,
        0.0,
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 12.0,
            char_width,
            space_width,
            default_row_height: 16.0,
        },
        tab_policy,
    )
}

fn test_append_frame_at(
    row: usize,
    y: f32,
    glyph_y: f32,
    area: DisplayRowAppendArea,
    metrics: DisplayRowAppendMetrics,
    tab_policy: DisplayTabPolicy,
) -> DisplayRowAppendFrame {
    let surface = DisplayRowAppendSurface::new(area, tab_policy);
    let geometry = DisplayRowGeometryState::new(row, y, 0.0, metrics.height, metrics.ascent);
    surface.frame_from_geometry_state(&geometry, glyph_y - y, metrics)
}

#[test]
fn text_window_append_surface_request_reserves_right_columns() {
    let tab_stops = vec![4, 12];
    let surface =
        TextWindowAppendSurfaceRequest::new(20.0, 200.0, 16.0, true, true, 8.0, 6, &tab_stops)
            .into_surface();

    assert_eq!(surface.content_x(), 20.0);
    assert_eq!(surface.right_edge(), 188.0);
    assert_eq!(surface.full_text_right_edge(), 204.0);
}

#[test]
fn fallback_buffer_text_source_natural_advance_uses_frame_tab_policy() {
    let active_face = test_active_face_state(7, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(4));
    let mut font_metrics = None;

    let advance = fallback_buffer_text_source_range_natural_advance_to_text_row(
        &mut font_metrics,
        &active_face,
        &frame,
        DisplayRowPosition { x_px: 8.0, col: 1 },
        BufferTextSourceClusterState::for_char('\t', None),
    );

    assert_eq!(advance, 24.0);
}

#[test]
fn fallback_buffer_text_source_natural_advance_zeroes_cluster_continuation() {
    let active_face = test_active_face_state(7, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let mut font_metrics = None;

    let advance = fallback_buffer_text_source_range_natural_advance_to_text_row(
        &mut font_metrics,
        &active_face,
        &frame,
        DisplayRowPosition { x_px: 8.0, col: 1 },
        BufferTextSourceClusterState::for_char('\u{301}', Some(('e', false))),
    );

    assert_eq!(advance, 0.0);
}

#[test]
fn fallback_buffer_text_source_natural_advance_uses_face_columns() {
    let active_face = test_active_face_state(7, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let mut font_metrics = None;

    let advance = fallback_buffer_text_source_range_natural_advance_to_text_row(
        &mut font_metrics,
        &active_face,
        &frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        BufferTextSourceClusterState::for_char('中', None),
    );

    assert_eq!(advance, 16.0);
}

#[test]
fn buffer_text_source_natural_fallback_advance_names_width_policy() {
    assert_eq!(
        BufferTextSourceNaturalFallbackAdvance::for_cluster_state(
            BufferTextSourceClusterState::for_char('\t', None),
        ),
        BufferTextSourceNaturalFallbackAdvance::Tab
    );
    assert_eq!(
        BufferTextSourceNaturalFallbackAdvance::for_cluster_state(
            BufferTextSourceClusterState::for_char('\u{301}', Some(('e', false))),
        ),
        BufferTextSourceNaturalFallbackAdvance::ClusterContinuation
    );
    assert_eq!(
        BufferTextSourceNaturalFallbackAdvance::for_cluster_state(
            BufferTextSourceClusterState::for_char('x', None),
        ),
        BufferTextSourceNaturalFallbackAdvance::FaceColumns { columns: 1 }
    );
    assert_eq!(
        BufferTextSourceNaturalFallbackAdvance::for_cluster_state(
            BufferTextSourceClusterState::for_char('中', None),
        ),
        BufferTextSourceNaturalFallbackAdvance::FaceColumns { columns: 2 }
    );
}

#[test]
fn buffer_text_source_advance_path_names_append_measurement_policy() {
    assert_eq!(
        BufferTextSourceAdvancePath::for_cluster_state(BufferTextSourceClusterState::for_char(
            'x', None,
        )),
        BufferTextSourceAdvancePath::NaturalRenderedSource
    );
    assert_eq!(
        BufferTextSourceAdvancePath::for_cluster_state(BufferTextSourceClusterState::for_char(
            '\t', None,
        )),
        BufferTextSourceAdvancePath::NaturalRenderedSource
    );
    assert_eq!(
        BufferTextSourceAdvancePath::for_cluster_state(BufferTextSourceClusterState::for_char(
            '\u{301}',
            Some(('e', false)),
        )),
        BufferTextSourceAdvancePath::NaturalRenderedSource
    );
    assert_eq!(
        BufferTextSourceAdvancePath::for_cluster_state(BufferTextSourceClusterState::for_char(
            '中', None,
        )),
        BufferTextSourceAdvancePath::NaturalRenderedSource
    );
    assert_eq!(
        BufferTextSourceAdvancePath::for_cluster_state(BufferTextSourceClusterState::for_char(
            '\u{0633}', None,
        )),
        BufferTextSourceAdvancePath::ResolvedComplexRun
    );
}

fn current_buffer_snapshot(eval: &Context, buf_id: BufferId) -> LayoutBufferSnapshot {
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    LayoutBufferSnapshot::from_buffer(buffer)
}

#[test]
fn buffer_text_source_range_append_requests_preserve_source_and_kind() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("\tA");
    }
    let snapshot = current_buffer_snapshot(&eval, buf_id);

    let tab_request = buffer_text_source_range_append_request(
        BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
        buf_id,
        &snapshot,
        7,
    )
    .expect("tab append request");
    assert_eq!(tab_request.append_kind(), DisplayRowAppendKind::Tab);
    let tab_item = tab_request.into_item();
    assert_eq!(tab_item.face, RenderFaceRef::FaceId(7));
    assert_eq!(
        tab_item.span.start,
        DisplaySourcePosition::buffer(buf_id, CharPos0::new(0), EmacsBytePos::new(0))
    );
    assert!(matches!(
        &tab_item.kind,
        DisplayItemKind::TextRun(run) if run.text.as_ref() == "\t"
    ));

    let mapped_request = buffer_text_source_item_append_request(
        BufferTextSourceItemRequest::new(
            BufferTextSourceRange::new(CharPos0::new(1), CharPos0::new(2)),
            BufferTextSourceAppendItem::SourceMappedText { text: "x".into() },
        ),
        buf_id,
        &snapshot,
        9,
    )
    .expect("mapped append request");
    assert_eq!(
        mapped_request.append_kind(),
        DisplayRowAppendKind::SourceMappedText
    );
    let mapped_item = mapped_request.into_item();
    assert_eq!(mapped_item.face, RenderFaceRef::FaceId(9));
    assert_eq!(
        mapped_item.span.start,
        DisplaySourcePosition::buffer(buf_id, CharPos0::new(1), EmacsBytePos::new(1))
    );
    assert!(matches!(
        &mapped_item.kind,
        DisplayItemKind::SourceMappedText(text) if text.text.as_ref() == "x"
    ));
}

#[test]
fn buffer_text_source_append_context_resolves_natural_measurement_for_ascii() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let mut append_state = BufferTextRowAppendState::default();
    let append_context = BufferTextSourceRangeAppendContext::new(
        &snapshot,
        buf_id,
        active_face.face_id(),
        active_face.resolved_face(),
        frame,
    );

    let resolved = append_context.resolve_source_advance_request_to_text_row(
        &mut append_state,
        &mut TextRowSourceMeasureState::new(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
        ),
        &active_face,
        BufferTextSourceAdvanceRequest {
            text: b"x",
            byte_idx: 0,
            range: BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
            position: DisplayRowPosition { x_px: 0.0, col: 0 },
            cluster: BufferTextSourceClusterState::for_char('x', None),
        },
    );

    assert_eq!(resolved.advance_px(), 8.0);
    assert_eq!(
        resolved.append_measurement(),
        DisplaySourceAppendMeasurement::Natural
    );
}

#[test]
fn buffer_text_source_append_context_measures_ascii_at_right_edge() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let mut append_state = BufferTextRowAppendState::default();
    let append_context = BufferTextSourceRangeAppendContext::new(
        &snapshot,
        buf_id,
        active_face.face_id(),
        active_face.resolved_face(),
        frame,
    );

    let resolved = append_context.resolve_source_advance_request_to_text_row(
        &mut append_state,
        &mut TextRowSourceMeasureState::new(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
        ),
        &active_face,
        BufferTextSourceAdvanceRequest {
            text: b"x",
            byte_idx: 0,
            range: BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
            position: DisplayRowPosition {
                x_px: 80.0,
                col: 10,
            },
            cluster: BufferTextSourceClusterState::for_char('x', None),
        },
    );

    assert_eq!(resolved.advance_px(), 8.0);
    assert_eq!(
        resolved.append_measurement(),
        DisplaySourceAppendMeasurement::Natural
    );
}

#[test]
fn buffer_text_source_append_context_resolves_complex_text_measurement() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let mut append_state = BufferTextRowAppendState::default();
    let append_context = BufferTextSourceRangeAppendContext::new(
        &snapshot,
        buf_id,
        active_face.face_id(),
        active_face.resolved_face(),
        frame,
    );

    let resolved = append_context.resolve_source_advance_request_to_text_row(
        &mut append_state,
        &mut TextRowSourceMeasureState::new(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
        ),
        &active_face,
        BufferTextSourceAdvanceRequest {
            text: "\u{0633}".as_bytes(),
            byte_idx: 0,
            range: BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
            position: DisplayRowPosition { x_px: 0.0, col: 0 },
            cluster: BufferTextSourceClusterState::for_char('\u{0633}', None),
        },
    );

    assert_eq!(resolved.advance_px(), 8.0);
    assert_eq!(
        resolved.append_measurement(),
        DisplaySourceAppendMeasurement::ResolvedAdvance { advance_px: 8.0 }
    );
}

#[test]
fn synthetic_display_text_item_builds_synthetic_text_run() {
    let item = synthetic_display_text_item(SyntheticTextSource::new(9, "..."), 7);

    assert_eq!(item.face, RenderFaceRef::FaceId(7));
    assert_eq!(item.span.start, DisplaySourcePosition::synthetic(9, 0));
    assert_eq!(item.span.end, DisplaySourcePosition::synthetic(9, 3));
    match item.kind {
        DisplayItemKind::TextRun(run) => assert_eq!(&*run.text, "..."),
        other => panic!("expected text run, got {other:?}"),
    }
}

#[test]
fn display_row_append_frame_builds_from_geometry_state() {
    let geometry = DisplayRowGeometryState::new(2, 40.0, 0.0, 18.0, 13.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 10.0,
            width: 90.0,
            text_width: 120.0,
            line_number_width: 6.0,
        },
        DisplayTabPolicy::every(4),
    );

    let frame = surface.frame_from_geometry_state(
        &geometry,
        3.0,
        DisplayRowAppendMetrics {
            height: 18.0,
            ascent: 13.0,
            char_width: 7.0,
            space_width: 8.0,
            default_row_height: 16.0,
        },
    );

    assert_eq!(frame.row, 2);
    assert_eq!(frame.glyph_y, 43.0);
    assert_eq!(frame.geometry.y, 40.0);
    assert_eq!(frame.geometry.width, 90.0);
    assert_eq!(frame.geometry.height, 18.0);
    assert_eq!(frame.default_row_height, 16.0);
    assert_eq!(frame.content_x, 10.0);
    assert_eq!(frame.text_width, 120.0);
    assert_eq!(frame.line_number_width, 6.0);
}

#[test]
fn synthetic_text_append_context_renders_fragment_and_emits_slots() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("append-synthetic-text", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let append_context =
        SyntheticTextRowAppendContext::new(&surface, &geometry, &active_face, 0.0, 16.0);
    let (progress, end) = append_context
        .append_request_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            SyntheticTextAppendRequest::active_source(
                DisplayRowPosition { x_px: 0.0, col: 0 },
                SyntheticTextSource::new(99, "..."),
            ),
        )
        .expect("synthetic text progress");

    assert_eq!(end, DisplayRowPosition { x_px: 24.0, col: 3 });
    assert_eq!(progress.metrics.width_px, 24.0);
    assert_eq!(progress.metrics.width_cols, 3);
    assert_eq!(progress.slots.len(), 3);
    assert_eq!(
        progress.slots[0].source,
        DisplaySourcePosition::synthetic(99, 0)
    );
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 3);
            assert!(text.iter().all(|glyph| glyph.face_id == 7));
            assert!(
                text.iter()
                    .all(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '.' }))
            );
        })
        .expect("current row");
}

#[test]
fn synthetic_text_marker_names_source_ids_and_text() {
    assert_eq!(SyntheticTextMarker::InvisibleEllipsis.source_id(), 3);
    assert_eq!(SyntheticTextMarker::InvisibleEllipsis.text(), "...");
    assert_eq!(SyntheticTextMarker::HscrollTruncation.source_id(), 4);
    assert_eq!(SyntheticTextMarker::HscrollTruncation.text(), "$");
    assert_eq!(SyntheticTextMarker::SelectiveEllipsis.source_id(), 5);
    assert_eq!(SyntheticTextMarker::SelectiveEllipsis.text(), "...");
}

#[test]
fn buffer_synthetic_text_render_context_renders_active_marker() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-synthetic-active-marker", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);

    let end = BufferSyntheticTextRenderContext::new(&surface, &active_face, 0.0, 16.0, 12.0, 8.0)
        .render_active_marker_to_text_row(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            &geometry,
            DisplayRowPosition { x_px: 0.0, col: 0 },
            SyntheticTextMarker::InvisibleEllipsis,
        )
        .expect("active marker end position");

    assert_eq!(end, DisplayRowPosition { x_px: 24.0, col: 3 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 3);
            assert!(text.iter().all(|glyph| glyph.face_id == 7));
        })
        .expect("current row");
}

#[test]
fn buffer_synthetic_text_render_context_renders_hscroll_marker() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-synthetic-hscroll-marker", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);

    let end = BufferSyntheticTextRenderContext::new(&surface, &active_face, 0.0, 16.0, 12.0, 8.0)
        .render_hscroll_truncation_marker_to_text_row(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            &geometry,
            0.0,
        )
        .expect("hscroll marker end position");

    assert_eq!(end, DisplayRowPosition { x_px: 8.0, col: 1 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 1);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: '$' }));
            assert_eq!(text[0].face_id, 0);
        })
        .expect("current row");
}

#[test]
fn display_row_prefix_source_builds_append_request_with_prefix_source_id() {
    let _eval = Context::new();
    let value = Value::string("=>");
    let source = DisplayRowPrefixRequest::Line
        .source_for_value(value, CharPos0::new(4))
        .expect("prefix source");

    let request = source.append_request(DisplayRowPosition { x_px: 10.0, col: 2 });

    assert_eq!(request.value, value);
    assert_eq!(request.source_id, LispStringSourceId::PREFIX);
    assert_eq!(request.position, DisplayRowPosition { x_px: 10.0, col: 2 });
}

#[test]
fn buffer_line_prefix_render_context_renders_default_prefix_and_clears_request() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("abc");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-line-prefix-context", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let values = DisplayRowPrefixValues::default_values(Some(Value::string("=>")), None);
    let mut prefix_request = DisplayRowPrefixRequest::Line;

    let end =
        BufferLinePrefixRenderContext::new(values, &surface, &geometry, &active_face, 0.0, 16.0)
            .render_requested_to_text_row_and_emit(
                &mut prefix_request,
                &mut eval,
                &mut output_emitter,
                &snapshot,
                0,
                &mut font_metrics,
                &face_resolver,
                &mut face_ids,
                &mut builder,
                DisplayRowPosition { x_px: 0.0, col: 0 },
            );

    assert_eq!(prefix_request, DisplayRowPrefixRequest::None);
    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 2);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: '=' }));
            assert!(matches!(text[1].glyph_type, GlyphType::Char { ch: '>' }));
            assert_eq!(text[0].face_id, 0);
            assert_eq!(text[1].face_id, 0);
        })
        .expect("current row");
}

#[test]
fn buffer_line_prefix_render_request_applies_rendered_position() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("abc");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-line-prefix-request", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let values = DisplayRowPrefixValues::default_values(Some(Value::string("=>")), None);
    let mut prefix_request = DisplayRowPrefixRequest::Line;
    let mut x = 0.0;
    let mut col = 0;

    BufferLinePrefixRenderRequest::new(
        BufferLinePrefixRenderContext::new(values, &surface, &geometry, &active_face, 0.0, 16.0),
        DisplayRowPosition { x_px: x, col },
    )
    .render_requested_to_text_row_and_apply(
        &mut prefix_request,
        &mut eval,
        &mut output_emitter,
        &snapshot,
        0,
        &mut font_metrics,
        &face_resolver,
        &mut face_ids,
        &mut builder,
        &mut x,
        &mut col,
    );

    assert_eq!(prefix_request, DisplayRowPrefixRequest::None);
    assert_eq!(x, 16.0);
    assert_eq!(col, 2);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 2);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: '=' }));
            assert!(matches!(text[1].glyph_type, GlyphType::Char { ch: '>' }));
        })
        .expect("current row");
}

#[test]
fn synthetic_text_append_context_composes_with_current_row_tail() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-synthetic-combining", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 1, 0.0, 8.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row_with_width(&mut builder, 'e', 7, 0, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));

    let append_context = SyntheticTextAppendContext::new(7, base_face, frame);
    let (progress, end) = append_context
        .append_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            DisplayRowPosition { x_px: 8.0, col: 1 },
            SyntheticTextSource::new(100, "\u{301}"),
        )
        .expect("combining fragment progress");

    assert_eq!(end, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(progress.metrics.width_px, 0.0);
    assert_eq!(progress.metrics.width_cols, 0);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert!(matches!(
                &text[0].glyph_type,
                GlyphType::Composite { text } if text.as_ref() == "e\u{301}"
            ));
        })
        .expect("current row");
}

#[test]
fn buffer_overlay_string_render_context_disabled_keeps_render_state() {
    let mut ctx = RowTransitionTestContext::new("overlay-disabled-render-state");
    let buf_id = ctx
        .eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = current_buffer_snapshot(&ctx.eval, buf_id);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let active_face = test_active_face_state(7, 8.0);
    let render_context =
        BufferOverlayStringTextRowRenderContext::new(false, 1, &surface, 16.0, 12.0, 0.0, 0, 4);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut x = 24.0;
    let mut col = 3;
    let mut cursor_info = CursorCaptureState::new();
    let mut hit_row_range = HitRowRangeTracker::new(2);

    {
        let mut state = OverlayStringRenderState::new(
            &mut ctx.eval,
            &mut ctx.output_emitter,
            &mut font_metrics,
            &face_resolver,
            &mut x,
            &mut col,
            &mut ctx.geometry,
            &mut cursor_info,
            &mut ctx.hit_rows,
            &mut hit_row_range,
            &mut ctx.row_y_positions,
            &mut face_ids,
            &mut ctx.builder,
        );
        render_context.render_before_at(&buffer, 5, &active_face, &mut state);
    }

    assert_eq!(x, 24.0);
    assert_eq!(col, 3);
    assert_eq!(ctx.geometry.row(), 0);
    assert!(cursor_info.captured().is_none());
    assert!(ctx.hit_rows.is_empty());
    assert_eq!(hit_row_range.start(), 2);
}

#[test]
fn overlay_string_row_break_context_finishes_current_row() {
    let mut ctx = RowTransitionTestContext::new("overlay-row-break-context");
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let active_face = test_active_face_state(7, 8.0);
    let row_context =
        OverlayStringRenderRowContext::new(&surface, &active_face, 16.0, 12.0, 0.0, 0, 4);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut x = 24.0;
    let mut col = 3;
    let mut cursor_info = CursorCaptureState::new();
    let mut hit_row_range = HitRowRangeTracker::new(2);

    {
        let mut state = OverlayStringRenderState::new(
            &mut ctx.eval,
            &mut ctx.output_emitter,
            &mut font_metrics,
            &face_resolver,
            &mut x,
            &mut col,
            &mut ctx.geometry,
            &mut cursor_info,
            &mut ctx.hit_rows,
            &mut hit_row_range,
            &mut ctx.row_y_positions,
            &mut face_ids,
            &mut ctx.builder,
        );

        assert!(OverlayStringRowBreakRenderContext::new(5, row_context).finish_row(&mut state));
    }

    assert_eq!(x, 0.0);
    assert_eq!(col, 0);
    assert_eq!(ctx.geometry.row(), 1);
    assert_eq!(ctx.hit_rows.len(), 1);
    assert_eq!(ctx.hit_rows[0].charpos_start, 2);
    assert_eq!(ctx.hit_rows[0].charpos_end, 5);
    assert_eq!(hit_row_range.start(), 5);
}

#[test]
fn overlay_string_render_batch_empty_keeps_render_state() {
    let mut ctx = RowTransitionTestContext::new("overlay-empty-render-state");
    let buf_id = ctx
        .eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = current_buffer_snapshot(&ctx.eval, buf_id);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let active_face = test_active_face_state(7, 8.0);
    let row_context =
        OverlayStringRenderRowContext::new(&surface, &active_face, 16.0, 12.0, 0.0, 0, 4);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut x = 24.0;
    let mut col = 3;
    let mut cursor_info = CursorCaptureState::new();
    let mut hit_row_range = HitRowRangeTracker::new(2);
    let overlay_strings: [OverlayDisplayString; 0] = [];

    {
        let mut state = OverlayStringRenderState::new(
            &mut ctx.eval,
            &mut ctx.output_emitter,
            &mut font_metrics,
            &face_resolver,
            &mut x,
            &mut col,
            &mut ctx.geometry,
            &mut cursor_info,
            &mut ctx.hit_rows,
            &mut hit_row_range,
            &mut ctx.row_y_positions,
            &mut face_ids,
            &mut ctx.builder,
        );
        render_overlay_string_batch(
            &buffer,
            OverlayStringRenderBatchSource::new(
                &overlay_strings,
                CharPos0::new(5),
                OverlayStringKind::Before,
            ),
            row_context,
            &mut state,
        );
    }

    assert_eq!(x, 24.0);
    assert_eq!(col, 3);
    assert_eq!(ctx.geometry.row(), 0);
    assert!(cursor_info.captured().is_none());
    assert!(ctx.hit_rows.is_empty());
    assert_eq!(hit_row_range.start(), 2);
}

#[test]
fn render_natural_display_item_source_into_current_text_row_and_emit_uses_current_row_tail() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("render-current-row-fragment", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 1, 0.0, 8.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut base_face = face_resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let mut source = crate::display_source::LispStringSourceCursor::new(
        101,
        Value::string("\u{301}"),
        RenderFaceRef::FaceId(7),
    )
    .expect("lisp string source");
    let mut source_state = DisplayRowSourceState::default();
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(8);

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row_with_width(&mut builder, 'e', 7, 0, 8.0);

    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let request = frame.source_append_request(
        DisplayRowPosition { x_px: 8.0, col: 1 },
        7,
        &base_face,
        DisplayRowAppendKind::SourceText,
    );

    let outcome = request
        .render_natural_display_source_into_current_text_row_and_emit(
            &mut DisplayRowCurrentTextRenderState {
                builder: &mut builder,
                output_emitter: &mut output_emitter,
                evaluator: &mut eval,
                font_metrics: &mut font_metrics,
                face_resolver: &face_resolver,
                face_ids: &mut face_ids,
            },
            &mut source,
            &mut source_state,
        )
        .expect("current-row fragment outcome");

    assert_eq!(
        outcome.end_position(),
        DisplayRowPosition { x_px: 8.0, col: 1 }
    );
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert!(matches!(
                &text[0].glyph_type,
                GlyphType::Composite { text } if text.as_ref() == "e\u{301}"
            ));
        })
        .expect("current row");
}

#[test]
fn render_face_ref_id_uses_fallback_for_inherit() {
    assert_eq!(render_face_ref_id(RenderFaceRef::FaceId(12), 7), 12);
    assert_eq!(render_face_ref_id(RenderFaceRef::Inherit, 7), 7);
}

#[test]
fn current_text_row_render_outcome_builds_append_progress() {
    let outcome = CurrentTextRowRenderOutcome {
        stop: DisplayRowRenderStop::Clipped,
        source_slots: vec![DisplayRowGlyphSlot {
            source: DisplaySourcePosition::synthetic(9, 0),
            x_px: 8.0,
            col: 1,
            width_px: 16.0,
            width_cols: 2,
        }],
        end: DisplayRowPosition { x_px: 24.0, col: 3 },
        row_height_px: 18.0,
        row_ascent_px: 13.0,
    };
    let start = DisplayRowPosition { x_px: 8.0, col: 1 };

    let (progress, end) = outcome.into_append_progress_and_position(start);

    assert_eq!(end, DisplayRowPosition { x_px: 24.0, col: 3 });
    assert_eq!(progress.start, start);
    assert_eq!(progress.end, end);
    assert_eq!(progress.metrics.width_px, 16.0);
    assert_eq!(progress.metrics.width_cols, 2);
    assert_eq!(progress.status, DisplayRowAppendStatus::Clipped);
    assert_eq!(progress.slots.len(), 1);
    assert_eq!(
        progress.slots[0].source,
        DisplaySourcePosition::synthetic(9, 0)
    );
}

#[test]
fn append_rendered_display_row_fragment_to_text_row_and_emit_appends_glyphs_and_slots() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("AB");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-rendered-fragment", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut base_face = face_resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let mut face_ids = FrameFaceIdAllocator::new(8);
    let mut font_metrics = None;
    let rendered = {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        let mut source = crate::display_source::BufferTextSourceCursor::new(
            buf_id,
            buffer,
            CharPos0::new(0),
            CharPos0::new(2),
            RenderFaceRef::FaceId(7),
        );
        let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
        let mut source_state = DisplayRowSourceState::default();
        DisplayRowItemSourceRenderRequest::new(
            DisplayRowSourceRequestPolicy::new(
                0.0,
                160.0,
                16.0,
                8.0,
                12.0,
                DisplayTabPolicy::every(8),
                GlyphRowRole::Text,
            )
            .source_request_for_base_face_id(7, &base_face)
            .with_render_bounds(DisplayRowRenderBounds {
                start: DisplayRowPosition { x_px: 16.0, col: 2 },
                max_x_px: 160.0,
            }),
        )
        .render_fragment_step_with_display_host(
            &mut renderer,
            &mut source,
            &mut source_state,
            &face_resolver,
            None,
            &mut face_ids,
        )
        .expect("rendered source")
        .rendered
    };
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 2, 0.0, 16.0);

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row_with_width(&mut builder, 'X', 7, 0, 8.0);
    write_char_to_current_row_with_width(&mut builder, 'Y', 7, 0, 8.0);

    let end = append_rendered_display_row_fragment_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &rendered,
        crate::window_output::TextRowOutput {
            row: 0,
            row_y: 0.0,
            glyph_y: 0.0,
            height: 16.0,
        },
    );

    assert_eq!(end, DisplayRowPosition { x_px: 32.0, col: 4 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 4);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: 'X' }));
            assert!(matches!(text[1].glyph_type, GlyphType::Char { ch: 'Y' }));
            assert!(matches!(text[2].glyph_type, GlyphType::Char { ch: 'A' }));
            assert!(matches!(text[3].glyph_type, GlyphType::Char { ch: 'B' }));
            assert_eq!(row.start_charpos, 0);
            assert_eq!(row.end_charpos, 2);
        })
        .expect("current row");

    let first = output_emitter
        .point_for_lisp_buffer_pos(LispCharPos1::new(1))
        .expect("first buffer display point");
    assert_eq!(first.x, 16);
    assert_eq!(first.col, 2);
    let second = output_emitter
        .point_for_lisp_buffer_pos(LispCharPos1::new(2))
        .expect("second buffer display point");
    assert_eq!(second.x, 24);
    assert_eq!(second.col, 3);
}

#[test]
fn display_row_append_surface_builds_positioned_source_requests() {
    let tab_policy = DisplayTabPolicy::from_tab_width_and_stops(8.0, 4, &[6, 10]);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        tab_policy.clone(),
    );

    let geometry = DisplayRowGeometryState::new(3, 20.0, 0.0, 16.0, 11.0);
    let frame = surface.frame_from_geometry_state(
        &geometry,
        2.0,
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
    );
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = resolver.default_face();
    let request = frame.source_append_request(
        DisplayRowPosition { x_px: 18.0, col: 2 },
        42,
        base_face,
        DisplayRowAppendKind::SourceText,
    );

    assert_eq!(
        request.start_position(),
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(
        request.render_bounds().start,
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(request.render_bounds().max_x_px, 128.0);
    assert_eq!(request.role(), GlyphRowRole::Text);
    assert_eq!(request.base_face_ref(), RenderFaceRef::FaceId(42));
    assert_eq!(
        *request.geometry(),
        DisplayRowGeometry {
            y: 20.0,
            width: 120.0,
            height: 16.0,
            char_width: 9.0,
            ascent: 11.0,
            tab_policy,
        }
    );
    assert_eq!(request.output().row, 3);
    assert_eq!(request.output().row_y, 20.0);
    assert_eq!(request.output().glyph_y, 22.0);
    assert_eq!(request.output().height, 16.0);
}

#[test]
fn display_row_append_frame_derives_layout_output_and_bounds() {
    let tab_policy = DisplayTabPolicy::from_tab_width_and_stops(8.0, 4, &[6, 10]);
    let frame = test_append_frame_at(
        3,
        20.0,
        22.0,
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
        tab_policy,
    );
    let position = DisplayRowPosition { x_px: 8.0, col: 0 };
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = resolver.default_face();

    let ordinary =
        frame.source_append_request(position, 42, base_face, DisplayRowAppendKind::SourceText);
    assert_eq!(
        ordinary.render_bounds().start,
        DisplayRowPosition { x_px: 8.0, col: 0 }
    );
    assert_eq!(ordinary.render_bounds().max_x_px, 128.0);
    assert_eq!(ordinary.geometry().char_width, 9.0);
    assert_eq!(ordinary.output().row, 3);
    assert_eq!(ordinary.output().row_y, 20.0);
    assert_eq!(ordinary.output().glyph_y, 22.0);
    assert_eq!(ordinary.output().height, 16.0);

    let tab = frame.source_append_request(position, 42, base_face, DisplayRowAppendKind::Tab);
    assert_eq!(tab.render_bounds().max_x_px, f32::INFINITY);
    assert_eq!(tab.geometry().char_width, 7.0);
    assert_eq!(tab.output().height, 14.0);

    let control =
        frame.source_append_request(position, 42, base_face, DisplayRowAppendKind::ControlChar);
    assert_eq!(control.render_bounds().max_x_px, 148.0);
    assert_eq!(control.geometry().char_width, 9.0);
    assert_eq!(control.output().height, 14.0);

    let mapped = frame.source_append_request(
        position,
        42,
        base_face,
        DisplayRowAppendKind::SourceMappedText,
    );
    assert_eq!(mapped.render_bounds().max_x_px, 128.0);
    assert_eq!(mapped.output().height, 14.0);

    let glyphless =
        frame.source_append_request(position, 42, base_face, DisplayRowAppendKind::Glyphless);
    assert_eq!(glyphless.render_bounds().max_x_px, 128.0);
    assert_eq!(glyphless.output().height, 16.0);

    let replacement = frame.source_append_request(
        position,
        42,
        base_face,
        DisplayRowAppendKind::DisplayReplacement,
    );
    assert_eq!(replacement.render_bounds().max_x_px, 128.0);
    assert_eq!(replacement.geometry().char_width, 9.0);
    assert_eq!(replacement.output().height, 16.0);

    let replacement_string = frame.source_append_request(
        position,
        42,
        base_face,
        DisplayRowAppendKind::DisplayReplacementString,
    );
    assert_eq!(replacement_string.render_bounds().max_x_px, 128.0);
    assert_eq!(replacement_string.geometry().char_width, 7.0);
    assert_eq!(replacement_string.output().height, 16.0);
}

#[test]
fn display_row_append_kind_names_width_clip_and_output_policy() {
    let frame = test_append_frame_at(
        3,
        20.0,
        22.0,
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
        DisplayTabPolicy::every(4),
    );

    assert_eq!(DisplayRowAppendKind::SourceText.char_width(&frame), 9.0);
    assert_eq!(DisplayRowAppendKind::Tab.char_width(&frame), 7.0);
    assert_eq!(
        DisplayRowAppendKind::DisplayReplacementString.char_width(&frame),
        7.0
    );
    assert!(DisplayRowAppendKind::Tab.max_x(&frame).is_infinite());
    assert_eq!(DisplayRowAppendKind::ControlChar.max_x(&frame), 148.0);
    assert_eq!(DisplayRowAppendKind::Glyphless.max_x(&frame), 128.0);
    assert_eq!(
        DisplayRowAppendKind::DisplayReplacement.output_height(&frame),
        16.0
    );
    assert_eq!(
        DisplayRowAppendKind::ControlChar.output_height(&frame),
        14.0
    );
}

#[test]
fn display_row_append_frame_builds_positioned_source_request() {
    let tab_policy = DisplayTabPolicy::every(4);
    let frame = test_append_frame_at(
        3,
        20.0,
        22.0,
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
        tab_policy,
    );
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = resolver.default_face();

    let request = frame.source_append_request(
        DisplayRowPosition { x_px: 18.0, col: 2 },
        42,
        base_face,
        DisplayRowAppendKind::SourceText,
    );

    assert_eq!(
        request.start_position(),
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(request.render_bounds().max_x_px, 128.0);
    assert_eq!(request.base_face_ref(), RenderFaceRef::FaceId(42));
    assert_eq!(request.output().row, 3);
}

#[test]
fn display_row_append_frame_builds_source_request_directly() {
    let tab_policy = DisplayTabPolicy::every(4);
    let frame = test_append_frame_at(
        3,
        20.0,
        22.0,
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
        tab_policy,
    );
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = resolver.default_face();

    let request = frame.source_append_request(
        DisplayRowPosition { x_px: 18.0, col: 2 },
        42,
        base_face,
        DisplayRowAppendKind::SourceText,
    );

    assert_eq!(
        request.start_position(),
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(request.render_bounds().max_x_px, 128.0);
    assert_eq!(request.base_face_ref(), RenderFaceRef::FaceId(42));
    assert_eq!(request.output().row, 3);
}

#[test]
fn display_row_source_append_request_uses_frame_policy() {
    let tab_policy = DisplayTabPolicy::every(4);
    let frame = test_append_frame_at(
        3,
        20.0,
        22.0,
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
        tab_policy,
    );
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = resolver.default_face();

    let request = frame.source_append_request(
        DisplayRowPosition { x_px: 18.0, col: 2 },
        42,
        base_face,
        DisplayRowAppendKind::ControlChar,
    );

    assert_eq!(
        request.start_position(),
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(request.base_face_id(), 42);
    assert_eq!(
        request.render_bounds().start,
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(request.render_bounds().max_x_px, 148.0);
    assert_eq!(request.geometry().height, 16.0);
    assert_eq!(request.output().height, 14.0);
}

#[test]
fn display_row_append_frame_builds_source_append_request() {
    let frame = test_append_frame_at(
        3,
        20.0,
        22.0,
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
        DisplayTabPolicy::every(4),
    );
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = resolver.default_face();

    let request = frame.source_append_request(
        DisplayRowPosition { x_px: 18.0, col: 2 },
        42,
        base_face,
        DisplayRowAppendKind::ControlChar,
    );

    assert_eq!(
        request.start_position(),
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(request.base_face_id(), 42);
    assert_eq!(
        request.render_bounds().start,
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(request.render_bounds().max_x_px, 148.0);
    assert_eq!(request.base_face_id(), 42);
    assert_eq!(request.output().row, 3);
    assert_eq!(request.output().height, 14.0);
}

#[test]
fn display_row_append_surface_builds_frames_with_shared_area() {
    let tab_policy = DisplayTabPolicy::every(4);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        tab_policy.clone(),
    );

    assert_eq!(surface.content_x(), 8.0);
    assert_eq!(surface.right_edge(), 128.0);

    let full_text_surface = surface.full_text_width_surface();
    assert_eq!(full_text_surface.content_x(), 8.0);
    assert_eq!(full_text_surface.right_edge(), 148.0);
    assert_eq!(surface.full_text_right_edge(), 148.0);

    let geometry = DisplayRowGeometryState::new(3, 20.0, 0.0, 16.0, 11.0);
    let frame = surface.frame_from_geometry_state(
        &geometry,
        2.0,
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
    );

    assert_eq!(frame.row, 3);
    assert_eq!(frame.glyph_y, 22.0);
    assert_eq!(
        frame.geometry,
        DisplayRowGeometry {
            y: 20.0,
            width: 120.0,
            height: 16.0,
            char_width: 9.0,
            ascent: 11.0,
            tab_policy,
        }
    );
    assert_eq!(frame.content_x, 8.0);
    assert_eq!(frame.text_width, 150.0);
    assert_eq!(frame.line_number_width, 10.0);
}

#[test]
fn display_row_text_append_context_builds_text_frame_from_shared_surface() {
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayTabPolicy::every(4),
    );
    let geometry = DisplayRowGeometryState::new(3, 20.0, 0.0, 16.0, 11.0);

    let frame = DisplayRowTextAppendContext::new(&surface, &geometry, 2.0, 14.0)
        .text_row_frame(16.0, 11.0, 9.0);

    assert_eq!(frame.row, 3);
    assert_eq!(frame.glyph_y, 22.0);
    assert_eq!(frame.geometry.height, 16.0);
    assert_eq!(frame.geometry.ascent, 11.0);
    assert_eq!(frame.geometry.char_width, 9.0);
    assert_eq!(frame.face_space_width, 9.0);
    assert_eq!(frame.default_row_height, 14.0);
    assert_eq!(frame.content_x, 8.0);
    assert_eq!(frame.text_width, 150.0);
    assert_eq!(frame.line_number_width, 10.0);
}

#[test]
fn display_row_append_surface_builds_frame_from_active_face_state() {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base = resolver.default_face().clone();
    let mut font_metrics = None;
    let measured = DisplayRowMeasurementPolicy::for_frame(false).measured_face(
        7,
        &base,
        None,
        7.5,
        DisplayRowFallbackMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
        },
        &mut font_metrics,
    );
    let active_face = DisplayRowActiveFaceState::new(base, measured);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayTabPolicy::every(4),
    );

    let geometry = DisplayRowGeometryState::new(3, 20.0, 0.0, 16.0, 12.0);
    let frame =
        DisplayRowActiveFaceAppendContext::new(&surface, &geometry, &active_face, 2.0, 16.0)
            .active_face_frame();

    assert_eq!(frame.row, 3);
    assert_eq!(frame.glyph_y, 22.0);
    assert_eq!(frame.geometry.height, 18.0);
    assert_eq!(frame.geometry.ascent, 13.0);
    assert_eq!(frame.geometry.char_width, 7.5);
    assert_eq!(frame.face_space_width, 8.0);
    assert_eq!(frame.default_row_height, 16.0);

    let full_text_frame =
        DisplayRowActiveFaceAppendContext::new(&surface, &geometry, &active_face, 2.0, 16.0)
            .full_text_width_active_face_frame();
    assert_eq!(full_text_frame.geometry.width, 140.0);
}

#[test]
fn display_row_append_frame_preserves_geometry_and_area() {
    let tab_policy = DisplayTabPolicy::every(4);
    let frame = test_append_frame_at(
        3,
        20.0,
        22.0,
        DisplayRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 11.0,
            char_width: 9.0,
            space_width: 7.0,
            default_row_height: 14.0,
        },
        tab_policy.clone(),
    );

    assert_eq!(frame.row, 3);
    assert_eq!(frame.glyph_y, 22.0);
    assert_eq!(
        frame.geometry,
        DisplayRowGeometry {
            y: 20.0,
            width: 120.0,
            height: 16.0,
            char_width: 9.0,
            ascent: 11.0,
            tab_policy,
        }
    );
    assert_eq!(frame.default_row_height, 14.0);
    assert_eq!(frame.content_x, 8.0);
    assert_eq!(frame.text_width, 150.0);
    assert_eq!(frame.line_number_width, 10.0);
    assert_eq!(frame.face_space_width, 7.0);
}

#[test]
fn layout_display_source_face_resolver_records_pending_faces_without_builder() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut resolve_state = crate::display_source_resolver::DisplaySourceResolveState::default();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut pending_faces = Vec::new();
    let params = crate::display_source_resolver::DisplaySourceResolveParams::new(
        crate::display_source_resolver::DisplaySourceFaceBasis::new(
            &face_resolver,
            0,
            base_face,
            crate::display_source_resolver::DisplaySourceFallbackMetrics::new(8.0, 12.0, 16.0),
        ),
        None,
    );
    let mut resolver = crate::display_source_resolver::DisplaySourcePropertyResolver::new(
        params,
        &mut resolve_state,
        &mut face_ids,
        &mut pending_faces,
    );
    let face_value = Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]);

    let face = crate::display_source::DisplayItemFaceResolver::resolve_face_ref(
        &mut resolver,
        RenderFaceRef::FaceId(0),
        face_value,
    );

    assert_eq!(face, RenderFaceRef::FaceId(20));
    assert_eq!(face_ids.finish(), 21);
    assert_eq!(pending_faces.len(), 1);
    assert_eq!(pending_faces[0].face_id, 20);
    assert_eq!(pending_faces[0].resolved.fg, 0x00ff0000);
}

#[test]
fn display_source_resolve_params_are_built_from_typed_face_basis() {
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let fallback =
        crate::display_source_resolver::DisplaySourceFallbackMetrics::new(8.0, 12.0, 16.0);
    let basis = crate::display_source_resolver::DisplaySourceFaceBasis::new(
        &face_resolver,
        7,
        base_face,
        fallback,
    );

    let params = crate::display_source_resolver::DisplaySourceResolveParams::new(basis, None);

    assert_eq!(params.face_basis().base_face_id(), 7);
    assert_eq!(params.face_basis().fallback_metrics(), fallback);
    assert!(std::ptr::eq(params.face_basis().base_face(), base_face));
    assert!(std::ptr::eq(
        params.face_basis().canonical_face(),
        face_resolver.default_face()
    ));
}

#[test]
fn resolve_next_display_source_item_returns_item_and_pending_faces() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut resolve_state = crate::display_source_resolver::DisplaySourceResolveState::default();
    let value = Value::string_with_text_properties(
        "a",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
            ]),
        }],
    );
    let mut source =
        crate::display_source::LispStringSourceCursor::new(1, value, RenderFaceRef::FaceId(0))
            .expect("string source");

    let resolved = crate::display_source_resolver::resolve_next_display_source_item(
        &mut source,
        crate::display_source_resolver::DisplaySourceResolveParams::new(
            crate::display_source_resolver::DisplaySourceFaceBasis::new(
                &face_resolver,
                0,
                base_face,
                crate::display_source_resolver::DisplaySourceFallbackMetrics::new(8.0, 12.0, 16.0),
            ),
            None,
        ),
        &mut resolve_state,
        &mut face_ids,
    );

    let item = resolved.item.expect("source item");
    assert_eq!(item.face, RenderFaceRef::FaceId(20));
    assert_eq!(resolved.pending_faces.len(), 1);
    assert_eq!(resolved.pending_faces[0].face_id, 20);
    assert_eq!(resolved.pending_faces[0].resolved.fg, 0x00ff0000);
}

#[test]
fn resolve_next_display_source_item_resolves_height_modifier_to_pending_face() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut resolve_state = crate::display_source_resolver::DisplaySourceResolveState::default();
    let value = Value::string_with_text_properties(
        "a",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![Value::symbol("height"), Value::make_float(2.0)]),
            ]),
        }],
    );
    let mut source =
        crate::display_source::LispStringSourceCursor::new(1, value, RenderFaceRef::FaceId(0))
            .expect("string source");

    let resolved = crate::display_source_resolver::resolve_next_display_source_item(
        &mut source,
        crate::display_source_resolver::DisplaySourceResolveParams::new(
            crate::display_source_resolver::DisplaySourceFaceBasis::new(
                &face_resolver,
                0,
                base_face,
                crate::display_source_resolver::DisplaySourceFallbackMetrics::new(8.0, 12.0, 16.0),
            ),
            None,
        ),
        &mut resolve_state,
        &mut face_ids,
    );

    let item = resolved.item.expect("source item");
    assert_eq!(item.face, RenderFaceRef::FaceId(20));
    assert_eq!(resolved.pending_faces.len(), 1);
    assert_eq!(resolved.pending_faces[0].face_id, 20);
    assert_eq!(resolved.pending_faces[0].resolved.font_size, 28.0);
    assert_eq!(resolved.pending_faces[0].resolved.font_line_height, 32.0);
    assert_eq!(resolved.pending_faces[0].resolved.font_ascent, 24.0);
    assert_eq!(resolved.pending_faces[0].resolved.font_char_width, 16.0);
}

#[test]
fn display_row_source_walker_reuses_face_cache_across_items() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let face_value = Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]);
    let value = Value::string_with_text_properties(
        "aba",
        vec![
            StringTextPropertyRun {
                start: 0,
                end: 1,
                plist: Value::list(vec![Value::symbol("face"), face_value.clone()]),
            },
            StringTextPropertyRun {
                start: 2,
                end: 3,
                plist: Value::list(vec![Value::symbol("face"), face_value]),
            },
        ],
    );
    let source =
        crate::display_source::LispStringSourceCursor::new(1, value, RenderFaceRef::FaceId(0))
            .expect("string source");
    let mut source = crate::display_row::DisplayRowSourceWalker::new(source);
    let (first, second, third) = {
        let mut next_item = |label: &str| {
            let mut step = source
                .next_step(
                    &face_resolver,
                    base_face,
                    0,
                    &mut face_ids,
                    None,
                    8.0,
                    12.0,
                    16.0,
                )
                .unwrap_or_else(|| panic!("{label} source item"));
            apply_pending_display_source_faces(&mut builder, &mut step.pending_faces);
            step.item
        };
        (next_item("first"), next_item("second"), next_item("third"))
    };

    assert_eq!(first.face, RenderFaceRef::FaceId(20));
    assert_eq!(second.face, RenderFaceRef::FaceId(0));
    assert_eq!(third.face, RenderFaceRef::FaceId(20));
    assert_eq!(face_ids.finish(), 21);
    assert_eq!(
        builder.faces().get(&20).map(|face| face.foreground),
        Some(Color::from_pixel(0x00ff0000))
    );
}

#[test]
fn append_lisp_string_to_text_row_appends_propertized_string_items() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("append-lisp-string", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let value = Value::string_with_text_properties(
        "ab",
        vec![StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
            ]),
        }],
    );
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));

    let end = append_lisp_string_to_text_row(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        value,
        1,
        &face_resolver,
        base_face,
        0,
        &mut face_ids,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    );

    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(face_ids.finish(), 21);
    assert_eq!(
        builder.faces().get(&20).map(|face| face.foreground),
        Some(Color::from_pixel(0x00ff0000))
    );
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text[0].face_id, 0);
            assert_eq!(text[1].face_id, 20);
        })
        .expect("current row");
}

#[test]
fn lisp_string_append_context_appends_fragment_items() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-lisp-fragment-context", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let active_face = test_active_face_state(0, 8.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let append_context =
        LispStringRowAppendContext::new(&surface, &geometry, &active_face, 0.0, 16.0);

    let request = LispStringSourceAppendRequest::new(
        DisplayRowPosition { x_px: 0.0, col: 0 },
        LispStringSourceId::PREFIX,
        Value::string("=>"),
    );
    let end = append_context.render_active_face_source_request_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &mut font_metrics,
        &face_resolver,
        &mut face_ids,
        0,
        base_face,
        request,
    );

    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 2);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: '=' }));
            assert!(matches!(text[1].glyph_type, GlyphType::Char { ch: '>' }));
        })
        .expect("current row");
}

#[test]
fn buffer_text_source_append_context_appends_source_char() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("ab");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-buffer-fragment", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        LayoutBufferSnapshot::from_buffer(buffer)
    };
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);

    let append_context =
        BufferTextRowAppendContext::new(&snapshot, buf_id, &surface, &active_face, 0.0, 16.0);
    let source_char = BufferTextSourceChar::new('a', CharPos0::new(0), 2);
    let mut append_state = BufferTextRowAppendState::default();
    let prepared_append = append_context
        .prepare_source_char_at(
            &geometry,
            &mut append_state,
            &mut TextRowSourceMeasureState::new(
                &mut builder,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            &source_char,
            b"a",
            0,
            DisplayRowPosition { x_px: 0.0, col: 0 },
            None,
        )
        .into_text()
        .expect("ordinary buffer char should prepare text append");
    let cursor_info = prepared_append.cursor_info_for_main_char(
        &active_face,
        geometry.text_position(2.0, 0, 3),
        false,
    );
    assert_eq!(cursor_info.x, 2.0);
    assert_eq!(cursor_info.col, 3);
    assert_eq!(cursor_info.slot_width, Some(8.0));
    assert!(!cursor_info.stretch_like);
    let mut cursor_capture = CursorCaptureState::new();
    prepared_append.capture_cursor_info_for_main_char_if_point(
        &mut cursor_capture,
        &active_face,
        &geometry,
        2.0,
        0,
        3,
        false,
        4,
        5,
    );
    assert!(cursor_capture.is_missing());
    prepared_append.capture_cursor_info_for_main_char_if_point(
        &mut cursor_capture,
        &active_face,
        &geometry,
        2.0,
        0,
        3,
        false,
        5,
        5,
    );
    let captured_cursor = cursor_capture.as_ref().expect("captured cursor");
    assert_eq!(captured_cursor.x, 2.0);
    assert_eq!(captured_cursor.col, 3);
    assert_eq!(captured_cursor.slot_width, Some(8.0));
    assert!(!captured_cursor.stretch_like);
    assert_eq!(
        prepared_append.overflow_decision('a', 80.0, false, WordWrapRenderState::new(false)),
        BufferTextRowOverflowDecision::Fits
    );
    assert!(matches!(
        prepared_append.overflow_action('a', 80.0, false, WordWrapRenderState::new(false)),
        BufferTextSourceCharOverflowAction::Fits
    ));
    assert!(matches!(
        prepared_append.overflow_action('a', 4.0, true, WordWrapRenderState::new(false)),
        BufferTextSourceCharOverflowAction::Truncate { .. }
    ));
    assert!(matches!(
        prepared_append.overflow_action('a', 4.0, false, WordWrapRenderState::new(false)),
        BufferTextSourceCharOverflowAction::CharacterWrap { .. }
    ));
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    word_wrap.record_candidate(
        'a',
        0,
        0,
        2,
        (Some(LispCharPos1::new(1)), Some(LispCharPos1::new(1))),
    );
    assert!(matches!(
        prepared_append.overflow_action('a', 4.0, false, word_wrap),
        BufferTextSourceCharOverflowAction::WordWrap { break_candidate, .. }
            if break_candidate.byte_idx() == 0
                && break_candidate.charpos() == 0
                && break_candidate.display_point_count() == 2
    ));
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(true, 0x00ff00);
    let mut word_wrap = WordWrapRenderState::new(true);
    let mut charpos = 4;
    let mut end_x = 0.0;
    let mut end_col = 0;
    let continuation = prepared_append.append_to_text_row_and_apply(
        &append_context,
        &geometry,
        ' ',
        &mut BufferTextSourceCharRenderState::new(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            &mut trailing_whitespace,
            &mut word_wrap,
            &mut end_x,
            &mut end_col,
            &mut charpos,
        ),
    );
    assert_eq!(continuation, BufferTextSourceAppendContinuation::Rendered);
    assert_eq!(
        trailing_whitespace
            .highlight_start_x(&geometry)
            .map(|(_color, x)| x),
        Some(0.0)
    );
    assert_eq!(end_x, 8.0);
    assert_eq!(end_col, 1);
    assert_eq!(charpos, 5);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert_eq!(text[0].face_id, 7);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: 'a' }
            ));
        })
        .expect("current row");
}

#[test]
fn buffer_text_source_char_render_request_appends_ordinary_char_and_updates_walk_state() {
    let mut context = RowTransitionTestContext::new("source-char-render-request");
    let buf_id = context
        .eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = context
            .eval
            .buffer_manager_mut()
            .get_mut(buf_id)
            .expect("buffer");
        buffer.insert("ab");
    }
    let snapshot = current_buffer_snapshot(&context.eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let overlay_context =
        BufferOverlayStringTextRowRenderContext::new(false, 1, &surface, 16.0, 12.0, 0.0, 0, 4);
    let params = test_display_space_window_params();
    let text = b"ab";
    let mut byte_idx = 0;
    let decoded_source_char =
        BufferTextDecodedSourceChar::consume_from_text(text, &mut byte_idx, 0)
            .expect("decoded char");
    let mut append_state = BufferTextRowAppendState::default();
    let mut charpos = 0;
    let mut col = 0;
    let mut row_extend = DisplayRowScopedValue::inactive();
    let mut x = 0.0;
    let mut line_numbers = LineNumberRenderState::new(false, 0, 0);
    let mut hit_row_range = HitRowRangeTracker::new(0);
    let mut prefix_request = DisplayRowPrefixRequest::None;
    let mut hscroll_skip = HorizontalScrollSkipState::new(false, 0);
    let mut word_wrap = WordWrapRenderState::new(false);
    let mut trailing_whitespace = TrailingWhitespaceRenderState::new(false, 0);
    let mut face_scan = FaceScanCheckpoint::initial();
    let mut font_metrics = None;
    let mut cursor_info = CursorCaptureState::new();
    let mut face_ids = FrameFaceIdAllocator::new(7);
    let mut raise_span = ActiveDisplayPropertySpan::inactive();

    let outcome = BufferTextSourceCharRenderRequest::new(
        decoded_source_char,
        text,
        0,
        buf_id,
        &surface,
        overlay_context,
        &active_face,
        &params,
        0.0,
        16.0,
        99,
        DisplayRowVisibilityLimit {
            max_rows: 4,
            bottom_y: 64.0,
        },
        0.0,
        false,
        context.defaults,
        0,
        4,
        context.row_limit,
    )
    .render_and_apply(
        &snapshot,
        BufferTextSourceCharRenderRequestState {
            append_state: &mut append_state,
            byte_idx: &mut byte_idx,
            charpos: &mut charpos,
            col: &mut col,
            output_emitter: &mut context.output_emitter,
            row_extend: &mut row_extend,
            x: &mut x,
            line_numbers: &mut line_numbers,
            row_geometry: &mut context.geometry,
            row_flags: &mut context.row_flags,
            hit_rows: &mut context.hit_rows,
            hit_row_range: &mut hit_row_range,
            builder: &mut context.builder,
            evaluator: &mut context.eval,
            prefix_request: &mut prefix_request,
            hscroll_skip: &mut hscroll_skip,
            word_wrap: &mut word_wrap,
            trailing_whitespace: &mut trailing_whitespace,
            face_scan: &mut face_scan,
            row_y_positions: &mut context.row_y_positions,
            font_metrics: &mut font_metrics,
            face_resolver: &face_resolver,
            cursor_info: &mut cursor_info,
            face_ids: &mut face_ids,
            raise_span: &mut raise_span,
        },
    );

    assert_eq!(outcome, BufferTextSourceCharRenderOutcome::Rendered);
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 1);
    assert_eq!(x, 8.0);
    assert_eq!(col, 1);
    context
        .builder
        .with_current_row_mut(|row| {
            let text_glyphs = &row.glyphs[GlyphArea::Text as usize];
            assert_eq!(text_glyphs.len(), 1);
            assert!(matches!(
                text_glyphs[0].glyph_type,
                GlyphType::Char { ch: 'a' }
            ));
        })
        .expect("current row");
}

#[test]
fn buffer_text_source_append_context_prepares_current_text_row_source_char() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("ab");
    }
    let snapshot = {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        LayoutBufferSnapshot::from_buffer(buffer)
    };
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let append_context =
        BufferTextRowAppendContext::new(&snapshot, buf_id, &surface, &active_face, 0.0, 16.0);
    let source_char = BufferTextSourceChar::new('a', CharPos0::new(0), 2);
    let mut append_state = BufferTextRowAppendState::default();

    let prepared_append = append_context
        .prepare_source_char_for_current_text_row(
            BufferTextSourceCharPreparationRequest::new(
                geometry,
                &source_char,
                b"a",
                0,
                DisplayRowPosition { x_px: 0.0, col: 0 },
            ),
            &mut BufferTextSourceCharPreparationState::new(
                &mut append_state,
                &mut builder,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
        )
        .into_text()
        .expect("ordinary buffer char should prepare text append");

    assert_eq!(
        prepared_append.overflow_decision('a', 80.0, false, WordWrapRenderState::new(false)),
        BufferTextRowOverflowDecision::Fits
    );
}

#[test]
fn buffer_end_of_buffer_cursor_action_captures_visible_eob_cursor() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(2, 32.0, 0.0, 16.0, 12.0);
    let action = BufferEndOfBufferCursorAction::new(5, 9, 9, 9);
    let mut cursor = CursorCaptureState::new();

    action.capture_cursor_if_point(&mut cursor, &active_face, &geometry, 48.0, 6);

    let captured = cursor.as_ref().expect("cursor captured");
    assert_eq!(captured.x, 48.0);
    assert_eq!(captured.y, 32.0);
    assert_eq!(captured.byte_idx, 5);
    assert_eq!(captured.col, 6);
    assert_eq!(captured.matrix_row, 2);
    assert_eq!(captured.slot_width, Some(8.0));
    assert!(!captured.stretch_like);
}

#[test]
fn buffer_end_of_buffer_cursor_action_keeps_cursor_missing_when_point_differs() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(2, 32.0, 0.0, 16.0, 12.0);
    let action = BufferEndOfBufferCursorAction::new(5, 9, 12, 10);
    let mut cursor = CursorCaptureState::new();

    action.capture_cursor_if_point(&mut cursor, &active_face, &geometry, 48.0, 6);

    assert!(cursor.as_ref().is_none());
}

#[test]
fn buffer_end_of_buffer_tail_action_reports_cursor_and_overlay_state() {
    let active_face = test_active_face_state(9, 8.0);
    let geometry = DisplayRowGeometryState::new(2, 32.0, 0.0, 16.0, 12.0);
    let row_limit = DisplayRowLimit { max_rows: 4 };
    let action = BufferEndOfBufferTailAction::new(5, 9, 9, 9, true);
    let mut cursor = CursorCaptureState::new();

    assert!(action.point_is_visible_eob());
    assert!(action.should_render_overlay_strings(&geometry, row_limit));
    action.capture_cursor_if_point(&mut cursor, &active_face, &geometry, 48.0, 6);

    let captured = cursor.as_ref().expect("cursor captured");
    assert_eq!(captured.x, 48.0);
    assert_eq!(captured.matrix_row, 2);

    let overlays_disabled = BufferEndOfBufferTailAction::new(5, 9, 9, 9, false);
    assert!(!overlays_disabled.should_render_overlay_strings(&geometry, row_limit));
}

#[test]
fn buffer_end_of_buffer_tail_render_request_captures_cursor_and_renders_overlay() {
    let mut context = RowTransitionTestContext::new("eob-tail-render-request");
    let buf_id = context
        .eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = context
            .eval
            .buffer_manager_mut()
            .get_mut(buf_id)
            .expect("buffer");
        buffer.insert("abc");
        let eob = buffer.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buffer.overlays_mut().insert_overlay(overlay);
        let _ = buffer.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("Z"),
        );
    }

    let snapshot = current_buffer_snapshot(&context.eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let overlay_context =
        BufferOverlayStringTextRowRenderContext::new(true, 1, &surface, 16.0, 12.0, 0.0, 0, 4);
    let mut x = 24.0;
    let mut col = 3;
    let mut cursor_info = CursorCaptureState::new();
    let mut hit_row_range = HitRowRangeTracker::new(0);
    let mut face_ids = FrameFaceIdAllocator::new(7);
    let mut font_metrics = None;

    let outcome = BufferEndOfBufferTailRenderRequest::new(
        3,
        3,
        3,
        3,
        true,
        overlay_context,
        &active_face,
        context.row_limit,
    )
    .render_and_apply(
        &snapshot,
        BufferEndOfBufferTailRenderState {
            output_emitter: &mut context.output_emitter,
            x: &mut x,
            col: &mut col,
            row_geometry: &mut context.geometry,
            cursor_info: &mut cursor_info,
            hit_rows: &mut context.hit_rows,
            hit_row_range: &mut hit_row_range,
            row_y_positions: &mut context.row_y_positions,
            face_ids: &mut face_ids,
            builder: &mut context.builder,
            evaluator: &mut context.eval,
            font_metrics: &mut font_metrics,
            face_resolver: &face_resolver,
        },
    );

    assert!(outcome.point_is_visible_eob());
    let captured = cursor_info.captured().expect("EOB cursor captured");
    assert_eq!(captured.x, 24.0);
    assert_eq!(captured.col, 3);
    assert_eq!(x, 32.0);
    assert_eq!(col, 4);
    context
        .builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[GlyphArea::Text.index()];
            assert_eq!(text.len(), 1);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: 'Z' }));
        })
        .expect("current row");
}

#[test]
fn buffer_text_window_tail_finalize_request_publishes_cursor_and_finishes_row() {
    let mut context = RowTransitionTestContext::new("tail-finalize-request");
    let mut params = test_display_space_window_params();
    params.window_id = 1;
    params.selected = true;
    params.cursor_color = 0x00ffffff;
    params.text_bounds = Rect::new(0.0, 0.0, 160.0, 48.0);
    params.visual_cursors = vec![crate::types::VisualCursorSpec {
        id: -42,
        charpos: 0,
        cursor_kind: neomacs_display_protocol::frame_glyphs::CursorKind::Bar,
        cursor_bar_width: neomacs_display_protocol::frame_glyphs::CursorBarWidth::new(3),
        color: 0x00112233,
        effects: None,
    }];
    context
        .output_emitter
        .push_display_point(LispCharPos1::ONE, 34.0, 20.0, 11.0, 16.0, 0, 4);

    let mut cursor_info = CursorCaptureState::new();
    cursor_info.capture_once(crate::display_cursor::CapturedCursorInfo {
        x: 0.0,
        y: 0.0,
        face_w: 8.0,
        face_h: 16.0,
        face_ascent: 12.0,
        bg: Color::BLACK,
        byte_idx: 0,
        col: 0,
        matrix_row: 0,
        slot_width: Some(8.0),
        stretch_like: false,
    });
    let mut hit_row_range = HitRowRangeTracker::new(0);

    let outcome = BufferTextWindowTailFinalizeRequest::new(
        &params,
        b"abc",
        0,
        0.0,
        0.0,
        0.0,
        48.0,
        8.0,
        16.0,
        0,
        0,
        3,
        false,
        context.row_limit,
    )
    .finalize_and_apply(BufferTextWindowTailFinalizeState {
        cursor_info: &mut cursor_info,
        row_geometry: &context.geometry,
        row_y_positions: &context.row_y_positions,
        hit_row_range: &mut hit_row_range,
        hit_rows: &mut context.hit_rows,
        builder: &mut context.builder,
        output_emitter: &mut context.output_emitter,
        evaluator: &mut context.eval,
    });

    assert!(outcome.cursor_requested());
    assert!(outcome.cursor_published());
    assert!(outcome.pending_row_finished());
    assert_eq!(outcome.visual_cursor_summary().requested, 1);
    assert_eq!(outcome.visual_cursor_summary().published, 1);
    assert_eq!(context.hit_rows.len(), 1);
    let cursor = context.builder.phys_cursor().expect("physical cursor");
    assert_eq!(cursor.window_id, 1);
    assert_eq!(cursor.row, 0);
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.x, 0.0);
    assert_eq!(cursor.height, 16.0);
    let cursors = context.builder.cursors();
    assert_eq!(cursors.len(), 1);
    assert_eq!(cursors[0].window_id, -42);
    assert_eq!(cursors[0].slot_id.row, 0);
    assert_eq!(cursors[0].slot_id.col, 4);
}

#[test]
fn buffer_text_window_body_install_request_records_positions_and_edge_markers() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("body-install-request", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(41, 1, 5, Rect::new(0.0, 0.0, 40.0, 20.0), true);
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    crate::window_output::TextMatrixRowOutput::new(&mut builder, &mut output_emitter, &mut eval)
        .begin(crate::window_output::TextMatrixRowBegin {
            matrix_row: 0,
            row: 0,
            col: 0,
            y: 2.0,
            x: 0.0,
        });
    output_emitter.note_display_buffer_pos(LispCharPos1::new(7));
    write_char_to_current_row_with_width(&mut builder, 'x', 7, 0, 8.0);
    crate::window_output::finish_text_matrix_row_output(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        crate::window_output::TextMatrixRowMetrics {
            y: 2.0,
            height: 20.0,
            ascent: 15.0,
        },
    );

    let mut row_flags = DisplayRowFlags::new(1);
    row_flags.mark(0, DisplayRowFlagKind::Truncated);
    let positions = BufferTextWindowBodyInstallRequest::new(
        41, 3, 100, 4, true, false, 0, 5, &row_flags, 9, 8.0,
    )
    .install_and_apply(BufferTextWindowBodyInstallState {
        builder: &mut builder,
        output_emitter: &output_emitter,
    });

    assert_eq!(positions.window_start, LispCharPos1::new(4));
    assert_eq!(positions.window_end, LispCharPos1::new(8));
    assert_eq!(positions.window_end_byte, EmacsBytePos::new(104));
    assert_eq!(positions.window_end_vpos, 0);

    builder.end_window();
    let state = builder.finish(5, 1, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert_eq!(row.height_px, 20.0);
    assert_eq!(row.ascent_px, 15.0);
    let text = &row.glyphs[GlyphArea::Text.index()];
    assert!(matches!(text[4].glyph_type, GlyphType::Char { ch: '$' }));
    assert_eq!(text[4].face_id, 9);
}

#[test]
fn buffer_text_window_begin_request_opens_window_and_first_text_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("begin-request", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let mut output_emitter = BufferTextWindowBeginRequest::new(
        frame_id,
        window_id,
        2,
        10.0,
        5.0,
        41,
        4,
        8,
        Rect::new(3.0, 5.0, 80.0, 64.0),
        Rect::new(10.0, 9.0, 64.0, 48.0),
        true,
        crate::window_output::TextMatrixRowBegin {
            matrix_row: 2,
            row: 0,
            col: 1,
            y: 9.0,
            x: 18.0,
        },
    )
    .begin_and_apply(BufferTextWindowBeginState {
        builder: &mut builder,
        evaluator: &mut eval,
    });

    output_emitter.move_text_output_to(&mut eval, 0, 3, 9.0, 34.0);
    crate::window_output::finish_text_matrix_row_output(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        crate::window_output::TextMatrixRowMetrics {
            y: 9.0,
            height: 17.0,
            ascent: 12.0,
        },
    );
    crate::window_output::close_text_window_output(&mut builder);

    let state = builder.finish(8, 4, 8.0, 16.0);
    assert_eq!(state.window_matrices.len(), 1);
    let window = &state.window_matrices[0];
    assert_eq!(window.window_id, 41);
    assert!(window.selected);
    assert_eq!(window.pixel_bounds, Rect::new(3.0, 5.0, 80.0, 64.0));
    assert_eq!(window.text_pixel_bounds, Rect::new(10.0, 9.0, 64.0, 48.0));
    assert_eq!(window.matrix.rows[2].role, GlyphRowRole::Text);
    assert_eq!(window.matrix.rows[2].pixel_y, 4.0);
    assert_eq!(window.matrix.rows[2].height_px, 17.0);
    assert_eq!(window.matrix.rows[2].ascent_px, 12.0);
}

#[test]
fn buffer_text_window_cursor_effects_request_installs_effect_profile() {
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let effects = EffectsConfig::default();

    let installed = BufferTextWindowCursorEffectsRequest::new(42, Some(effects.clone()))
        .install_and_apply(&mut builder);

    assert!(installed);
    let state = builder.finish(1, 1, 8.0, 16.0);
    assert_eq!(state.cursor_effects_by_window.get(&42), Some(&effects));
}

#[test]
fn buffer_text_window_cursor_effects_request_ignores_missing_effect_profile() {
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();

    let installed =
        BufferTextWindowCursorEffectsRequest::new(42, None).install_and_apply(&mut builder);

    assert!(!installed);
    let state = builder.finish(1, 1, 8.0, 16.0);
    assert!(!state.cursor_effects_by_window.contains_key(&42));
}

#[test]
fn buffer_text_window_terminal_right_border_request_installs_face_and_border() {
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 5, Rect::new(0.0, 0.0, 40.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    for ch in "abcd".chars() {
        write_char_to_current_row_with_width(&mut builder, ch, 0, 0, 8.0);
    }
    builder.end_row();
    builder.end_window();

    let face_id = BufferTextWindowTerminalRightBorderRequest::new(8.0)
        .install_and_apply(&mut builder, &face_resolver);

    let state = builder.finish(5, 1, 8.0, 16.0);
    assert!(state.faces.contains_key(&face_id));
    let row = &state.window_matrices[0].matrix.rows[0];
    let text = &row.glyphs[GlyphArea::Text.index()];
    let right = &row.glyphs[GlyphArea::RightMargin.index()];
    assert_eq!(text.len(), 4);
    assert_eq!(right.len(), 1);
    assert_eq!(right[0].glyph_type, GlyphType::Char { ch: '|' });
    assert_eq!(right[0].face_id, face_id);
}

#[test]
fn buffer_text_window_finish_request_closes_window_and_returns_snapshot_artifacts() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("finish-request", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(41, 1, 5, Rect::new(0.0, 0.0, 40.0, 20.0), true);
    let output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 10.0, 5.0);
    output_emitter.begin_update(&mut eval);
    let hit_rows = vec![crate::hit_test::HitRow {
        y_start: 2.0,
        y_end: 18.0,
        charpos_start: 3,
        charpos_end: 9,
    }];

    let finished = BufferTextWindowFinishRequest::new(41, 12.0, 8.0, 2, 11, 7, 5)
        .finish_and_snapshot(BufferTextWindowFinishState {
            builder: &mut builder,
            output_emitter,
            evaluator: &mut eval,
            hit_rows,
        });

    assert_eq!(finished.hit_data.window_id, 41);
    assert_eq!(finished.hit_data.content_x, 12.0);
    assert_eq!(finished.hit_data.char_w, 8.0);
    assert_eq!(finished.hit_data.rows.len(), 1);
    assert_eq!(finished.hit_data.rows[0].charpos_start, 3);
    assert_eq!(finished.snapshot.text_area_left_offset, 2);
    assert_eq!(finished.snapshot.mode_line_height, 11);
    assert_eq!(finished.snapshot.header_line_height, 7);
    assert_eq!(finished.snapshot.tab_line_height, 5);

    let state = builder.finish(5, 1, 8.0, 16.0);
    assert_eq!(state.window_matrices.len(), 1);
    assert_eq!(state.window_matrices[0].window_id, 41);
}

#[test]
fn buffer_text_window_visibility_retry_request_scrolls_down_from_visible_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("abcdefghijklmnopqrstuvwxyz\n");
        buffer.point_max_char_pos().get() as i64
    };
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let access = RustBufferAccess::new(buffer);
    let rows = vec![
        emitted_row(0, 0, 16, 1, 8),
        emitted_row(1, 16, 16, 9, 16),
        emitted_row(2, 32, 16, 17, 24),
    ];

    let outcome = BufferTextWindowVisibilityRetryRequest::new(
        &rows,
        1,
        0,
        buffer_size,
        30,
        24,
        false,
        false,
        0,
        48,
        &access,
    )
    .decide();

    assert_eq!(outcome.visible_end_lisp(), Some(LispCharPos1::new(24)));
    assert!(outcome.point_beyond_visible_span());
    assert_eq!(outcome.scroll_down_window_start(), Some(24));
    assert_eq!(outcome.retry_window_start(), Some(24));
}

#[test]
fn buffer_text_window_visibility_retry_request_detects_partially_visible_point_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("abcdefghijklmnopqrstuvwxyz\n");
        buffer.point_max_char_pos().get() as i64
    };
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let access = RustBufferAccess::new(buffer);
    let rows = vec![
        emitted_row(0, 0, 20, 1, 10),
        emitted_row(1, 20, 20, 11, 20),
        emitted_row(2, 40, 30, 21, 30),
    ];

    let outcome = BufferTextWindowVisibilityRetryRequest::new(
        &rows,
        1,
        0,
        buffer_size,
        25,
        30,
        false,
        false,
        0,
        60,
        &access,
    )
    .decide();

    assert!(!outcome.point_beyond_visible_span());
    assert_eq!(outcome.point_row_window_start(), Some(10));
    assert_eq!(outcome.retry_window_start(), Some(10));
}

#[test]
fn buffer_text_window_visibility_retry_request_detects_point_line_continuation() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("abcdefghijklmnopqrstuvwxyz\n");
        buffer.point_max_char_pos().get() as i64
    };
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let access = RustBufferAccess::new(buffer);
    let rows = vec![
        emitted_row(0, 0, 16, 1, 10),
        emitted_row(1, 16, 16, 11, 20),
        emitted_row(2, 32, 16, 21, 25),
    ];

    let outcome = BufferTextWindowVisibilityRetryRequest::new(
        &rows,
        1,
        0,
        buffer_size,
        21,
        25,
        false,
        false,
        0,
        48,
        &access,
    )
    .decide();

    assert_eq!(outcome.point_line_window_start(), Some(20));
    assert_eq!(outcome.retry_window_start(), Some(20));
}

#[test]
fn measure_buffer_text_source_range_append_uses_shared_renderer_without_mutating_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("ab");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("measure-buffer-fragment-append", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        LayoutBufferSnapshot::from_buffer(buffer)
    };
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut base_face = face_resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row_with_width(&mut builder, 'x', 7, 0, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let position = DisplayRowPosition { x_px: 8.0, col: 1 };
    let source_range = BufferTextSourceRange::new(CharPos0::new(1), CharPos0::new(2));

    let append_context =
        BufferTextSourceRangeAppendContext::new(&snapshot, buf_id, 7, &base_face, frame);
    let measured_width = append_context
        .measure_source_range_natural_advance_to_text_row(
            &mut TextRowSourceMeasureState::new(
                &mut builder,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            source_range,
            position,
        )
        .expect("measured buffer fragment append");

    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert!(matches!(text[0].glyph_type, GlyphType::Char { ch: 'x' }));
        })
        .expect("current row");

    let (appended, end) = append_context
        .append_source_text_request_to_text_row(
            &mut TextRowSourceRenderState::new(
                &mut builder,
                &mut output_emitter,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            BufferTextSourceTextRequest::new(
                source_range,
                ResolvedBufferTextSourceAdvance::natural(measured_width),
            ),
            position,
        )
        .expect("appended buffer fragment");

    assert_eq!(end.x_px - position.x_px, measured_width);
    assert_eq!(appended.metrics.width_px, measured_width);
}

#[test]
fn buffer_text_source_append_context_uses_resolved_advance() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("a");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-buffer-resolved-fragment", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        LayoutBufferSnapshot::from_buffer(buffer)
    };
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));

    let append_context =
        BufferTextSourceRangeAppendContext::new(&snapshot, buf_id, 7, base_face, frame);
    let (progress, end) = append_context
        .append_source_text_request_to_text_row(
            &mut TextRowSourceRenderState::new(
                &mut builder,
                &mut output_emitter,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            BufferTextSourceTextRequest::new(
                BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
                ResolvedBufferTextSourceAdvance::resolved(13.0),
            ),
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("appended resolved buffer fragment");

    assert_eq!(end, DisplayRowPosition { x_px: 13.0, col: 1 });
    assert_eq!(progress.metrics.width_px, 13.0);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert_eq!(text[0].pixel_width, 13.0);
        })
        .expect("current row");
}

#[test]
fn buffer_text_source_append_context_composes_with_current_row_tail() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("e\u{301}");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-buffer-combining-char", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        LayoutBufferSnapshot::from_buffer(buffer)
    };
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut base_face = face_resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    write_char_to_current_row_with_width(&mut builder, 'e', 7, 0, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));

    let append_context =
        BufferTextSourceRangeAppendContext::new(&snapshot, buf_id, 7, &base_face, frame);
    let (progress, end) = append_context
        .append_source_text_request_to_text_row(
            &mut TextRowSourceRenderState::new(
                &mut builder,
                &mut output_emitter,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            BufferTextSourceTextRequest::new(
                BufferTextSourceRange::new(CharPos0::new(1), CharPos0::new(2)),
                ResolvedBufferTextSourceAdvance::natural(0.0),
            ),
            DisplayRowPosition { x_px: 8.0, col: 1 },
        )
        .expect("appended combining buffer char");

    assert_eq!(end, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(progress.metrics.width_px, 0.0);
    assert_eq!(progress.metrics.width_cols, 0);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert!(matches!(
                &text[0].glyph_type,
                GlyphType::Composite { text } if text.as_ref() == "e\u{301}"
            ));
        })
        .expect("current row");
}

#[test]
fn buffer_text_item_append_context_builds_control_char_item() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("\u{0001}");
    }
    let snapshot = {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        LayoutBufferSnapshot::from_buffer(buffer)
    };
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-buffer-text-item-fragment", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let item = BufferTextSourceAppendItem::ControlChar { ch: '\u{0001}' };
    let source_item = BufferTextSourceItemRequest::new(
        BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
        item.clone(),
    );

    let append_context = BufferTextItemAppendContext::new(&snapshot, buf_id, 7, base_face, frame);
    let measured_width = append_context
        .measure_source_request_width_to_text_row(
            &mut TextRowSourceMeasureState::new(
                &mut builder,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            source_item.clone(),
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("measured buffer text item fragment");
    builder
        .with_current_row_mut(|row| assert!(row.glyphs[1].is_empty()))
        .expect("current row");
    let fallback_width = append_context
        .measure_source_request_width_or_active_face_fallback_to_text_row(
            &mut TextRowSourceMeasureState::new(
                &mut builder,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            BufferTextSourceItemRequest::new(
                BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(0)),
                item.clone(),
            ),
            DisplayRowPosition { x_px: 0.0, col: 0 },
        );
    let edge_width = append_context
        .measure_source_request_width_or_active_face_fallback_to_text_row(
            &mut TextRowSourceMeasureState::new(
                &mut builder,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            source_item.clone(),
            DisplayRowPosition {
                x_px: 80.0,
                col: 10,
            },
        );

    let (progress, end) = append_context
        .append_source_request_to_text_row_and_emit(
            &mut TextRowSourceRenderState::new(
                &mut builder,
                &mut output_emitter,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
            source_item,
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("appended buffer text item fragment");

    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(measured_width, 16.0);
    assert_eq!(fallback_width, 16.0);
    assert_eq!(edge_width, 16.0);
    assert_eq!(progress.metrics.width_px, measured_width);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 2);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: '^' }
            ));
            assert!(matches!(
                text[1].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: 'A' }
            ));
        })
        .expect("current row");
}

#[test]
fn buffer_text_source_append_item_names_nobreak_display_policy() {
    assert_eq!(
        BufferTextSourceAppendItem::nobreak_display('\u{00A0}', 1),
        Some(BufferTextSourceAppendItem::SourceMappedText { text: " ".into() })
    );
    assert_eq!(
        BufferTextSourceAppendItem::nobreak_display('\u{00AD}', 1),
        Some(BufferTextSourceAppendItem::SourceMappedText { text: "-".into() })
    );
    assert_eq!(
        BufferTextSourceAppendItem::nobreak_display('\u{00A0}', 2),
        Some(BufferTextSourceAppendItem::SourceMappedText { text: "\\ ".into() })
    );
    assert_eq!(
        BufferTextSourceAppendItem::nobreak_display('\u{00AD}', 2),
        Some(BufferTextSourceAppendItem::SourceMappedText { text: "\\-".into() })
    );
    assert_eq!(
        BufferTextSourceAppendItem::nobreak_display('\u{00A0}', 0),
        None
    );
    assert_eq!(BufferTextSourceAppendItem::nobreak_display('x', 2), None);
}

#[test]
fn buffer_text_source_special_display_names_precluster_policy() {
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_precluster_char('\u{0001}', 2),
        Some(BufferTextSourceSpecialDisplay::Control(
            BufferTextSourceAppendItem::ControlChar { ch: '\u{0001}' }
        ))
    );
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_precluster_char('\u{007F}', 2),
        Some(BufferTextSourceSpecialDisplay::Control(
            BufferTextSourceAppendItem::ControlChar { ch: '\u{007F}' }
        ))
    );
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_precluster_char('\u{00A0}', 2),
        Some(BufferTextSourceSpecialDisplay::Nobreak(
            BufferTextSourceAppendItem::SourceMappedText { text: "\\ ".into() }
        ))
    );
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_precluster_char('\n', 2),
        None
    );
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_precluster_char('\t', 2),
        None
    );
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_precluster_char('x', 2),
        None
    );
}

#[test]
fn buffer_text_source_special_display_names_cluster_policy() {
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_cluster_state(BufferTextSourceClusterState::for_char(
            '\u{200E}',
            Some(('a', false)),
        )),
        Some(BufferTextSourceSpecialDisplay::Glyphless(
            BufferTextSourceAppendItem::Glyphless {
                ch: '\u{200E}',
                method: GlyphlessMethod::ZeroWidth,
            }
        ))
    );
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_cluster_state(BufferTextSourceClusterState::for_char(
            '\u{FE0F}',
            Some(('\u{2764}', false)),
        )),
        None
    );
    assert_eq!(
        BufferTextSourceSpecialDisplay::for_cluster_state(BufferTextSourceClusterState::for_char(
            'x', None,
        )),
        None
    );
}

#[test]
fn buffer_text_source_char_names_range_and_precluster_policy() {
    let source_char = BufferTextSourceChar::new('\u{00A0}', CharPos0::new(4), 2);
    let request = source_char
        .nobreak_special_request()
        .expect("nobreak source char should produce source request");

    assert_eq!(
        source_char.range(),
        BufferTextSourceRange::new(CharPos0::new(4), CharPos0::new(5))
    );
    assert!(source_char.control_special_request().is_none());
    assert_eq!(
        source_char
            .special_request(None)
            .map(|request| request.kind()),
        Some(BufferTextSourceSpecialDisplayKind::Nobreak)
    );
    assert_eq!(
        request.append_plan_at(DisplayRowPosition { x_px: 0.0, col: 0 }),
        BufferTextSpecialSourceCharRequest::new(
            &source_char,
            BufferTextSourceSpecialDisplay::Nobreak(BufferTextSourceAppendItem::SourceMappedText {
                text: "\\ ".into()
            }),
        )
        .append_plan_at(DisplayRowPosition { x_px: 0.0, col: 0 })
    );
}

#[test]
fn buffer_text_source_char_names_cluster_policy() {
    let source_char = BufferTextSourceChar::new('\u{FE0F}', CharPos0::new(1), 2);
    let cluster_tail = Some(('\u{2764}', false));

    assert_eq!(
        source_char.cluster_state(cluster_tail),
        BufferTextSourceClusterState::for_char('\u{FE0F}', cluster_tail)
    );
    assert_eq!(source_char.cluster_special_request(cluster_tail), None);

    let standalone_joiner = BufferTextSourceChar::new('\u{200D}', CharPos0::new(2), 2);
    assert_eq!(
        standalone_joiner
            .special_request(None)
            .map(|request| request.append_plan_at(DisplayRowPosition { x_px: 0.0, col: 0 })),
        BufferTextSourceSpecialDisplay::for_cluster_state(BufferTextSourceClusterState::for_char(
            '\u{200D}', None
        ))
        .map(|display| BufferTextSpecialSourceCharRequest::new(
            &standalone_joiner,
            display
        )
        .append_plan_at(DisplayRowPosition { x_px: 0.0, col: 0 }))
    );
}

#[test]
fn buffer_text_source_append_item_names_fallback_width_policy() {
    assert_eq!(
        BufferTextSourceAppendItem::ControlChar { ch: '\u{0001}' }.fallback_width_policy(),
        BufferTextSourceFallbackWidthPolicy::Columns(2)
    );
    assert_eq!(
        BufferTextSourceAppendItem::SourceMappedText { text: "\\ ".into() }.fallback_width_policy(),
        BufferTextSourceFallbackWidthPolicy::Columns(2)
    );
    assert_eq!(
        BufferTextSourceAppendItem::SourceMappedText { text: "".into() }
            .fallback_width_policy()
            .width_px(8.0),
        8.0
    );
    assert_eq!(
        BufferTextSourceAppendItem::Glyphless {
            ch: '\u{200E}',
            method: GlyphlessMethod::ZeroWidth,
        }
        .fallback_width_policy()
        .width_px(8.0),
        8.0
    );
}

#[test]
fn buffer_text_source_append_item_names_glyphless_display_policy() {
    let variation_selector_state =
        BufferTextSourceClusterState::for_char('\u{FE0F}', Some(('\u{2764}', false)));
    assert!(variation_selector_state.is_cluster_continuation());

    assert_eq!(
        BufferTextSourceAppendItem::glyphless_display(BufferTextSourceClusterState::for_char(
            '\u{0080}', None,
        )),
        Some(BufferTextSourceAppendItem::Glyphless {
            ch: '\u{0080}',
            method: GlyphlessMethod::HexCode,
        })
    );
    assert_eq!(
        BufferTextSourceAppendItem::glyphless_display(BufferTextSourceClusterState::for_char(
            '\u{FE0F}', None,
        )),
        Some(BufferTextSourceAppendItem::Glyphless {
            ch: '\u{FE0F}',
            method: GlyphlessMethod::ZeroWidth,
        })
    );
    assert_eq!(
        BufferTextSourceAppendItem::glyphless_display(variation_selector_state),
        None
    );
    assert_eq!(
        BufferTextSourceAppendItem::glyphless_display(BufferTextSourceClusterState::for_char(
            '\u{200E}',
            Some(('a', false)),
        )),
        Some(BufferTextSourceAppendItem::Glyphless {
            ch: '\u{200E}',
            method: GlyphlessMethod::ZeroWidth,
        })
    );
    assert_eq!(
        BufferTextSourceAppendItem::glyphless_display(BufferTextSourceClusterState::for_char(
            'x', None,
        )),
        None
    );
}

#[test]
fn buffer_text_item_append_context_builds_mapped_item() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval.frame_manager_mut().create_frame(
        "append-buffer-source-mapped-fragment",
        320,
        120,
        buf_id,
    );
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let append_context =
        BufferTextRowAppendContext::new(&snapshot, buf_id, &surface, &active_face, 0.0, 16.0);
    let source_char = BufferTextSourceChar::new('\u{00A0}', CharPos0::new(0), 2);
    let source_request = source_char
        .special_request(None)
        .expect("nobreak source char should map to a display item");
    let prepared_append = append_context.prepare_special_source_char_at(
        &geometry,
        &mut TextRowSourceMeasureState::new(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
        ),
        source_request,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    );
    assert_eq!(
        prepared_append.kind(),
        BufferTextSourceSpecialDisplayKind::Nobreak
    );
    assert_eq!(prepared_append.overflow_decision(0.0, 80.0, false), None);
    assert_eq!(prepared_append.overflow_action(0.0, 80.0, false), None);
    let mut params = test_display_space_window_params();
    params.nobreak_char_fg = 0x00ff00;
    let mut policy_face_ids = FrameFaceIdAllocator::new(30);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;
    let mut word_wrap = WordWrapRenderState::new(true);
    let mut charpos = 8;
    let mut end_x = 0.0;
    let mut end_col = 0;
    let continuation = prepared_append.append_to_text_row_and_apply(
        &append_context,
        &geometry,
        &params,
        &mut BufferTextSpecialSourceCharRenderState::new(
            &mut policy_face_ids,
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            &mut face_scan,
            &mut word_wrap,
            &mut end_x,
            &mut end_col,
            &mut charpos,
        ),
    );
    assert_eq!(continuation, BufferTextSourceAppendContinuation::Rendered);
    assert!(face_scan.should_resolve_at(1));
    assert_eq!(policy_face_ids.finish(), 31);

    assert_eq!(end_x, 16.0);
    assert_eq!(end_col, 2);
    assert_eq!(charpos, 9);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 2);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: '\\' }
            ));
            assert!(matches!(
                text[1].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: ' ' }
            ));
        })
        .expect("current row");
}

#[test]
fn buffer_text_item_append_context_builds_glyphless_item() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-buffer-glyphless-fragment", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let snapshot = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let append_context =
        BufferTextRowAppendContext::new(&snapshot, buf_id, &surface, &active_face, 0.0, 16.0);
    let source_char = BufferTextSourceChar::new('\u{fff0}', CharPos0::new(0), 2);
    let source_request = source_char
        .special_request(None)
        .expect("glyphless source char should map to a display item");
    let prepared_append = append_context.prepare_special_source_char_at(
        &geometry,
        &mut TextRowSourceMeasureState::new(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
        ),
        source_request,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    );
    assert_eq!(
        prepared_append.kind(),
        BufferTextSourceSpecialDisplayKind::Glyphless
    );
    assert_eq!(prepared_append.overflow_decision(0.0, 80.0, false), None);
    assert_eq!(prepared_append.overflow_action(0.0, 80.0, false), None);
    let mut policy_face_ids = FrameFaceIdAllocator::new(30);
    let params = test_display_space_window_params();
    let append_outcome = prepared_append
        .append_to_text_row(
            &append_context,
            &geometry,
            &params,
            &mut policy_face_ids,
            &mut TextRowSourceRenderState::new(
                &mut builder,
                &mut output_emitter,
                &mut eval,
                &mut font_metrics,
                &face_resolver,
            ),
        )
        .expect("appended glyphless buffer text item fragment");
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;
    let mut end_x = 0.0;
    let mut end_col = 0;
    append_outcome.apply_to_text_row_state(&mut face_scan, &mut end_x, &mut end_col);
    assert!(!face_scan.should_resolve_at(1));
    assert_eq!(policy_face_ids.finish(), 30);

    assert_eq!(end_x, 48.0);
    assert_eq!(end_col, 6);
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Glyphless { ch: '\u{fff0}' }
            ));
        })
        .expect("current row");
}

#[test]
fn append_lisp_string_to_text_row_stops_at_row_break() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-display-item-source", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));

    let end = append_lisp_string_to_text_row(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        Value::string("a\nb"),
        1,
        &face_resolver,
        base_face,
        7,
        &mut face_ids,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    );

    assert_eq!(end, DisplayRowPosition { x_px: 8.0, col: 1 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: 'a' }
            ));
        })
        .expect("current row");
}

#[test]
fn lisp_string_source_append_context_preserves_source_after_row_break() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("render-lisp-source-row-break", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut font_metrics = None;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let first_geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let second_geometry = DisplayRowGeometryState::new(1, 16.0, 0.0, 16.0, 12.0);

    let request = LispStringSourceAppendRequest::new(
        DisplayRowPosition { x_px: 0.0, col: 0 },
        LispStringSourceId::OVERLAY_STRING,
        Value::string("a\nb"),
    );
    let mut append_context = LispStringSourceRowAppendSession::new(
        request, 7, base_face, &surface, 0.0, 16.0, 12.0, 8.0, 16.0,
    )
    .expect("lisp string source session");

    let first = append_context
        .render_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            &mut face_ids,
            &first_geometry,
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("first lisp source append");

    assert_eq!(first.end, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(
        first.stop,
        crate::display_row::DisplayRowRenderStop::RowBreak
    );

    let second = append_context
        .render_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            &mut face_ids,
            &second_geometry,
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("second lisp source append");

    assert_eq!(second.end, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(
        second.stop,
        crate::display_row::DisplayRowRenderStop::SourceExhausted
    );
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 2);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: 'a' }
            ));
            assert!(matches!(
                text[1].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: 'b' }
            ));
        })
        .expect("current row");
}

#[test]
fn append_lisp_string_to_text_row_resolves_image_display_property_through_display_host() {
    let mut eval = Context::new();
    let requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingAppendImageHost {
        requests: Arc::clone(&requests),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-lisp-string-image", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 6.0);

    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00112233, 0x00445566, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let text_bounds = Rect::new(10.0, 20.0, 160.0, 64.0);
    builder.begin_window_with_text_bounds(
        77,
        1,
        24,
        Rect::new(0.0, 0.0, 200.0, 80.0),
        text_bounds,
        true,
    );
    builder.begin_row(0, GlyphRowRole::Text);
    let value = Value::string_with_text_properties(
        "A",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("image"),
                    Value::keyword("type"),
                    Value::symbol("png"),
                    Value::keyword("file"),
                    Value::string("./tmp/append-lisp-string.png"),
                ]),
            ]),
        }],
    );
    let frame = test_append_frame_at(
        0,
        0.0,
        6.0,
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 160.0,
            text_width: 160.0,
            line_number_width: 0.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 12.0,
            char_width: 8.0,
            space_width: 8.0,
            default_row_height: 16.0,
        },
        DisplayTabPolicy::every(8),
    );

    let end = append_lisp_string_to_text_row(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        value,
        1,
        &face_resolver,
        base_face,
        7,
        &mut face_ids,
        frame,
        DisplayRowPosition { x_px: 16.0, col: 2 },
    );

    assert_eq!(
        end,
        DisplayRowPosition {
            x_px: 80.0,
            col: 10
        }
    );
    builder.end_row();
    builder.end_window();
    let state = builder.finish(24, 1, 8.0, 16.0);
    let image = state.images.first().expect("image side item");
    assert_eq!(image.window_id, 77);
    assert_eq!(image.row_role, GlyphRowRole::Text);
    assert_eq!(image.clip_rect, Some(text_bounds));
    assert_eq!(image.image_id, 42);
    assert_eq!(image.x, 16.0);
    assert_eq!(image.y, 0.0);
    assert_eq!(image.width, 64.0);
    assert_eq!(image.height, 32.0);
    let requests = requests.lock().expect("image requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].fg_color, 0x00112233);
    assert_eq!(requests[0].bg_color, 0x00445566);
}

struct SourceMappedTextWidthByFace;

impl SourceMappedTextWidthByFace {
    fn new() -> Self {
        Self
    }
}

impl DisplayRowRenderPolicy for SourceMappedTextWidthByFace {
    fn measurement_for(
        &mut self,
        item: &crate::display_item::DisplayItem,
        face_id: u32,
        _font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        let DisplayItemKind::SourceMappedText(text) = &item.kind else {
            return DisplayRowItemMeasurement::Default;
        };
        let advance_px = if face_id == 20 { 13.0 } else { 11.0 };
        let advances = text
            .text
            .char_indices()
            .enumerate()
            .map(|(char_offset, (byte_offset, _))| {
                DisplayTextRunAdvance::new(char_offset, byte_offset, advance_px)
            })
            .collect();
        DisplayRowItemMeasurement::TextRun(DisplayTextRunMeasurement::Measured(advances))
    }
}

#[test]
fn display_replacement_active_face_measurer_names_cursor_and_display_width_policy() {
    let active_face = test_active_face_state(7, 8.0);
    let measurer = DisplayReplacementActiveFaceMeasurer::from_active_face_state(&active_face);
    let mut font_metrics = None;

    assert_eq!(
        measurer.replacement_string_cursor_slot_width_px(&mut font_metrics, "ab", 8.0),
        8.0
    );
    assert_eq!(
        measurer.replacement_string_cursor_slot_width_px(&mut font_metrics, "", 9.0),
        9.0
    );
    assert_eq!(
        measurer.source_char_width_px(&mut font_metrics, 'x', 8.0),
        8.0
    );
}

#[test]
fn display_replacement_string_append_item_names_cursor_and_source_policy() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let value = Value::string("ab");
    let item = DisplayReplacementStringAppendItem::display_property_string(
        value,
        CharPos0::new(4),
        DisplayPropertySource::TextProperty,
        9,
        &active_face,
        &mut font_metrics,
        8.0,
    )
    .expect("display property string item");

    assert_eq!(item.cursor_slot_width_px(), 8.0);
    assert!(!item.is_empty());
    let replacement_source = crate::display_source::BufferDisplayReplacementSource::new(
        buf_id,
        CharPos0::new(4),
        EmacsBytePos::new(4),
    );
    let request =
        item.source_append_request(replacement_source, DisplayRowPosition { x_px: 2.0, col: 1 });
    assert_eq!(request.value, value);
    assert_eq!(
        request.source_id,
        LispStringSourceId::display_replacement(9)
    );
    assert_eq!(request.position, DisplayRowPosition { x_px: 2.0, col: 1 });
    assert_eq!(
        item.origin(),
        DisplayOrigin::DisplayPropertyString {
            anchor_charpos: CharPos0::new(4),
            source: DisplayPropertySource::TextProperty,
        }
    );
    assert_eq!(
        item.base_face_policy(),
        BaseFacePolicy::DisplayPropertyUnderlyingFace
    );

    let empty = DisplayReplacementStringAppendItem::display_property_string(
        Value::string(""),
        CharPos0::new(4),
        DisplayPropertySource::TextProperty,
        10,
        &active_face,
        &mut font_metrics,
        9.0,
    )
    .expect("empty display property string item");
    assert!(empty.is_empty());
    assert_eq!(empty.cursor_slot_width_px(), 9.0);
}

#[test]
fn display_replacement_string_append_item_measures_source_text_from_active_face() {
    let _eval = Context::new();
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = Some(crate::font_metrics::FontMetricsService::new());
    let item = DisplayReplacementStringAppendItem::display_property_string(
        Value::string("abc"),
        CharPos0::new(0),
        DisplayPropertySource::TextProperty,
        11,
        &active_face,
        &mut font_metrics,
        8.0,
    )
    .expect("display property string item");
    let mut measurer = item.string_item_measurer();
    let source_item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::synthetic(11, 0, 3),
        RenderFaceRef::FaceId(7),
        DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new("abc")),
    );

    let measurement =
        DisplayRowRenderPolicy::measurement_for(&mut measurer, &source_item, 7, &mut font_metrics);

    let DisplayRowItemMeasurement::TextRun(measurement) = measurement else {
        panic!("replacement string text should use a direct text-run measurement");
    };
    let DisplayTextRunMeasurement::Measured(advances) = measurement else {
        panic!("replacement string run should be measured");
    };
    assert_eq!(
        advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset))
            .collect::<Vec<_>>(),
        vec![(0, 0), (1, 1), (2, 2)]
    );
}

#[test]
fn display_property_replacement_append_item_resolves_string_replacement() {
    let _eval = Context::new();
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let value = Value::string("ab");
    let classification = classify_display_property(value);

    let item = DisplayPropertyReplacementAppendItem::resolve(
        &classification,
        value,
        CharPos0::new(4),
        b"x",
        &active_face,
        &mut font_metrics,
        0.0,
        0.0,
        &test_display_space_window_params(),
        None,
    )
    .expect("string replacement append item");

    let DisplayPropertyReplacementAppendItem::String(item) = item else {
        panic!("expected string replacement append item");
    };
    assert_eq!(item.cursor_slot_width_px(), 8.0);
    assert!(!item.is_empty());
}

#[test]
fn display_property_replacement_append_item_resolves_stretch_replacement() {
    let _eval = Context::new();
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let value = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("relative-width"),
        Value::fixnum(2),
        Value::keyword("height"),
        Value::fixnum(3),
    ]);
    let classification = classify_display_property(value);

    let item = DisplayPropertyReplacementAppendItem::resolve(
        &classification,
        value,
        CharPos0::new(4),
        b"x",
        &active_face,
        &mut font_metrics,
        0.0,
        0.0,
        &test_display_space_window_params(),
        None,
    )
    .expect("stretch replacement append item");

    let DisplayPropertyReplacementAppendItem::Stretch(item) = item else {
        panic!("expected stretch replacement append item");
    };
    assert_eq!(item.width_px(), 16.0);
    assert_eq!(item.height_px(), 48.0);
}

#[test]
fn display_property_replacement_append_item_resolves_media_replacement() {
    let _eval = Context::new();
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let media = DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
        xwidget_id: 17,
        width: 42.0,
        height: 11.0,
    });
    let classification = DisplayPropertyClassification {
        replacement: Some(DisplayReplacementProperty::Media(
            DisplayMediaReplacementProperty::Xwidget(media),
        )),
        modifiers: Default::default(),
    };

    let item = DisplayPropertyReplacementAppendItem::resolve(
        &classification,
        Value::NIL,
        CharPos0::new(4),
        b"x",
        &active_face,
        &mut font_metrics,
        0.0,
        0.0,
        &test_display_space_window_params(),
        None,
    )
    .expect("media replacement append item");

    let DisplayPropertyReplacementAppendItem::Media(
        DisplayReplacementMediaAppendResolution::Media(item),
    ) = item
    else {
        panic!("expected media replacement append item");
    };
    assert_eq!(item.width_px(), 42.0);
    assert_eq!(item.display_height_px(), 11.0);
}

#[test]
fn display_property_replacement_append_item_names_cursor_policy() {
    let _eval = Context::new();
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let value = Value::string("ab");
    let classification = classify_display_property(value);
    let string = DisplayPropertyReplacementAppendItem::resolve(
        &classification,
        value,
        CharPos0::new(4),
        b"x",
        &active_face,
        &mut font_metrics,
        0.0,
        0.0,
        &test_display_space_window_params(),
        None,
    )
    .expect("string replacement append item");

    assert_eq!(
        string.cursor_policy(),
        DisplayPropertyReplacementCursorPolicy::TextSlot {
            width_px: 8.0,
            stretch_like: false,
        }
    );

    let stretch = DisplayPropertyReplacementAppendItem::Stretch(
        DisplayReplacementStretchAppendItem::from_space_extents(13.0, 16.0, 12.0, 8.0),
    );
    assert_eq!(
        stretch.cursor_policy(),
        DisplayPropertyReplacementCursorPolicy::TextSlot {
            width_px: 13.0,
            stretch_like: true,
        }
    );

    let media = DisplayPropertyReplacementAppendItem::Media(
        DisplayReplacementMediaAppendResolution::Media(DisplayReplacementMediaAppendItem::new(
            DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
                xwidget_id: 17,
                width: 42.0,
                height: 11.0,
            }),
            &active_face,
            true,
        )),
    );
    assert_eq!(
        media.cursor_policy(),
        DisplayPropertyReplacementCursorPolicy::DisplayBox {
            width_px: 42.0,
            cursor_face_height_px: 18.0,
            cursor_face_ascent_px: 13.0,
        }
    );

    let placeholder = DisplayPropertyReplacementAppendItem::Media(
        DisplayReplacementMediaAppendResolution::Placeholder(
            DisplayReplacementSourceMappedTextAppendItem::new("[img]"),
        ),
    );
    assert_eq!(
        placeholder.cursor_policy(),
        DisplayPropertyReplacementCursorPolicy::FaceChar
    );
}

#[test]
fn display_property_replacement_append_request_keeps_item_policy_and_start_position() {
    let item = DisplayPropertyReplacementAppendItem::Stretch(
        DisplayReplacementStretchAppendItem::from_space_extents(13.0, 16.0, 12.0, 8.0),
    );
    let request = DisplayPropertyReplacementAppendRequest::new(
        crate::display_source::BufferDisplayReplacementSource::new(
            BufferId(7),
            CharPos0::new(3),
            EmacsBytePos::new(12),
        ),
        item,
        -2.0,
        18.0,
        DisplayRowPosition { x_px: 24.0, col: 4 },
    );

    assert_eq!(
        request.cursor_policy(),
        DisplayPropertyReplacementCursorPolicy::TextSlot {
            width_px: 13.0,
            stretch_like: true,
        }
    );
    assert_eq!(
        request.start_position(),
        DisplayRowPosition { x_px: 24.0, col: 4 }
    );
    let DisplayPropertyReplacementAppendItem::Stretch(item) = request.into_item() else {
        panic!("expected stretch replacement item");
    };
    assert_eq!(item.height_px(), 16.0);
    assert_eq!(item.ascent_px(), 12.0);
}

#[test]
fn display_property_replacement_append_resolve_request_builds_append_request() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = current_buffer_snapshot(&eval, buf_id);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let value = Value::string("ab");
    let classification = classify_display_property(value);
    let params = test_display_space_window_params();
    let request = DisplayPropertyReplacementAppendResolveRequest::new(
        &classification,
        value,
        crate::display_source::BufferDisplayReplacementSource::new(
            buf_id,
            CharPos0::new(3),
            EmacsBytePos::new(12),
        ),
        CharPos0::new(3),
        b"x",
        &active_face,
        24.0,
        8.0,
        &params,
        -2.0,
        18.0,
        DisplayRowPosition { x_px: 24.0, col: 4 },
    )
    .resolve(&mut font_metrics, None)
    .expect("display replacement append request");

    assert_eq!(
        request.cursor_policy(),
        DisplayPropertyReplacementCursorPolicy::TextSlot {
            width_px: 8.0,
            stretch_like: false,
        }
    );
    assert_eq!(
        request.start_position(),
        DisplayRowPosition { x_px: 24.0, col: 4 }
    );
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let plan = request.into_plan(
        &buffer,
        &face_resolver,
        &active_face,
        &mut face_ids,
        &mut builder,
    );
    let string_append_request = plan
        .string_append_request()
        .expect("string replacement lowers to string append request");
    assert_eq!(
        string_append_request.origin(),
        DisplayOrigin::DisplayPropertyString {
            anchor_charpos: CharPos0::new(3),
            source: DisplayPropertySource::TextProperty,
        }
    );
    assert_eq!(
        string_append_request.base_face_policy(),
        BaseFacePolicy::DisplayPropertyUnderlyingFace
    );
    assert!(string_append_request.replacement_base_face.is_some());
}

#[test]
fn buffer_display_property_replacement_outcome_applies_walk_state_and_cursor() {
    let outcome = BufferDisplayPropertyTextReplacementOutcome {
        replacement: DisplayPropertyReplacementAppendOutcome {
            start_position: DisplayRowPosition { x_px: 4.0, col: 1 },
            end_position: DisplayRowPosition { x_px: 12.0, col: 2 },
            cursor_policy: DisplayPropertyReplacementCursorPolicy::FaceChar,
        },
        skip_to: 4,
    };
    let mut byte_idx = "a".len();
    let mut charpos = 1;
    let mut x = 4.0;
    let mut col = 1;

    outcome.apply_to_walk_state(
        "a界b\n".as_bytes(),
        &mut byte_idx,
        &mut charpos,
        &mut x,
        &mut col,
    );

    assert_eq!(byte_idx, "a界b\n".len());
    assert_eq!(charpos, 4);
    assert_eq!(x, 12.0);
    assert_eq!(col, 2);
    assert_eq!(outcome.skip_to(), 4);

    let active_face = test_active_face_state(7, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut cursor_info = CursorCaptureState::new();
    outcome.capture_cursor_info_if_point(
        &mut cursor_info,
        &active_face,
        &geometry,
        2,
        1,
        "a".len(),
    );
    let cursor = cursor_info.captured().expect("captured replacement cursor");
    assert_eq!(cursor.x, 4.0);
    assert_eq!(cursor.byte_idx, "a".len());
    assert_eq!(cursor.col, 1);
    assert_eq!(cursor.slot_width, Some(8.0));
}

#[test]
fn buffer_display_property_append_action_applies_replacement_walk_state() {
    let outcome = BufferDisplayPropertyTextReplacementOutcome {
        replacement: DisplayPropertyReplacementAppendOutcome {
            start_position: DisplayRowPosition { x_px: 4.0, col: 1 },
            end_position: DisplayRowPosition { x_px: 12.0, col: 2 },
            cursor_policy: DisplayPropertyReplacementCursorPolicy::FaceChar,
        },
        skip_to: 4,
    };
    let action = BufferDisplayPropertyTextAppendAction::Replacement(outcome);
    let active_face = test_active_face_state(7, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut cursor_info = CursorCaptureState::new();
    let mut raise_span = ActiveDisplayPropertySpan::inactive();
    let mut height_span = ActiveDisplayPropertySpan::inactive();
    let mut face_scan = FaceScanCheckpoint::initial();
    let mut byte_idx = "a".len();
    let mut charpos = 1;
    let mut x = 4.0;
    let mut col = 1;

    let walk_outcome = action.apply_to_buffer_walk_state(
        "a界b\n".as_bytes(),
        &mut byte_idx,
        &mut charpos,
        &mut x,
        &mut col,
        &mut cursor_info,
        &active_face,
        &geometry,
        2,
        &mut raise_span,
        &mut height_span,
        &mut face_scan,
    );

    assert_eq!(
        walk_outcome,
        BufferDisplayPropertyTextWalkOutcome::ReplacementConsumed
    );
    assert!(walk_outcome.should_continue_buffer_walk());
    assert!(!walk_outcome.should_resolve_face());
    assert_eq!(byte_idx, "a界b\n".len());
    assert_eq!(charpos, 4);
    assert_eq!(x, 12.0);
    assert_eq!(col, 2);
    assert!(cursor_info.captured().is_some());
    assert_eq!(raise_span.value(), None);
    assert_eq!(height_span.value(), None);
}

#[test]
fn display_property_replacement_resolve_request_appends_and_reports_outcome() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = current_buffer_snapshot(&eval, buf_id);
    let frame_id = eval.frame_manager_mut().create_frame(
        "display-property-replacement-request",
        320,
        120,
        buf_id,
    );
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 32.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let mut geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let active_face = test_active_face_state(7, 8.0);
    let value = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("relative-width"),
        Value::fixnum(2),
        Value::keyword("height"),
        Value::fixnum(3),
    ]);
    let classification = classify_display_property(value);
    let params = test_display_space_window_params();

    let outcome = DisplayPropertyReplacementAppendResolveRequest::for_text_property(
        &classification,
        value,
        buf_id,
        CharPos0::new(3),
        EmacsBytePos::new(12),
        b"x",
        &active_face,
        24.0,
        8.0,
        &params,
        -2.0,
        18.0,
        DisplayRowPosition { x_px: 24.0, col: 4 },
    )
    .resolve_and_append_to_text_row(
        &buffer,
        &mut eval,
        &mut output_emitter,
        &mut builder,
        &mut font_metrics,
        &face_resolver,
        &mut face_ids,
        &surface,
        &mut geometry,
    )
    .expect("display replacement outcome");

    assert_eq!(
        outcome.start_position(),
        DisplayRowPosition { x_px: 24.0, col: 4 }
    );
    assert_eq!(
        outcome.end_position(),
        DisplayRowPosition { x_px: 40.0, col: 6 }
    );
    let cursor = outcome.cursor_info(
        &active_face,
        geometry.text_position(
            outcome.start_position().x_px,
            0,
            outcome.start_position().col,
        ),
    );
    assert_eq!(cursor.x, 24.0);
    assert_eq!(cursor.slot_width, Some(16.0));
    let metrics = geometry.row_metrics_snapshot(0);
    assert!(metrics.height > 16.0);
    assert!(metrics.ascent > 12.0);
}

#[test]
fn buffer_display_property_render_context_returns_modifier_action_from_checkpoint() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert("abc");
        let _ = eval
            .buffer_manager_mut()
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                EmacsByteRange::from_usize(0, 1),
                Value::symbol("display"),
                Value::list(vec![Value::keyword("raise"), Value::make_float(0.5)]),
            );
    }
    let frame_id = eval.frame_manager_mut().create_frame(
        "display-property-render-context-modifier",
        320,
        120,
        buf_id,
    );
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let buffer = current_buffer_snapshot(&eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let mut row_geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let params = test_display_space_window_params();
    let mut checkpoints = TextPropertyScanCheckpoints::new(0);

    let action = BufferDisplayPropertyTextRenderContext::new(
        buf_id,
        0,
        b"abc",
        &active_face,
        0.0,
        0.0,
        &params,
        0.0,
        16.0,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    )
    .resolve_and_append_at_checkpoint(
        &buffer,
        &mut eval,
        &mut output_emitter,
        &mut builder,
        &mut font_metrics,
        &face_resolver,
        &mut face_ids,
        &surface,
        &mut row_geometry,
        &mut checkpoints,
        0,
        0,
        3,
    );

    match action {
        BufferDisplayPropertyTextAppendAction::Modifiers(modifiers) => {
            assert_eq!(modifiers.raise_offset_px(), Some(-8.0));
            assert_eq!(modifiers.height_factor(), None);
            assert_eq!(modifiers.next_change(), 1);
        }
        _ => panic!("expected modifier action"),
    }
    assert_eq!(checkpoints.display_next(), 1);
}

#[test]
fn buffer_display_property_checkpoint_render_request_applies_modifier_and_resolves_face() {
    let mut context = RowTransitionTestContext::new("display-property-checkpoint-request");
    let buf_id = context
        .eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = context
            .eval
            .buffer_manager_mut()
            .get_mut(buf_id)
            .expect("buffer");
        buffer.insert("abc");
        let _ = context
            .eval
            .buffer_manager_mut()
            .put_buffer_text_property_in_emacs_byte_range(
                buf_id,
                EmacsByteRange::from_usize(0, 1),
                Value::symbol("display"),
                Value::list(vec![Value::keyword("height"), Value::fixnum(2)]),
            );
    }
    let buffer = current_buffer_snapshot(&context.eval, buf_id);
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let default_face = face_resolver.default_face().clone();
    let measurement_policy = DisplayRowMeasurementPolicy::for_frame(false);
    let mut font_metrics = None;
    let measured = measurement_policy.measured_face(
        7,
        &default_face,
        None,
        8.0,
        DisplayRowFallbackMetrics::from_default_face_extents(8.0, 16.0, 12.0),
        &mut font_metrics,
    );
    let mut active_face = DisplayRowActiveFaceState::new(default_face.clone(), measured);
    let mut face_scan = FaceScanCheckpoint::initial();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut row_extend = DisplayRowScopedValue::inactive();
    let mut box_face = BoxFaceRowState::inactive();
    let mut checkpoints = TextPropertyScanCheckpoints::new(0);
    let mut byte_idx = 0;
    let mut charpos = 0;
    let mut x = 0.0;
    let mut col = 0;
    let mut cursor_info = CursorCaptureState::new();
    let mut raise_span = ActiveDisplayPropertySpan::inactive();
    let mut height_span = ActiveDisplayPropertySpan::inactive();
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let params = test_display_space_window_params();

    let outcome = BufferDisplayPropertyCheckpointRenderRequest::new(
        BufferCurrentFaceResolutionContext::new(
            &buffer,
            &face_resolver,
            measurement_policy,
            &default_face,
            8.0,
            12.0,
            16.0,
            8.0,
            16.0,
            12.0,
            false,
        ),
        buf_id,
        0,
        b"abc",
        x,
        0.0,
        &params,
        0.0,
        16.0,
        DisplayRowPosition { x_px: x, col },
        charpos,
        byte_idx,
        3,
    )
    .render_and_apply(BufferDisplayPropertyCheckpointRenderState {
        output_emitter: &mut context.output_emitter,
        builder: &mut context.builder,
        evaluator: &mut context.eval,
        font_metrics: &mut font_metrics,
        face_ids: &mut face_ids,
        append_surface: &surface,
        row_geometry: &mut context.geometry,
        checkpoints: &mut checkpoints,
        face_scan: &mut face_scan,
        active_face_state: &mut active_face,
        row_extend: &mut row_extend,
        box_face: &mut box_face,
        byte_idx: &mut byte_idx,
        charpos: &mut charpos,
        x: &mut x,
        col: &mut col,
        cursor_info: &mut cursor_info,
        raise_span: &mut raise_span,
        height_span: &mut height_span,
        point_charpos: 99,
    });

    assert_eq!(
        outcome,
        BufferDisplayPropertyTextWalkOutcome::FaceStateChanged
    );
    assert!(!outcome.should_continue_buffer_walk());
    assert_eq!(height_span.value(), Some(2.0));
    assert_eq!(active_face.face_id(), 21);
    assert_eq!(face_ids.allocate(), 22);
    assert_eq!(byte_idx, 0);
    assert_eq!(charpos, 0);
    assert_eq!(checkpoints.display_next(), 1);
}

#[test]
fn buffer_display_property_text_modifier_action_applies_walk_state() {
    let action = BufferDisplayPropertyTextModifierAction::new_for_test(Some(-4.0), Some(1.5), 11);
    let mut raise_span = ActiveDisplayPropertySpan::inactive();
    let mut height_span = ActiveDisplayPropertySpan::inactive();
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;

    let outcome = action.apply_to_walk_state(&mut raise_span, &mut height_span, &mut face_scan);

    assert!(outcome.height_face_changed());
    assert_eq!(raise_span.value(), Some(-4.0));
    assert_eq!(height_span.value(), Some(1.5));
    assert!(face_scan.should_resolve_at(0));
}

#[test]
fn buffer_display_property_append_action_applies_modifier_walk_state() {
    let action = BufferDisplayPropertyTextAppendAction::Modifiers(
        BufferDisplayPropertyTextModifierAction::new_for_test(Some(-4.0), Some(1.5), 11),
    );
    let active_face = test_active_face_state(7, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut cursor_info = CursorCaptureState::new();
    let mut raise_span = ActiveDisplayPropertySpan::inactive();
    let mut height_span = ActiveDisplayPropertySpan::inactive();
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;
    let mut byte_idx = 1;
    let mut charpos = 1;
    let mut x = 4.0;
    let mut col = 1;

    let walk_outcome = action.apply_to_buffer_walk_state(
        b"abc",
        &mut byte_idx,
        &mut charpos,
        &mut x,
        &mut col,
        &mut cursor_info,
        &active_face,
        &geometry,
        2,
        &mut raise_span,
        &mut height_span,
        &mut face_scan,
    );

    assert_eq!(
        walk_outcome,
        BufferDisplayPropertyTextWalkOutcome::FaceStateChanged
    );
    assert!(!walk_outcome.should_continue_buffer_walk());
    assert!(walk_outcome.should_resolve_face());
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 1);
    assert_eq!(x, 4.0);
    assert_eq!(col, 1);
    assert!(cursor_info.captured().is_none());
    assert_eq!(raise_span.value(), Some(-4.0));
    assert_eq!(height_span.value(), Some(1.5));
    assert!(face_scan.should_resolve_at(0));
}

#[test]
fn buffer_display_property_append_action_none_keeps_walk_state() {
    let action = BufferDisplayPropertyTextAppendAction::None;
    let active_face = test_active_face_state(7, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let mut cursor_info = CursorCaptureState::new();
    let mut raise_span = ActiveDisplayPropertySpan::inactive();
    let mut height_span = ActiveDisplayPropertySpan::inactive();
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;
    let mut byte_idx = 1;
    let mut charpos = 1;
    let mut x = 4.0;
    let mut col = 1;

    let walk_outcome = action.apply_to_buffer_walk_state(
        b"abc",
        &mut byte_idx,
        &mut charpos,
        &mut x,
        &mut col,
        &mut cursor_info,
        &active_face,
        &geometry,
        2,
        &mut raise_span,
        &mut height_span,
        &mut face_scan,
    );

    assert_eq!(walk_outcome, BufferDisplayPropertyTextWalkOutcome::Continue);
    assert!(!walk_outcome.should_continue_buffer_walk());
    assert!(!walk_outcome.should_resolve_face());
    assert_eq!(byte_idx, 1);
    assert_eq!(charpos, 1);
    assert_eq!(x, 4.0);
    assert_eq!(col, 1);
    assert!(cursor_info.captured().is_none());
    assert_eq!(raise_span.value(), None);
    assert_eq!(height_span.value(), None);
    assert!(!face_scan.should_resolve_at(0));
}

#[test]
fn buffer_display_property_text_modifier_action_clears_expired_spans() {
    let mut raise_span = ActiveDisplayPropertySpan::inactive();
    raise_span.set(-3.0, 7);
    let mut height_span = ActiveDisplayPropertySpan::inactive();
    height_span.set(1.25, 9);
    let mut face_scan = FaceScanCheckpoint::initial();
    *face_scan.next_check_mut() = 99;

    BufferDisplayPropertyTextModifierAction::clear_expired_raise_span(&mut raise_span, 7, 1);
    let outcome = BufferDisplayPropertyTextModifierAction::clear_expired_height_span(
        &mut height_span,
        &mut face_scan,
        9,
        1,
    );

    assert!(outcome.height_face_changed());
    assert_eq!(raise_span.value(), None);
    assert_eq!(height_span.value(), None);
    assert!(face_scan.should_resolve_at(0));
}

#[test]
fn display_replacement_stretch_append_item_names_cursor_and_extent_policy() {
    let item = DisplayReplacementStretchAppendItem::from_space_extents(13.0, 16.0, 12.0, 8.0);
    assert_eq!(item.width_px(), 13.0);
    assert_eq!(item.height_px(), 16.0);
    assert_eq!(item.ascent_px(), 12.0);
    assert_eq!(item.cursor_slot_width_px(), 13.0);

    let narrow = DisplayReplacementStretchAppendItem::from_space_extents(3.0, 10.0, 7.0, 8.0);
    assert_eq!(narrow.width_px(), 3.0);
    assert_eq!(narrow.cursor_slot_width_px(), 8.0);

    let clamped = DisplayReplacementStretchAppendItem::from_extents(-1.0, -2.0, -3.0);
    assert_eq!(clamped.width_px(), 0.0);
    assert_eq!(clamped.height_px(), 0.0);
    assert_eq!(clamped.ascent_px(), 0.0);
    assert_eq!(clamped.cursor_slot_width_px(), 0.0);
}

fn test_display_space_window_params() -> WindowParams {
    WindowParams {
        window_id: 1,
        buffer_id: 1,
        bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
        text_bounds: Rect::new(0.0, 0.0, 800.0, 560.0),
        selected: true,
        is_minibuffer: false,
        window_start: 1,
        window_end: 0,
        point: 1,
        buffer_size: 1,
        buffer_begv: 1,
        hscroll: 0,
        vscroll: 0,
        truncate_lines: false,
        word_wrap: false,
        tab_width: 8,
        tab_stop_list: vec![],
        default_fg: 0xFFFFFF,
        default_bg: 0x000000,
        char_width: 8.0,
        char_height: 16.0,
        window_system: true,
        font_pixel_size: 14.0,
        font_ascent: 12.0,
        mode_line_height: 0.0,
        header_line_height: 0.0,
        tab_line_height: 0.0,
        cursor_kind: neomacs_display_protocol::frame_glyphs::CursorKind::FilledBox,
        cursor_bar_width: neomacs_display_protocol::frame_glyphs::CursorBarWidth::TWO,
        x_stretch_cursor: false,
        cursor_color: 0xFFFFFF,
        cursor_effects: None,
        visual_cursors: Vec::new(),
        left_fringe_width: 0.0,
        right_fringe_width: 0.0,
        indicate_empty_lines: 0,
        show_trailing_whitespace: false,
        trailing_ws_bg: 0,
        fill_column_indicator: -1,
        fill_column_indicator_char: '|',
        fill_column_indicator_fg: 0,
        extra_line_spacing: 0.0,
        selective_display: 0,
        escape_glyph_fg: 0,
        nobreak_char_display: 0,
        nobreak_char_fg: 0,
        glyphless_char_fg: 0,
        wrap_prefix: vec![],
        line_prefix: vec![],
        left_margin_width: 0.0,
        right_margin_width: 0.0,
        vertical_scroll_bar_side: None,
        horizontal_scroll_bar: false,
        scroll_bar_pixel_width: 0.0,
        scroll_bar_pixel_height: 0.0,
    }
}

#[test]
fn display_replacement_space_width_policy_names_width_sources() {
    let _eval = Context::new();
    let explicit = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("width"),
        Value::fixnum(4),
    ]);
    let relative = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("relative-width"),
        Value::fixnum(2),
    ]);
    let align_to = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("align-to"),
        Value::fixnum(12),
    ]);
    let default = Value::list(vec![Value::symbol("space")]);

    assert!(matches!(
        DisplayReplacementSpaceWidthPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&explicit).expect("explicit list")
        ),
        DisplayReplacementSpaceWidthPolicy::Explicit(_)
    ));
    assert!(matches!(
        DisplayReplacementSpaceWidthPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&relative).expect("relative list")
        ),
        DisplayReplacementSpaceWidthPolicy::Relative { factor } if factor == 2.0
    ));
    assert!(matches!(
        DisplayReplacementSpaceWidthPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&align_to).expect("align list")
        ),
        DisplayReplacementSpaceWidthPolicy::AlignTo(_)
    ));
    assert!(matches!(
        DisplayReplacementSpaceWidthPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&default).expect("default list")
        ),
        DisplayReplacementSpaceWidthPolicy::Default
    ));
}

#[test]
fn display_replacement_space_height_policy_names_height_sources() {
    let _eval = Context::new();
    let explicit = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("height"),
        Value::fixnum(4),
    ]);
    let relative = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("relative-height"),
        Value::fixnum(2),
    ]);
    let default = Value::list(vec![Value::symbol("space")]);

    assert!(matches!(
        DisplayReplacementSpaceHeightPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&explicit).expect("explicit list")
        ),
        DisplayReplacementSpaceHeightPolicy::Explicit(_)
    ));
    assert!(matches!(
        DisplayReplacementSpaceHeightPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&relative).expect("relative list")
        ),
        DisplayReplacementSpaceHeightPolicy::Relative { factor } if factor == 2.0
    ));
    assert!(matches!(
        DisplayReplacementSpaceHeightPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&default).expect("default list")
        ),
        DisplayReplacementSpaceHeightPolicy::Default
    ));
}

#[test]
fn display_replacement_space_ascent_policy_names_ascent_sources() {
    let _eval = Context::new();
    let percent = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("ascent"),
        Value::fixnum(40),
    ]);
    let pixel = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("ascent"),
        Value::fixnum(140),
    ]);
    let default = Value::list(vec![Value::symbol("space")]);

    assert!(matches!(
        DisplayReplacementSpaceAscentPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&percent).expect("percent list")
        ),
        DisplayReplacementSpaceAscentPolicy::Percent { percent } if percent == 40.0
    ));
    assert!(matches!(
        DisplayReplacementSpaceAscentPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&pixel).expect("pixel list")
        ),
        DisplayReplacementSpaceAscentPolicy::Pixel(_)
    ));
    assert!(matches!(
        DisplayReplacementSpaceAscentPolicy::from_items(
            &neovm_core::emacs_core::value::list_to_vec(&default).expect("default list")
        ),
        DisplayReplacementSpaceAscentPolicy::Default
    ));
}

#[test]
fn display_replacement_stretch_append_item_resolves_display_space_property() {
    let _eval = Context::new();
    let active_face = test_active_face_state(7, 8.0);
    let mut font_metrics = None;
    let spec = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("relative-width"),
        Value::fixnum(2),
        Value::keyword("height"),
        Value::list(vec![Value::fixnum(10)]),
        Value::keyword("ascent"),
        Value::fixnum(40),
    ]);

    let item = DisplayReplacementStretchAppendItem::from_display_space_property(
        &spec,
        "x".as_bytes(),
        &active_face,
        &mut font_metrics,
        0.0,
        0.0,
        8.0,
        18.0,
        13.0,
        &test_display_space_window_params(),
    );

    assert_eq!(item.width_px(), 16.0);
    assert_eq!(item.height_px(), 10.0);
    assert_eq!(item.ascent_px(), 4.0);
    assert_eq!(item.cursor_slot_width_px(), 16.0);
}

#[test]
fn display_replacement_media_append_item_names_display_and_cursor_extents() {
    let active_face = test_active_face_state(7, 8.0);
    let media = DisplayMediaReplacement::image(DisplayImageItem {
        image_id: 42,
        width: 64.0,
        height: 10.0,
    });

    let ordinary = DisplayReplacementMediaAppendItem::new(media, &active_face, false);
    assert_eq!(ordinary.width_px(), 64.0);
    assert_eq!(ordinary.display_height_px(), 10.0);
    assert_eq!(ordinary.display_ascent_px(), 10.0);
    assert_eq!(ordinary.cursor_face_height_px(), 10.0);
    assert_eq!(ordinary.cursor_face_ascent_px(), 10.0);

    let xwidget_cursor = DisplayReplacementMediaAppendItem::new(media, &active_face, true);
    assert_eq!(xwidget_cursor.cursor_face_height_px(), 18.0);
    assert_eq!(xwidget_cursor.cursor_face_ascent_px(), 13.0);
}

#[test]
fn display_replacement_media_append_item_resolves_direct_media_property() {
    let active_face = test_active_face_state(7, 8.0);
    let media = DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
        xwidget_id: 17,
        width: 42.0,
        height: 11.0,
    });
    let replacement = DisplayMediaReplacementProperty::Xwidget(media);

    let resolved = DisplayReplacementMediaAppendItem::resolve_display_property(
        Value::NIL,
        &replacement,
        None,
        &active_face,
        8.0,
        16.0,
    )
    .expect("direct media replacement");

    match resolved {
        DisplayReplacementMediaAppendResolution::Media(item) => {
            assert_eq!(item.width_px(), 42.0);
            assert_eq!(item.display_height_px(), 11.0);
            assert_eq!(item.cursor_face_height_px(), 18.0);
        }
        DisplayReplacementMediaAppendResolution::Placeholder(_) => {
            panic!("expected direct media item")
        }
    }
}

#[test]
fn display_replacement_media_append_item_resolves_placeholder_item_without_host() {
    let active_face = test_active_face_state(7, 8.0);

    let resolved = DisplayReplacementMediaAppendItem::resolve_display_property(
        Value::NIL,
        &DisplayMediaReplacementProperty::Image,
        None,
        &active_face,
        8.0,
        16.0,
    )
    .expect("image placeholder");

    match resolved {
        DisplayReplacementMediaAppendResolution::Placeholder(item) => {
            assert_eq!(
                item,
                DisplayReplacementSourceMappedTextAppendItem::new("[img]")
            );
        }
        DisplayReplacementMediaAppendResolution::Media(_) => panic!("expected placeholder item"),
    }
}

#[test]
fn display_replacement_media_append_item_names_row_extent_policy() {
    let active_face = test_active_face_state(7, 8.0);
    let item = DisplayReplacementMediaAppendItem::new(
        DisplayMediaReplacement::image(DisplayImageItem {
            image_id: 42,
            width: 64.0,
            height: 10.0,
        }),
        &active_face,
        false,
    );
    let mut progress = DisplayRowAppendProgress::from_positions(
        DisplayRowPosition { x_px: 0.0, col: 0 },
        DisplayRowPosition { x_px: 64.0, col: 8 },
        DisplayRowAppendStatus::Complete,
        Vec::new(),
    );

    assert_eq!(item.row_extents_after_append(&progress), Some((10.0, 10.0)));

    progress.status = DisplayRowAppendStatus::Clipped;
    assert_eq!(item.row_extents_after_append(&progress), None);

    progress.status = DisplayRowAppendStatus::Complete;
    progress.metrics.width_px = 0.0;
    assert_eq!(item.row_extents_after_append(&progress), None);
}

#[test]
fn display_replacement_append_context_walks_string_faces_and_measurements() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-replacement-string-source", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);

    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let value = Value::string_with_text_properties(
        "ab",
        vec![StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
            ]),
        }],
    );
    let replacement_source = crate::display_source::BufferDisplayReplacementSource::new(
        buf_id,
        CharPos0::new(0),
        EmacsBytePos::new(0),
    );
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let mut font_metrics = None;
    let mut measurer = SourceMappedTextWidthByFace::new();
    let request = DisplayReplacementStringSourceAppendRequest::new(
        DisplayRowPosition { x_px: 0.0, col: 0 },
        LispStringSourceId::display_replacement(1),
        value,
        replacement_source,
    );

    let append_context =
        DisplayReplacementAppendContext::new(replacement_source, 7, base_face, frame);
    let end = request.render_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &mut font_metrics,
        &face_resolver,
        &mut face_ids,
        &append_context,
        &mut measurer,
    );

    assert_eq!(end, DisplayRowPosition { x_px: 24.0, col: 2 });
    assert_eq!(face_ids.finish(), 21);
    assert_eq!(
        builder.faces().get(&20).map(|face| face.foreground),
        Some(Color::from_pixel(0x00ff0000))
    );
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 2);
            assert_eq!(text[0].face_id, 7);
            assert_eq!(text[0].pixel_width, 11.0);
            assert_eq!(text[1].face_id, 20);
            assert_eq!(text[1].pixel_width, 13.0);
        })
        .expect("current row");
}

#[test]
fn append_raw_display_replacement_item_to_text_row_and_emit_uses_face_fallback() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-replacement-item-face", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::synthetic(9, 0, 1),
        RenderFaceRef::Inherit,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(13.0)),
            height: Some(DisplayLength::Pixels(16.0)),
            ascent: Some(DisplayLength::Pixels(12.0)),
        }),
    );

    let (progress, end) = append_raw_display_replacement_item_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &mut font_metrics,
        item,
        &face_resolver,
        base_face,
        7,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    )
    .expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 0.0, col: 0 });
    assert_eq!(end, DisplayRowPosition { x_px: 13.0, col: 2 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert_eq!(text[0].face_id, 7);
            assert_eq!(text[0].pixel_width, 13.0);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { width_cols: 2 }
            ));
        })
        .expect("current row");
}

#[test]
fn display_replacement_append_context_advances_stretch_output() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-replacement-item", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let replacement_source = crate::display_source::BufferDisplayReplacementSource::new(
        buf_id,
        CharPos0::new(0),
        EmacsBytePos::new(0),
    );
    let active_face = test_active_face_state(3, 8.0);

    let append_context = DisplayReplacementRowAppendContext::new(
        replacement_source,
        &surface,
        &geometry,
        &active_face,
        0.0,
        16.0,
    );
    let request = DisplayReplacementStretchAppendItem::from_extents(13.0, 16.0, 12.0)
        .append_request(DisplayRowPosition { x_px: 0.0, col: 0 })
        .expect("stretch append request");
    let (_progress, end) = append_context
        .append_item_request_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            request,
        )
        .expect("append progress");

    assert_eq!(end, DisplayRowPosition { x_px: 13.0, col: 2 });
    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(window_id))
        .and_then(|window| window.display())
        .expect("window display state");
    assert_eq!(
        display.output_cursor,
        Some(neovm_core::window::WindowCursorPos {
            x: 13,
            y: 0,
            row: 0,
            col: 2,
        })
    );
}

#[test]
fn display_replacement_append_context_advances_source_mapped_text_output() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-replacement-mapped-text", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 16.0, 12.0);
    let replacement_source = crate::display_source::BufferDisplayReplacementSource::new(
        buf_id,
        CharPos0::new(0),
        EmacsBytePos::new(0),
    );
    let active_face = test_active_face_state(3, 8.0);

    let append_context = DisplayReplacementRowAppendContext::new(
        replacement_source,
        &surface,
        &geometry,
        &active_face,
        0.0,
        16.0,
    );
    let request = DisplayReplacementSourceMappedTextAppendItem::new("??")
        .append_request(DisplayRowPosition { x_px: 0.0, col: 0 });
    let (_progress, end) = append_context
        .append_item_request_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            request,
        )
        .expect("append progress");

    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 2);
            assert_eq!(text[0].face_id, 3);
            assert!(matches!(
                text[0].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: '?' }
            ));
            assert!(matches!(
                text[1].glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch: '?' }
            ));
        })
        .expect("current row");
}

#[test]
fn synthetic_text_append_context_uses_source_append_request() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("append-display-item", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let active_face = test_active_face_state(3, 8.0);
    let geometry = DisplayRowGeometryState::new(0, 0.0, 0.0, 18.0, 13.0);

    let append_context =
        SyntheticTextRowAppendContext::new(&surface, &geometry, &active_face, 0.0, 10.0);
    let (_progress, end) = append_context
        .append_request_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            SyntheticTextAppendRequest::text_row_metrics_source(
                DisplayRowPosition { x_px: 0.0, col: 0 },
                SyntheticTextSource::new(9, "x"),
                7,
                base_face,
                16.0,
                12.0,
                8.0,
            ),
        )
        .expect("append progress");

    assert_eq!(end, DisplayRowPosition { x_px: 8.0, col: 1 });
    builder
        .with_current_row_mut(|row| {
            let text = &row.glyphs[1];
            assert_eq!(text.len(), 1);
            assert_eq!(text[0].face_id, 7);
        })
        .expect("current row");
}

#[test]
fn display_replacement_append_context_installs_xwidget_replacements() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("append-xwidget-item", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let text_bounds = Rect::new(10.0, 20.0, 160.0, 64.0);
    builder.begin_window_with_text_bounds(
        77,
        1,
        24,
        Rect::new(0.0, 0.0, 200.0, 80.0),
        text_bounds,
        true,
    );
    builder.begin_row(0, GlyphRowRole::Text);
    let surface = DisplayRowAppendSurface::new(
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 160.0,
            text_width: 160.0,
            line_number_width: 0.0,
        },
        DisplayTabPolicy::every(8),
    );
    let geometry = DisplayRowGeometryState::new(0, 4.0, 0.0, 16.0, 12.0);
    let replacement_source = crate::display_source::BufferDisplayReplacementSource::new(
        buf_id,
        CharPos0::new(0),
        EmacsBytePos::new(0),
    );

    let active_face = test_active_face_state(3, 8.0);
    let media_item = DisplayReplacementMediaAppendItem::new(
        DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
            xwidget_id: 1234,
            width: 96.0,
            height: 54.0,
        }),
        &active_face,
        true,
    );
    let append_context = DisplayReplacementRowAppendContext::new(
        replacement_source,
        &surface,
        &geometry,
        &active_face,
        2.0,
        16.0,
    );
    let request = media_item.append_request(DisplayRowPosition { x_px: 16.0, col: 2 });
    let (progress, end) = append_context
        .append_item_request_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            request,
        )
        .expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(progress.metrics.width_px, 96.0);
    assert_eq!(
        end,
        DisplayRowPosition {
            x_px: 112.0,
            col: 14
        }
    );
    builder
        .with_current_row_mut(|row| {
            let glyph = &row.glyphs[1][0];
            assert_eq!(glyph.face_id, 3);
            assert_eq!(glyph.pixel_width, 96.0);
            assert_eq!(glyph.pixel_height, 54.0);
            assert_eq!(glyph.pixel_ascent, 54.0);
            assert!(matches!(
                glyph.glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { width_cols: 12 }
            ));
        })
        .expect("current row");

    builder.end_row();
    builder.end_window();
    let state = builder.finish(24, 1, 8.0, 16.0);
    let xwidget = state.xwidgets.first().expect("xwidget side item");
    assert_eq!(xwidget.window_id, 77);
    assert_eq!(xwidget.row_role, GlyphRowRole::Text);
    assert_eq!(xwidget.clip_rect, Some(text_bounds));
    assert_eq!(
        xwidget.slot_id,
        Some(neomacs_display_protocol::frame_glyphs::DisplaySlotId {
            window_id: 77,
            row: 0,
            col: 2,
        })
    );
    assert_eq!(xwidget.xwidget_id, 1234);
    assert_eq!(xwidget.x, 16.0);
    assert_eq!(xwidget.y, 4.0);
    assert_eq!(xwidget.width, 96.0);
    assert_eq!(xwidget.height, 54.0);
}

#[test]
fn display_replacement_append_context_installs_image_replacements() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("append-image-item", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let text_bounds = Rect::new(10.0, 20.0, 160.0, 64.0);
    builder.begin_window_with_text_bounds(
        77,
        1,
        24,
        Rect::new(0.0, 0.0, 200.0, 80.0),
        text_bounds,
        true,
    );
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = test_append_frame_at(
        0,
        4.0,
        6.0,
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 160.0,
            text_width: 160.0,
            line_number_width: 0.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 12.0,
            char_width: 8.0,
            space_width: 8.0,
            default_row_height: 16.0,
        },
        DisplayTabPolicy::every(8),
    );
    let replacement_source = crate::display_source::BufferDisplayReplacementSource::new(
        buf_id,
        CharPos0::new(0),
        EmacsBytePos::new(0),
    );

    let active_face = test_active_face_state(3, 8.0);
    let media_item = DisplayReplacementMediaAppendItem::new(
        DisplayMediaReplacement::image(DisplayImageItem {
            image_id: 42,
            width: 64.0,
            height: 32.0,
        }),
        &active_face,
        false,
    );
    let append_context =
        DisplayReplacementAppendContext::new(replacement_source, 3, base_face, frame);
    let (progress, end) = append_context
        .append_replacement_item_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            DisplayReplacementAppendItem::media(media_item),
            DisplayRowPosition { x_px: 16.0, col: 2 },
        )
        .expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(progress.metrics.width_px, 64.0);
    assert_eq!(
        end,
        DisplayRowPosition {
            x_px: 80.0,
            col: 10
        }
    );
    builder
        .with_current_row_mut(|row| {
            let glyph = &row.glyphs[1][0];
            assert_eq!(glyph.face_id, 3);
            assert_eq!(glyph.pixel_width, 64.0);
            assert_eq!(glyph.pixel_height, 32.0);
            assert_eq!(glyph.pixel_ascent, 32.0);
            assert!(matches!(
                glyph.glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { width_cols: 8 }
            ));
        })
        .expect("current row");

    builder.end_row();
    builder.end_window();
    let state = builder.finish(24, 1, 8.0, 16.0);
    let image = state.images.first().expect("image side item");
    assert_eq!(image.window_id, 77);
    assert_eq!(image.row_role, GlyphRowRole::Text);
    assert_eq!(image.clip_rect, Some(text_bounds));
    assert_eq!(
        image.slot_id,
        Some(neomacs_display_protocol::frame_glyphs::DisplaySlotId {
            window_id: 77,
            row: 0,
            col: 2,
        })
    );
    assert_eq!(image.image_id, 42);
    assert_eq!(image.x, 16.0);
    assert_eq!(image.y, 4.0);
    assert_eq!(image.width, 64.0);
    assert_eq!(image.height, 32.0);
}

#[test]
fn display_replacement_append_context_installs_video_replacements() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("append-video-item", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter =
        crate::window_output::WindowOutputEmitter::new(frame_id, window_id, 0, 0.0, 0.0);
    output_emitter.begin_update(&mut eval);
    output_emitter.begin_text_row(&mut eval, 0, 0, 0.0, 0.0);
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut font_metrics = None;

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let text_bounds = Rect::new(10.0, 20.0, 160.0, 64.0);
    builder.begin_window_with_text_bounds(
        77,
        1,
        24,
        Rect::new(0.0, 0.0, 200.0, 80.0),
        text_bounds,
        true,
    );
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = test_append_frame_at(
        0,
        4.0,
        6.0,
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 160.0,
            text_width: 160.0,
            line_number_width: 0.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 12.0,
            char_width: 8.0,
            space_width: 8.0,
            default_row_height: 16.0,
        },
        DisplayTabPolicy::every(8),
    );
    let replacement_source = crate::display_source::BufferDisplayReplacementSource::new(
        buf_id,
        CharPos0::new(0),
        EmacsBytePos::new(0),
    );

    let active_face = test_active_face_state(3, 8.0);
    let media_item = DisplayReplacementMediaAppendItem::new(
        DisplayMediaReplacement::video(DisplayVideoItem {
            video_id: 88,
            width: 80.0,
            height: 45.0,
            loop_count: -1,
            autoplay: true,
        }),
        &active_face,
        false,
    );
    let append_context =
        DisplayReplacementAppendContext::new(replacement_source, 3, base_face, frame);
    let (progress, end) = append_context
        .append_replacement_item_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            DisplayReplacementAppendItem::media(media_item),
            DisplayRowPosition { x_px: 16.0, col: 2 },
        )
        .expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(progress.metrics.width_px, 80.0);
    assert_eq!(
        end,
        DisplayRowPosition {
            x_px: 96.0,
            col: 12
        }
    );
    builder
        .with_current_row_mut(|row| {
            let glyph = &row.glyphs[1][0];
            assert_eq!(glyph.face_id, 3);
            assert_eq!(glyph.pixel_width, 80.0);
            assert_eq!(glyph.pixel_height, 45.0);
            assert_eq!(glyph.pixel_ascent, 45.0);
            assert!(matches!(
                glyph.glyph_type,
                neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { width_cols: 10 }
            ));
        })
        .expect("current row");

    builder.end_row();
    builder.end_window();
    let state = builder.finish(24, 1, 8.0, 16.0);
    let video = state.videos.first().expect("video side item");
    assert_eq!(video.window_id, 77);
    assert_eq!(video.row_role, GlyphRowRole::Text);
    assert_eq!(video.clip_rect, Some(text_bounds));
    assert_eq!(
        video.slot_id,
        Some(neomacs_display_protocol::frame_glyphs::DisplaySlotId {
            window_id: 77,
            row: 0,
            col: 2,
        })
    );
    assert_eq!(video.video_id, 88);
    assert_eq!(video.x, 16.0);
    assert_eq!(video.y, 4.0);
    assert_eq!(video.width, 80.0);
    assert_eq!(video.height, 45.0);
    assert_eq!(video.loop_count, -1);
    assert!(video.autoplay);
}
