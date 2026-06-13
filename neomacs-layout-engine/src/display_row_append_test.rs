use super::*;
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::{
    DisplayImageItem, DisplayItemKind, DisplayLength, DisplayMediaReplacement,
    DisplaySourceMappedText, DisplaySourcePosition, DisplayStretch, DisplayStretchWidth,
    DisplayVideoItem, DisplayXwidgetItem, GlyphlessMethod, RenderFaceRef,
};
use crate::display_origin::{DisplayOrigin, DisplayPropertySource};
use crate::display_property::{
    DisplayMediaReplacementProperty, DisplayPropertyClassification, DisplayReplacementProperty,
    classify_display_property,
};
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFallbackMetrics, DisplayRowGeometry,
    DisplayRowMeasuredFaceMetrics, DisplayRowMeasurementPolicy, DisplayRowRenderBounds,
    DisplayRowRenderPolicy, DisplayRowRenderer, DisplayRowSourceRequestPolicy,
    DisplayRowSourceState,
};
use crate::display_row_builder::{
    DisplayRowAppendProgress, DisplayRowAppendStatus, DisplayRowGlyphSlot,
    DisplayRowItemMeasurement, DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::DisplayRowGeometryState;
use crate::display_text_run_measurement::{DisplayTextRunAdvance, DisplayTextRunMeasurement};
use crate::neovm_bridge::{FaceResolver, LayoutBufferSnapshot};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::GlyphType;
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::eval::{
    DisplayHost, GuiFrameHostRequest, ImageResolveRequest, ResolvedImage,
};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;
use std::sync::{Arc, Mutex};

struct RecordingAppendImageHost {
    requests: Arc<Mutex<Vec<ImageResolveRequest>>>,
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
    let mut resolver = BufferTextSourceAdvanceResolver::default();
    let append_context = BufferTextSourceRangeAppendContext::new(
        &snapshot,
        buf_id,
        active_face.face_id(),
        active_face.resolved_face(),
        frame,
    );

    let resolved = append_context.resolve_source_range_advance_to_text_row(
        &mut resolver,
        &mut builder,
        &mut eval,
        &mut font_metrics,
        b"x",
        0,
        BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
        &face_resolver,
        &active_face,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        BufferTextSourceClusterState::for_char('x', None),
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
    let mut resolver = BufferTextSourceAdvanceResolver::default();
    let append_context = BufferTextSourceRangeAppendContext::new(
        &snapshot,
        buf_id,
        active_face.face_id(),
        active_face.resolved_face(),
        frame,
    );

    let resolved = append_context.resolve_source_range_advance_to_text_row(
        &mut resolver,
        &mut builder,
        &mut eval,
        &mut font_metrics,
        b"x",
        0,
        BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
        &face_resolver,
        &active_face,
        DisplayRowPosition {
            x_px: 80.0,
            col: 10,
        },
        BufferTextSourceClusterState::for_char('x', None),
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
    let mut resolver = BufferTextSourceAdvanceResolver::default();
    let append_context = BufferTextSourceRangeAppendContext::new(
        &snapshot,
        buf_id,
        active_face.face_id(),
        active_face.resolved_face(),
        frame,
    );

    let resolved = append_context.resolve_source_range_advance_to_text_row(
        &mut resolver,
        &mut builder,
        &mut eval,
        &mut font_metrics,
        "\u{0633}".as_bytes(),
        0,
        BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
        &face_resolver,
        &active_face,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        BufferTextSourceClusterState::for_char('\u{0633}', None),
    );

    assert_eq!(resolved.advance_px(), 8.0);
    assert_eq!(
        resolved.append_measurement(),
        DisplaySourceAppendMeasurement::ResolvedAdvance { advance_px: 8.0 }
    );
}

#[test]
fn synthetic_display_text_item_builds_synthetic_text_run() {
    let item = synthetic_display_text_item(9, "...", 7);

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
        .append_active_face_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            DisplayRowPosition { x_px: 0.0, col: 0 },
            99,
            "...",
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
    builder.push_char_with_pixel_width('e', 7, 0, 8.0);
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
            100,
            "\u{301}",
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
    builder.push_char_with_pixel_width('e', 7, 0, 8.0);

    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let request = frame.source_append_request(
        DisplayRowPosition { x_px: 8.0, col: 1 },
        7,
        &base_face,
        DisplayRowAppendKind::SourceText,
    );

    let outcome = render_natural_display_source_append_request_into_current_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &mut font_metrics,
        &mut source,
        &mut source_state,
        &face_resolver,
        &mut face_ids,
        request,
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
        renderer
            .render_display_item_source_row_fragment_step_from_request_with_display_host(
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
    builder.push_char_with_pixel_width('X', 7, 0, 8.0);
    builder.push_char_with_pixel_width('Y', 7, 0, 8.0);

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
    let parts = request.into_render_parts();
    assert_eq!(
        parts.request.render_bounds().start,
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(parts.request.render_bounds().max_x_px, 128.0);
    assert_eq!(parts.request.role(), GlyphRowRole::Text);
    assert_eq!(parts.request.base_face_ref(), RenderFaceRef::FaceId(42));
    assert_eq!(
        *parts.request.geometry(),
        DisplayRowGeometry {
            y: 20.0,
            width: 120.0,
            height: 16.0,
            char_width: 9.0,
            ascent: 11.0,
            tab_policy,
        }
    );
    assert_eq!(parts.output.row, 3);
    assert_eq!(parts.output.row_y, 20.0);
    assert_eq!(parts.output.glyph_y, 22.0);
    assert_eq!(parts.output.height, 16.0);
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

    let ordinary = frame
        .source_append_request(position, 42, base_face, DisplayRowAppendKind::SourceText)
        .into_render_parts();
    assert_eq!(
        ordinary.request.render_bounds().start,
        DisplayRowPosition { x_px: 8.0, col: 0 }
    );
    assert_eq!(ordinary.request.render_bounds().max_x_px, 128.0);
    assert_eq!(ordinary.request.geometry().char_width, 9.0);
    assert_eq!(ordinary.output.row, 3);
    assert_eq!(ordinary.output.row_y, 20.0);
    assert_eq!(ordinary.output.glyph_y, 22.0);
    assert_eq!(ordinary.output.height, 16.0);

    let tab = frame
        .source_append_request(position, 42, base_face, DisplayRowAppendKind::Tab)
        .into_render_parts();
    assert_eq!(tab.request.render_bounds().max_x_px, f32::INFINITY);
    assert_eq!(tab.request.geometry().char_width, 7.0);
    assert_eq!(tab.output.height, 14.0);

    let control = frame
        .source_append_request(position, 42, base_face, DisplayRowAppendKind::ControlChar)
        .into_render_parts();
    assert_eq!(control.request.render_bounds().max_x_px, 148.0);
    assert_eq!(control.request.geometry().char_width, 9.0);
    assert_eq!(control.output.height, 14.0);

    let mapped = frame
        .source_append_request(
            position,
            42,
            base_face,
            DisplayRowAppendKind::SourceMappedText,
        )
        .into_render_parts();
    assert_eq!(mapped.request.render_bounds().max_x_px, 128.0);
    assert_eq!(mapped.output.height, 14.0);

    let glyphless = frame
        .source_append_request(position, 42, base_face, DisplayRowAppendKind::Glyphless)
        .into_render_parts();
    assert_eq!(glyphless.request.render_bounds().max_x_px, 128.0);
    assert_eq!(glyphless.output.height, 16.0);

    let replacement = frame
        .source_append_request(
            position,
            42,
            base_face,
            DisplayRowAppendKind::DisplayReplacement,
        )
        .into_render_parts();
    assert_eq!(replacement.request.render_bounds().max_x_px, 128.0);
    assert_eq!(replacement.request.geometry().char_width, 9.0);
    assert_eq!(replacement.output.height, 16.0);

    let replacement_string = frame
        .source_append_request(
            position,
            42,
            base_face,
            DisplayRowAppendKind::DisplayReplacementString,
        )
        .into_render_parts();
    assert_eq!(replacement_string.request.render_bounds().max_x_px, 128.0);
    assert_eq!(replacement_string.request.geometry().char_width, 7.0);
    assert_eq!(replacement_string.output.height, 16.0);
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
    let parts = request.into_render_parts();
    assert_eq!(parts.request.render_bounds().max_x_px, 128.0);
    assert_eq!(parts.request.base_face_ref(), RenderFaceRef::FaceId(42));
    assert_eq!(parts.output.row, 3);
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
    let parts = request.into_render_parts();
    assert_eq!(parts.request.render_bounds().max_x_px, 128.0);
    assert_eq!(parts.request.base_face_ref(), RenderFaceRef::FaceId(42));
    assert_eq!(parts.output.row, 3);
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
    let parts = request.into_render_parts();
    assert_eq!(
        parts.request.render_bounds().start,
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(parts.request.render_bounds().max_x_px, 148.0);
    assert_eq!(parts.request.geometry().height, 16.0);
    assert_eq!(parts.output.height, 14.0);
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
    let parts = request.into_render_parts();
    assert_eq!(
        parts.request.render_bounds().start,
        DisplayRowPosition { x_px: 18.0, col: 2 }
    );
    assert_eq!(parts.request.render_bounds().max_x_px, 148.0);
    assert_eq!(parts.request.base_face_id(), 42);
    assert_eq!(parts.output.row, 3);
    assert_eq!(parts.output.height, 14.0);
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

    let end = append_context.append_active_face_value_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &mut font_metrics,
        Value::string("=>"),
        2,
        &face_resolver,
        &mut face_ids,
        0,
        base_face,
        DisplayRowPosition { x_px: 0.0, col: 0 },
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
    let (_progress, end) = append_context
        .append_resolved_source_char_to_text_row(
            &geometry,
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &source_char,
            &face_resolver,
            ResolvedBufferTextSourceAdvance::natural(8.0),
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("appended buffer fragment");

    assert_eq!(end, DisplayRowPosition { x_px: 8.0, col: 1 });
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
    builder.push_char_with_pixel_width('x', 7, 0, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));
    let position = DisplayRowPosition { x_px: 8.0, col: 1 };
    let source_range = BufferTextSourceRange::new(CharPos0::new(1), CharPos0::new(2));

    let append_context =
        BufferTextSourceRangeAppendContext::new(&snapshot, buf_id, 7, &base_face, frame);
    let measured_width = append_context
        .measure_source_range_natural_advance_to_text_row(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            source_range,
            &face_resolver,
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
        .append_resolved_source_range_to_text_row(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            source_range,
            &face_resolver,
            ResolvedBufferTextSourceAdvance::natural(measured_width),
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
        .append_resolved_source_range_to_text_row(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
            &face_resolver,
            ResolvedBufferTextSourceAdvance::resolved(13.0),
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
    builder.push_char_with_pixel_width('e', 7, 0, 8.0);
    let frame = test_append_frame(8.0, 8.0, DisplayTabPolicy::every(8));

    let append_context =
        BufferTextSourceRangeAppendContext::new(&snapshot, buf_id, 7, &base_face, frame);
    let (progress, end) = append_context
        .append_resolved_source_range_to_text_row(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            BufferTextSourceRange::new(CharPos0::new(1), CharPos0::new(2)),
            &face_resolver,
            ResolvedBufferTextSourceAdvance::natural(0.0),
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

    let append_context = BufferTextItemAppendContext::new(&snapshot, buf_id, 7, base_face, frame);
    let measured_width = append_context
        .measure_source_range_width_to_text_row(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
            &face_resolver,
            item.clone(),
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("measured buffer text item fragment");
    builder
        .with_current_row_mut(|row| assert!(row.glyphs[1].is_empty()))
        .expect("current row");
    let fallback_width = append_context
        .measure_source_range_width_or_active_face_fallback_to_text_row(
            &mut builder,
            &mut eval,
            &mut font_metrics,
            BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(0)),
            &face_resolver,
            item.clone(),
            DisplayRowPosition { x_px: 0.0, col: 0 },
        );
    let edge_width = append_context.measure_source_range_width_or_active_face_fallback_to_text_row(
        &mut builder,
        &mut eval,
        &mut font_metrics,
        BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
        &face_resolver,
        item.clone(),
        DisplayRowPosition {
            x_px: 80.0,
            col: 10,
        },
    );

    let (progress, end) = append_context
        .append_source_range_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            BufferTextSourceRange::new(CharPos0::new(0), CharPos0::new(1)),
            &face_resolver,
            item,
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

    assert_eq!(
        source_char.range(),
        BufferTextSourceRange::new(CharPos0::new(4), CharPos0::new(5))
    );
    assert_eq!(
        source_char.precluster_special_display(),
        Some(&BufferTextSourceSpecialDisplay::Nobreak(
            BufferTextSourceAppendItem::SourceMappedText { text: "\\ ".into() }
        ))
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
    assert_eq!(source_char.cluster_special_display(cluster_tail), None);

    let standalone_joiner = BufferTextSourceChar::new('\u{200D}', CharPos0::new(2), 2);
    assert_eq!(
        standalone_joiner.cluster_special_display(None),
        BufferTextSourceSpecialDisplay::for_cluster_state(BufferTextSourceClusterState::for_char(
            '\u{200D}', None
        ))
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
    let special_display = source_char
        .precluster_special_display()
        .cloned()
        .expect("nobreak source char should map to a display item");
    let (_progress, end) = append_context
        .append_special_source_char_to_text_row_and_emit(
            &geometry,
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &source_char,
            &face_resolver,
            special_display,
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("appended source-mapped buffer text item fragment");

    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
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
    let special_display = source_char
        .cluster_special_display(None)
        .expect("glyphless source char should map to a display item");
    let (_progress, end) = append_context
        .append_special_source_char_to_text_row_and_emit(
            &geometry,
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &source_char,
            &face_resolver,
            special_display,
            DisplayRowPosition { x_px: 0.0, col: 0 },
        )
        .expect("appended glyphless buffer text item fragment");

    assert_eq!(end, DisplayRowPosition { x_px: 48.0, col: 6 });
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
    let mut source = crate::display_source::LispStringSourceCursor::new(
        1,
        Value::string("a\nb"),
        RenderFaceRef::FaceId(7),
    )
    .expect("lisp string source");
    let mut source_state = DisplayRowSourceState::default();
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

    let mut append_context = LispStringSourceRowAppendContext::new(
        &mut source,
        &mut source_state,
        7,
        base_face,
        &surface,
        0.0,
        16.0,
        12.0,
        8.0,
        16.0,
    );

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
    let _eval = Context::new();
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
    assert_eq!(item.value(), value);
    assert_eq!(item.source_id(), 9);
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

    let append_context =
        DisplayReplacementAppendContext::new(replacement_source, 7, base_face, frame);
    let end = append_context.append_string_source_value_to_text_row(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        &mut font_metrics,
        value,
        1,
        &face_resolver,
        &mut face_ids,
        DisplayRowPosition { x_px: 0.0, col: 0 },
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
    let (_progress, end) = append_context
        .append_active_face_stretch_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            DisplayReplacementStretchAppendItem::from_extents(13.0, 16.0, 12.0),
            DisplayRowPosition { x_px: 0.0, col: 0 },
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
    let (_progress, end) = append_context
        .append_active_face_source_mapped_text_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            DisplayReplacementSourceMappedTextAppendItem::new("??"),
            DisplayRowPosition { x_px: 0.0, col: 0 },
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
        .append_text_row_metrics_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            DisplayRowPosition { x_px: 0.0, col: 0 },
            9,
            "x",
            7,
            base_face,
            16.0,
            12.0,
            8.0,
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
    let (progress, end) = append_context
        .append_display_box_media_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            media_item,
            DisplayRowPosition { x_px: 16.0, col: 2 },
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
        .append_media_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            media_item,
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
        .append_media_to_text_row_and_emit(
            &mut builder,
            &mut output_emitter,
            &mut eval,
            &mut font_metrics,
            &face_resolver,
            media_item,
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
