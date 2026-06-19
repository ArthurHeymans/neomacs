use super::*;
use crate::display_row_matrix_install::{
    DisplayRowInstaller, install_mock_display_row_in_matrix_row,
};
use crate::display_row_source_render::current_display_row_cluster_tail;
use crate::neovm_bridge::{FaceResolver, LayoutBufferSnapshot, LayoutBufferView};
use neomacs_display_protocol::Rect;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow, GlyphType};
use neovm_core::buffer::{CharPos0, EmacsByteRange};
use neovm_core::emacs_core::eval::{
    DisplayHost, GuiFrameHostRequest, ImageResolveRequest, ResolvedImage, ResolvedVideo,
    ResolvedWebKit, VideoResolveRequest, WebKitResolveRequest,
};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;
use std::sync::Mutex;

fn base_face() -> crate::neovm_bridge::ResolvedFace {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    resolver.default_face().clone()
}

fn display_row_request_from_base_face<'a>(
    geometry: DisplayRowGeometry,
    face_ids: &mut FrameFaceIdAllocator,
    base_face: &'a crate::neovm_bridge::ResolvedFace,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
) -> DisplayRowSourceRenderRequest<'a> {
    DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role)
        .with_symbol_values(symbol_values)
        .source_request_from_base_face(face_ids, base_face)
}

fn display_row_request_for_face<'a>(
    geometry: DisplayRowGeometry,
    base_face_id: u32,
    base_face: &'a crate::neovm_bridge::ResolvedFace,
    role: GlyphRowRole,
) -> DisplayRowSourceRenderRequest<'a> {
    DisplayRowSourceRequestPolicy::from_display_row_geometry(geometry, role)
        .source_request_for_base_face_id(base_face_id, base_face)
}

#[derive(Default)]
struct RecordingDisplayRowMediaHost {
    image_requests: Mutex<Vec<ImageResolveRequest>>,
    video_requests: Mutex<Vec<VideoResolveRequest>>,
    webkit_requests: Mutex<Vec<WebKitResolveRequest>>,
}

impl DisplayHost for RecordingDisplayRowMediaHost {
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
        panic!("display row rendering must use nonblocking request_image");
    }

    fn request_image(&self, request: ImageResolveRequest) -> Result<Option<ResolvedImage>, String> {
        self.image_requests
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

    fn request_video(&self, request: VideoResolveRequest) -> Result<Option<ResolvedVideo>, String> {
        self.video_requests
            .lock()
            .expect("video requests lock")
            .push(request);
        Ok(Some(ResolvedVideo { video_id: 84 }))
    }

    fn request_webkit(
        &self,
        request: WebKitResolveRequest,
    ) -> Result<Option<ResolvedWebKit>, String> {
        self.webkit_requests
            .lock()
            .expect("webkit requests lock")
            .push(request);
        Ok(Some(ResolvedWebKit { webkit_id: 99 }))
    }
}

#[test]
fn display_row_face_realizer_realizes_face_without_layout_engine() {
    let mut font_metrics = None;
    let mut realizer = DisplayRowFaceRealizer::new(&mut font_metrics);
    let mut face = base_face();
    face.font_char_width = 0.0;
    face.font_ascent = 0.0;
    face.font_line_height = 0.0;

    let rendered = realizer.realize_face(7, &face, 8.0, 12.0, 16.0);

    assert_eq!(rendered.face_id, 7);
    assert_eq!(rendered.font_char_width, 8.0);
    assert_eq!(rendered.font_ascent, 12.0);
    assert_eq!(rendered.font_descent, 4);
}

#[test]
fn display_row_render_item_lowers_media_replacement_to_row_stretch() {
    let media = DisplayMediaReplacement::xwidget(crate::display_item::DisplayXwidgetItem {
        xwidget_id: 17,
        width: 42.0,
        height: 11.0,
    });
    let source = DisplayItem::new(
        SourceSpan::synthetic(1, 0, 1),
        RenderFaceRef::FaceId(7),
        DisplayItemKind::MediaReplacement(media),
    );
    let render_item = DisplayRowRenderItem::from_source_item(source.clone());

    assert_eq!(render_item.source_item(), &source);
    assert_eq!(render_item.row_face(), RenderFaceRef::FaceId(7));
    let DisplayItemKind::Stretch(stretch) = &render_item.row_item().kind else {
        panic!("media replacement should lower to a row stretch item");
    };
    assert_eq!(
        stretch.width,
        crate::display_item::DisplayStretchWidth::Length(
            crate::display_item::DisplayLength::Pixels(42.0)
        )
    );

    let rendered_media = render_item
        .rendered_media_for_progress(
            &DisplayRowAppendProgress::from_positions(
                DisplayRowPosition { x_px: 8.0, col: 1 },
                DisplayRowPosition { x_px: 50.0, col: 2 },
                DisplayRowAppendStatus::Complete,
                Vec::new(),
            ),
            6.0,
        )
        .expect("media should render after complete nonempty append");

    assert_eq!(
        rendered_media.kind,
        RenderedDisplayRowMediaKind::Xwidget { xwidget_id: 17 }
    );
    assert_eq!(rendered_media.x, 8.0);
    assert_eq!(rendered_media.y, 6.0);
    assert_eq!(rendered_media.width, 42.0);
    assert_eq!(rendered_media.height, 11.0);
}

#[test]
fn current_display_row_cluster_tail_reports_live_text_row_tail() {
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 5, Rect::new(0.0, 0.0, 40.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);

    assert_eq!(current_display_row_cluster_tail(&builder), None);

    builder
        .edit_current_row_for_test(|row| {
            crate::glyph_row_writer::push_wide_char_to_row(row, '\u{1F1EF}', 3, 100, 0.0);
        })
        .expect("current row");
    assert_eq!(
        current_display_row_cluster_tail(&builder),
        Some(('\u{1F1EF}', true))
    );

    builder
        .edit_current_row_for_test(|row| {
            crate::glyph_row_writer::push_cluster_continuation_to_row(row, '\u{1F1F5}', 3, 101);
        })
        .expect("current row");
    assert_eq!(
        current_display_row_cluster_tail(&builder),
        Some(('\u{1F1F5}', false))
    );
}

#[test]
fn insert_resolved_display_row_face_applies_metric_overrides() {
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let face = base_face();

    builder.artifact_installer().set_resolved_display_row_face(
        9,
        &face,
        Some(FontMetrics {
            ascent: 10.0,
            descent: 3.0,
            line_height: 13.0,
            char_width: 7.0,
        }),
    );

    let rendered = builder.faces().get(&9).expect("inserted face");
    assert_eq!(rendered.id, 9);
    assert_eq!(rendered.font_ascent, 10);
    assert_eq!(rendered.font_descent, 3);
}

#[test]
fn display_row_source_geometry_allocates_dynamic_base_face_id_through_allocator() {
    let mut face = base_face();
    face.face_id = 0;
    let mut face_ids = FrameFaceIdAllocator::new(42);

    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 80.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &face,
        GlyphRowRole::Text,
        std::collections::HashMap::new(),
    );

    assert_eq!(request.base_face_id(), 42);
    assert_eq!(face_ids.finish(), 43);
}

#[test]
fn display_row_source_geometry_builds_whole_row_request() {
    let face = base_face();
    let geometry = DisplayRowGeometry {
        y: 4.0,
        width: 96.0,
        height: 18.0,
        char_width: 9.0,
        ascent: 13.0,
        tab_policy: DisplayTabPolicy::every(4),
    };

    let request =
        display_row_request_for_face(geometry.clone(), 17, &face, GlyphRowRole::Minibuffer);

    assert_eq!(request.geometry(), &geometry);
    assert_eq!(
        request.render_bounds(),
        DisplayRowRenderBounds::whole_row(96.0)
    );
    assert_eq!(request.base_face_id(), 17);
    assert!(std::ptr::eq(request.base_face(), &face));
    assert_eq!(request.role(), GlyphRowRole::Minibuffer);
    assert!(request.symbol_values().is_empty());
}

#[test]
fn display_row_source_geometry_allocates_base_face_id() {
    let mut face = base_face();
    face.face_id = 0;
    let mut face_ids = FrameFaceIdAllocator::new(24);
    let geometry = DisplayRowGeometry {
        y: 5.0,
        width: 120.0,
        height: 20.0,
        char_width: 10.0,
        ascent: 14.0,
        tab_policy: DisplayTabPolicy::every(8),
    };
    let mut symbol_values = std::collections::HashMap::new();
    symbol_values.insert("header-line-indent-width".to_string(), Value::fixnum(3));

    let request = display_row_request_from_base_face(
        geometry.clone(),
        &mut face_ids,
        &face,
        GlyphRowRole::HeaderLine,
        symbol_values.clone(),
    );

    assert_eq!(request.geometry(), &geometry);
    assert_eq!(
        request.render_bounds(),
        DisplayRowRenderBounds::whole_row(120.0)
    );
    assert_eq!(request.base_face_id(), 24);
    assert_eq!(face_ids.finish(), 25);
    assert_eq!(request.role(), GlyphRowRole::HeaderLine);
    assert_eq!(request.symbol_values(), &symbol_values);
}

#[test]
fn display_row_source_request_policy_builds_chrome_request() {
    let mut face = base_face();
    face.face_id = 0;
    let mut face_ids = FrameFaceIdAllocator::new(31);
    let mut symbol_values = std::collections::HashMap::new();
    symbol_values.insert("tab-bar-tab-hscroll".to_string(), Value::fixnum(2));

    let request = DisplayRowSourceRequestPolicy::from_origin(
        6.0,
        144.0,
        22.0,
        11.0,
        16.0,
        DisplayTabPolicy::every(8),
        crate::display_origin::DisplayOrigin::TabBar,
    )
    .with_symbol_values(symbol_values.clone())
    .source_request_from_base_face(&mut face_ids, &face);

    assert_eq!(
        request.geometry(),
        &DisplayRowGeometry {
            y: 6.0,
            width: 144.0,
            height: 22.0,
            char_width: 11.0,
            ascent: 16.0,
            tab_policy: DisplayTabPolicy::every(8),
        }
    );
    assert_eq!(
        request.render_bounds(),
        DisplayRowRenderBounds::whole_row(144.0)
    );
    assert_eq!(request.base_face_id(), 31);
    assert_eq!(face_ids.finish(), 32);
    assert_eq!(request.role(), GlyphRowRole::TabBar);
    assert_eq!(request.symbol_values(), &symbol_values);
}

#[test]
fn display_row_source_geometry_request_overrides_render_bounds() {
    let face = base_face();
    let geometry = DisplayRowGeometry {
        y: 0.0,
        width: 80.0,
        height: 16.0,
        char_width: 8.0,
        ascent: 12.0,
        tab_policy: DisplayTabPolicy::every(8),
    };
    let bounds = DisplayRowRenderBounds {
        start: DisplayRowPosition { x_px: 16.0, col: 2 },
        max_x: DisplayRowMaxX::Bounded(40.0),
    };

    let request = display_row_request_for_face(geometry, 7, &face, GlyphRowRole::Text)
        .with_render_bounds(bounds);

    assert_eq!(request.render_bounds(), bounds);
    assert_eq!(request.base_face_id(), 7);
    assert_eq!(request.role(), GlyphRowRole::Text);
}

#[test]
fn display_row_source_fragment_frame_builds_column_bounds_from_glyph_row() {
    let face = base_face();
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    row.pixel_y = 6.0;
    row.height_px = 18.0;
    row.ascent_px = 13.0;

    let request = DisplayRowSourceFragmentFrame::from_glyph_row_columns(
        &row,
        12,
        7.5,
        GlyphRowRole::Text,
        9,
        &face,
    )
    .render_request_from_column_for_area(3, 12, GlyphArea::RightMargin);

    assert_eq!(
        request.geometry(),
        &DisplayRowGeometry {
            y: 6.0,
            width: 90.0,
            height: 18.0,
            char_width: 7.5,
            ascent: 13.0,
            tab_policy: DisplayTabPolicy::every(8),
        }
    );
    assert_eq!(
        request.render_bounds(),
        DisplayRowRenderBounds {
            start: DisplayRowPosition { x_px: 22.5, col: 3 },
            max_x: DisplayRowMaxX::Bounded(90.0),
        }
    );
    assert_eq!(request.glyph_area(), GlyphArea::RightMargin);
}

#[test]
fn display_row_source_fragment_frame_builds_column_bounds_from_row_geometry() {
    let face = base_face();
    let row_geometry = DisplayRowGeometryState::new(4, 11.0, 24.0, 20.0, 15.0);

    let request = DisplayRowSourceFragmentFrame::from_row_geometry_columns(
        &row_geometry,
        5,
        9.0,
        GlyphRowRole::Text,
        17,
        &face,
    )
    .render_request_from_column_for_area(0, 5, GlyphArea::LeftMargin);

    assert_eq!(
        request.geometry(),
        &DisplayRowGeometry {
            y: 11.0,
            width: 45.0,
            height: 20.0,
            char_width: 9.0,
            ascent: 15.0,
            tab_policy: DisplayTabPolicy::every(8),
        }
    );
    assert_eq!(
        request.render_bounds(),
        DisplayRowRenderBounds {
            start: DisplayRowPosition { x_px: 0.0, col: 0 },
            max_x: DisplayRowMaxX::Bounded(45.0),
        }
    );
    assert_eq!(request.glyph_area(), GlyphArea::LeftMargin);
}

#[test]
fn display_row_render_context_builds_source_resolve_params() {
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let context = DisplayRowRenderContext::new(&face_resolver, None, &mut face_ids);
    let fallback =
        crate::display_source_resolver::DisplaySourceFallbackMetrics::new(8.0, 12.0, 16.0);

    let params = context.source_resolve_params(7, base_face, fallback);

    assert_eq!(params.face_basis().base_face_id(), 7);
    assert_eq!(params.face_basis().fallback_metrics(), fallback);
    assert!(std::ptr::eq(params.face_basis().base_face(), base_face));
    assert!(std::ptr::eq(
        params.face_basis().canonical_face(),
        base_face
    ));
}

#[test]
fn display_row_resolved_measured_face_installs_render_and_measurement_identity() {
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let mut font_metrics = None;
    let policy = DisplayRowMeasurementPolicy::for_frame(true);
    let face = base_face();

    let realized = policy.resolved_measured_face(
        12,
        face,
        Some(FontMetrics {
            ascent: 11.0,
            descent: 4.0,
            line_height: 15.0,
            char_width: 7.5,
        }),
        7.0,
        DisplayRowFallbackMetrics {
            char_width: 7.0,
            row_height: 14.0,
            ascent: 10.0,
        },
        &mut font_metrics,
    );

    builder.artifact_installer().set_resolved_display_row_face(
        realized.face_id(),
        realized.resolved_face(),
        realized.font_metrics(),
    );

    let rendered = builder.faces().get(&12).expect("installed face");
    assert_eq!(realized.face_id(), 12);
    assert_eq!(rendered.id, 12);
    assert_eq!(rendered.font_ascent, 11);
    assert_eq!(rendered.font_descent, 4);
}

#[test]
fn display_row_resolved_measured_face_builds_active_face_state_directly() {
    let mut font_metrics = None;
    let policy = DisplayRowMeasurementPolicy::for_frame(true);
    let face = base_face();

    let active = policy
        .resolved_measured_face(
            12,
            face.clone(),
            Some(FontMetrics {
                ascent: 11.0,
                descent: 4.0,
                line_height: 15.0,
                char_width: 7.5,
            }),
            7.0,
            DisplayRowFallbackMetrics {
                char_width: 7.0,
                row_height: 14.0,
                ascent: 10.0,
            },
            &mut font_metrics,
        )
        .into_active_face_state();

    assert_eq!(active.face_id(), 12);
    assert_eq!(active.resolved_face().fg, face.fg);
    assert_eq!(active.metrics().row_height, 15.0);
}

#[test]
fn display_row_active_face_groups_resolved_measurement_metrics_and_colors() {
    let mut font_metrics = None;
    let policy = DisplayRowMeasurementPolicy::for_frame(false);
    let mut face = base_face();
    face.fg = 0x00112233;
    face.bg = 0x00445566;

    let active = policy
        .resolved_measured_face(
            14,
            face.clone(),
            None,
            7.0,
            DisplayRowFallbackMetrics {
                char_width: 7.0,
                row_height: 15.0,
                ascent: 10.0,
            },
            &mut font_metrics,
        )
        .into_active_face_state();

    assert_eq!(active.face_id(), 14);
    assert_eq!(active.metrics().char_width, 7.0);
    assert_eq!(active.metrics().row_height, 15.0);
    assert_eq!(active.metrics().ascent, 10.0);
    assert_eq!(active.metrics().space_width, 7.0);
    assert_eq!(active.background(), Color::from_pixel(face.bg));
    assert_eq!(active.resolved_face().fg, face.fg);
}

#[test]
fn display_row_active_face_state_exposes_render_and_measurement_accessors() {
    let mut font_metrics = None;
    let policy = DisplayRowMeasurementPolicy::for_frame(false);
    let mut face = base_face();
    face.fg = 0x00112233;
    face.bg = 0x00445566;

    let active = policy
        .resolved_measured_face(
            14,
            face.clone(),
            None,
            7.0,
            DisplayRowFallbackMetrics {
                char_width: 7.0,
                row_height: 15.0,
                ascent: 10.0,
            },
            &mut font_metrics,
        )
        .into_active_face_state();

    assert_eq!(active.face_id(), 14);
    assert_eq!(active.background(), Color::from_pixel(face.bg));
    assert_eq!(active.resolved_face().fg, face.fg);
    assert_eq!(active.metrics().char_width, 7.0);
}

#[test]
fn display_row_active_face_state_constructs_from_resolved_and_measured_face() {
    let mut font_metrics = None;
    let policy = DisplayRowMeasurementPolicy::for_frame(false);
    let mut face = base_face();
    face.fg = 0x00112233;
    face.bg = 0x00445566;
    let measured = policy.measured_face(
        14,
        &face,
        None,
        7.0,
        DisplayRowFallbackMetrics {
            char_width: 7.0,
            row_height: 15.0,
            ascent: 10.0,
        },
        &mut font_metrics,
    );

    let active = DisplayRowActiveFaceState::new(face.clone(), measured);

    assert_eq!(active.face_id(), 14);
    assert_eq!(active.background(), Color::from_pixel(face.bg));
    assert_eq!(active.resolved_face().fg, face.fg);
    assert_eq!(active.metrics().char_width, 7.0);
}

#[test]
fn display_row_renderer_renders_lisp_string_without_layout_engine() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let rendered = DisplayRowLispStringRenderRequest::new(request, Value::string("A中"))
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row");

    assert_eq!(row_text_expanding_stretches(&rendered.row), "A中");
    assert_eq!(rendered.row.role, GlyphRowRole::TabLine);
    assert_eq!(rendered.progress.end_col, 3);
}

/// The HELLO file separates a script name from its greeting with a literal
/// TAB (tab-width 42). On a TTY the rendered column where the TAB stops must
/// match the buffer-level `current-column` model: GNU advances `current_x` by
/// the composed cluster's `char-width` sum, so combining marks contribute 0
/// columns and the TAB after `Arabic (العربيّة)` (string-width 16; the shadda
/// U+0651 is a zero-width combining mark) fills to the tab stop at column 42.
///
/// Regression for the composed/complex-run cell over-count: the TTY render
/// walk gave every complex-run member its own column (including the zero-width
/// shadda absorbed into the Arabic shaping run), so the running column ran
/// past the buffer model and the TAB over-filled, pushing the greeting right.
#[test]
fn tty_complex_run_then_tab_lands_on_buffer_tab_stop() {
    let _eval = Context::new();
    // font_metrics = None mirrors the TTY frame's fallback measurement path.
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut test_base_face = resolver.default_face().clone();
    test_base_face.font_char_width = 8.0;
    test_base_face.font_ascent = 12.0;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            // 160 cols * 8px so nothing clips.
            width: 1280.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(42),
        },
        &mut face_ids,
        &test_base_face,
        GlyphRowRole::Text,
        std::collections::HashMap::new(),
    );

    let rendered =
        DisplayRowLispStringRenderRequest::new(request, Value::string("Arabic (العربيّة)\tx"))
            .render(&mut renderer, &resolver, &mut face_ids)
            .expect("display source row");

    // The greeting (here `x`) must land at the tab stop, just past column 42.
    assert_eq!(
        rendered.progress.end_col, 43,
        "complex name + TAB must reach the buffer tab stop (col 42 + 1 for `x`); got {}",
        rendered.progress.end_col
    );
}

#[test]
fn display_row_source_state_reuses_face_cache_across_items() {
    let _eval = Context::new();
    let table = FaceTable::new();
    let face_resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let base_face = face_resolver.default_face();
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
    let mut source =
        crate::display_source::LispStringSourceCursor::new(1, value, RenderFaceRef::FaceId(0))
            .expect("string source");
    let mut state = DisplayRowSourceState::default();
    let mut face_ids = FrameFaceIdAllocator::new(20);
    let (first, second, third) = {
        let mut next_item = || {
            state.next_resolved_item(
                &mut source,
                crate::display_source_resolver::DisplaySourceResolveParams::new(
                    crate::display_source_resolver::DisplaySourceFaceBasis::new(
                        &face_resolver,
                        0,
                        base_face,
                        crate::display_source_resolver::DisplaySourceFallbackMetrics::new(
                            8.0, 12.0, 16.0,
                        ),
                    ),
                    None,
                ),
                &mut face_ids,
            )
        };
        (next_item(), next_item(), next_item())
    };

    assert_eq!(
        first.item.expect("first source item").face,
        RenderFaceRef::FaceId(20)
    );
    assert_eq!(first.pending_faces.len(), 1);
    assert_eq!(
        second.item.expect("second source item").face,
        RenderFaceRef::FaceId(0)
    );
    assert!(second.pending_faces.is_empty());
    assert_eq!(
        third.item.expect("third source item").face,
        RenderFaceRef::FaceId(20)
    );
    assert!(third.pending_faces.is_empty());
    assert_eq!(face_ids.finish(), 21);
}

#[test]
fn display_row_renderer_clips_lisp_string_rows_to_geometry_width() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut test_base_face = resolver.default_face().clone();
    test_base_face.font_char_width = 8.0;
    test_base_face.font_ascent = 12.0;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let spec = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 16.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &test_base_face,
        GlyphRowRole::ModeLine,
        std::collections::HashMap::new(),
    );

    let rendered = DisplayRowLispStringRenderRequest::new(spec, Value::string("ABC"))
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row");

    assert_eq!(row_text_expanding_stretches(&rendered.row), "AB");
    assert_eq!(rendered.progress.end_x, 16.0);
    assert_eq!(rendered.progress.end_col, 2);
}

#[test]
fn display_row_renderer_clips_from_render_bounds_start() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut test_base_face = resolver.default_face().clone();
    test_base_face.font_char_width = 8.0;
    test_base_face.font_ascent = 12.0;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &test_base_face,
        GlyphRowRole::ModeLine,
        std::collections::HashMap::new(),
    )
    .with_render_bounds(DisplayRowRenderBounds {
        start: DisplayRowPosition { x_px: 16.0, col: 2 },
        max_x: DisplayRowMaxX::Bounded(32.0),
    });

    let rendered = DisplayRowLispStringRenderRequest::new(request, Value::string("ABC"))
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row");

    assert_eq!(row_text_expanding_stretches(&rendered.row), "AB");
    assert_eq!(rendered.progress.end_x, 32.0);
    assert_eq!(rendered.progress.end_col, 4);
    assert_eq!(rendered.source_slots[0].x_px, 16.0);
    assert_eq!(rendered.source_slots[0].col, 2);
}

#[test]
fn display_row_renderer_uses_render_bounds_start_for_tab_advance() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut test_base_face = resolver.default_face().clone();
    test_base_face.font_char_width = 8.0;
    test_base_face.font_ascent = 12.0;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(4),
        },
        &mut face_ids,
        &test_base_face,
        GlyphRowRole::ModeLine,
        std::collections::HashMap::new(),
    )
    .with_render_bounds(DisplayRowRenderBounds {
        start: DisplayRowPosition { x_px: 16.0, col: 2 },
        max_x: DisplayRowMaxX::Bounded(240.0),
    });

    let rendered = DisplayRowLispStringRenderRequest::new(request, Value::string("\tX"))
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row");

    let glyphs = &rendered.row.glyphs[1];
    assert_eq!(glyphs[0].glyph_type, GlyphType::Stretch { width_cols: 2 });
    assert_eq!(glyphs[0].pixel_width, 16.0);
    assert_eq!(rendered.progress.end_x, 40.0);
    assert_eq!(rendered.progress.end_col, 5);
    assert_eq!(rendered.source_slots[0].x_px, 16.0);
    assert_eq!(rendered.source_slots[0].width_px, 16.0);
}

#[test]
fn display_row_renderer_continues_source_mapped_text_after_clip() {
    struct OnceSource {
        item: Option<crate::display_item::DisplayItem>,
    }

    impl crate::display_source::DisplayItemSource for OnceSource {
        fn next_item(
            &mut self,
            _context: &mut crate::display_source::DisplaySourceContext<'_>,
        ) -> Option<crate::display_item::DisplayItem> {
            self.item.take()
        }
    }

    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut test_base_face = resolver.default_face().clone();
    test_base_face.font_char_width = 8.0;
    test_base_face.font_ascent = 12.0;
    let base_face_id = 1;
    let mut face_ids = FrameFaceIdAllocator::new(2);
    let mut source = OnceSource {
        item: Some(crate::display_item::DisplayItem::new(
            crate::display_item::SourceSpan::synthetic(9, 0, 1),
            crate::display_item::RenderFaceRef::FaceId(base_face_id),
            crate::display_item::DisplayItemKind::SourceMappedText(
                crate::display_item::DisplaySourceMappedText::new("ABC"),
            ),
        )),
    };
    let mut state = DisplayRowSourceState::default();
    let mut context = DisplayRowRenderContext::new(&resolver, None, &mut face_ids);

    let first = DisplayRowItemSourceRenderRequest::new(display_row_request_for_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 16.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        base_face_id,
        &test_base_face,
        GlyphRowRole::Text,
    ))
    .render_step_with_context(&mut renderer, &mut source, &mut state, &mut context)
    .expect("first row");
    let second = DisplayRowItemSourceRenderRequest::new(display_row_request_for_face(
        DisplayRowGeometry {
            y: 16.0,
            width: 16.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        base_face_id,
        &test_base_face,
        GlyphRowRole::Text,
    ))
    .render_step_with_context(&mut renderer, &mut source, &mut state, &mut context)
    .expect("second row");

    assert_eq!(row_text_expanding_stretches(&first.rendered.row), "AB");
    assert_eq!(row_text_expanding_stretches(&second.rendered.row), "C");
}

#[test]
fn display_row_renderer_accepts_direct_text_run_measurement_policy() {
    struct OnceSource {
        item: Option<crate::display_item::DisplayItem>,
    }

    impl crate::display_source::DisplayItemSource for OnceSource {
        fn next_item(
            &mut self,
            _context: &mut crate::display_source::DisplaySourceContext<'_>,
        ) -> Option<crate::display_item::DisplayItem> {
            self.item.take()
        }
    }

    struct DirectTextRunPolicy;

    impl DisplayRowRenderPolicy for DirectTextRunPolicy {
        fn measurement_for(
            &mut self,
            _item: &crate::display_item::DisplayItem,
            _face_id: u32,
            _font_metrics: &mut Option<FontMetricsService>,
        ) -> DisplayRowItemMeasurement {
            DisplayRowItemMeasurement::TextRun(
                crate::display_text_run_measurement::DisplayTextRunMeasurementPlan::uniform_for_text(
                    "ABC", 5.0,
                ),
            )
        }
    }

    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let base_face = resolver.default_face();
    let base_face_id = 1;
    let mut face_ids = FrameFaceIdAllocator::new(2);
    let mut source = OnceSource {
        item: Some(crate::display_item::DisplayItem::new(
            crate::display_item::SourceSpan::synthetic(10, 0, 3),
            crate::display_item::RenderFaceRef::FaceId(base_face_id),
            crate::display_item::DisplayItemKind::TextRun(
                crate::display_item::DisplayTextRun::new("ABC"),
            ),
        )),
    };
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    let mut state = DisplayRowSourceState::default();
    let mut policy = DirectTextRunPolicy;
    let mut context = DisplayRowRenderContext::new(&resolver, None, &mut face_ids);

    let result = DisplayRowItemSourceRenderRequest::new(display_row_request_for_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        base_face_id,
        base_face,
        GlyphRowRole::Text,
    ))
    .render_fragment_step_into_row_with_policy(
        &mut renderer,
        &mut row,
        &mut source,
        &mut state,
        &mut context,
        &mut policy,
    )
    .expect("rendered row");

    assert_eq!(result.progress.end_x, 15.0);
    assert_eq!(result.progress.end_col, 3);
    assert_eq!(row.glyphs[GlyphArea::Text.index()][0].pixel_width, 5.0);
}

fn row_text_expanding_stretches(row: &GlyphRow) -> String {
    row.glyphs[1]
        .iter()
        .filter(|glyph| !glyph.padding)
        .flat_map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } => std::iter::repeat_n(*ch, 1).collect::<Vec<_>>(),
            GlyphType::Composite { text } => text.chars().collect::<Vec<_>>(),
            GlyphType::Stretch { width_cols } => {
                std::iter::repeat_n(' ', usize::from(*width_cols)).collect::<Vec<_>>()
            }
            _ => Vec::new(),
        })
        .collect()
}

fn row_text_glyph_types(row: &GlyphRow) -> Vec<GlyphType> {
    row.glyphs[1]
        .iter()
        .filter(|glyph| !glyph.padding)
        .map(|glyph| glyph.glyph_type.clone())
        .collect()
}

fn row_text_face_ids(row: &GlyphRow) -> Vec<u32> {
    row.glyphs[1]
        .iter()
        .filter(|glyph| !glyph.padding)
        .map(|glyph| glyph.face_id)
        .collect()
}

#[test]
fn display_row_geometry_builds_row_layout() {
    let tab_policy =
        crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(8.0, 4, &[6]);
    let geometry = DisplayRowGeometry {
        y: 20.0,
        width: 120.0,
        height: 16.0,
        char_width: 8.0,
        ascent: 11.0,
        tab_policy: tab_policy.clone(),
    };

    let layout = geometry.to_layout(
        GlyphRowRole::Text,
        9.0,
        12.0,
        RenderFaceRef::FaceId(42),
        std::collections::HashMap::new(),
    );

    assert_eq!(layout.role, GlyphRowRole::Text);
    assert_eq!(layout.y_px, 20.0);
    assert_eq!(layout.width_px, 120.0);
    assert_eq!(layout.height_px, 16.0);
    assert_eq!(layout.ascent_px, 12.0);
    assert_eq!(layout.char_width_px, 9.0);
    assert_eq!(layout.tab_policy, tab_policy);
    assert_eq!(layout.base_face, RenderFaceRef::FaceId(42));
}

fn render_lisp_display_row(rendered: Value, role: GlyphRowRole) -> GlyphRow {
    render_lisp_display_row_with_symbols(rendered, role, std::collections::HashMap::new())
}

fn render_lisp_display_row_with_symbols(
    rendered: Value,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
) -> GlyphRow {
    render_lisp_display_row_output_with_symbols(rendered, role, symbol_values).row
}

fn render_lisp_display_row_output(rendered: Value, role: GlyphRowRole) -> RenderedDisplayRow {
    render_lisp_display_row_output_with_symbols(rendered, role, std::collections::HashMap::new())
}

fn render_lisp_display_row_output_with_symbols(
    rendered: Value,
    role: GlyphRowRole,
    symbol_values: std::collections::HashMap<String, Value>,
) -> RenderedDisplayRow {
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        resolver.default_face(),
        role,
        symbol_values,
    );
    DisplayRowLispStringRenderRequest::new(request, rendered)
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row")
}

fn render_buffer_display_row(text: &str, role: GlyphRowRole) -> GlyphRow {
    render_buffer_display_row_with_properties(text, Vec::new(), role)
}

fn render_buffer_display_row_with_property(
    text: &str,
    property_start: usize,
    property_end: usize,
    property_name: Value,
    property_value: Value,
    role: GlyphRowRole,
) -> GlyphRow {
    render_buffer_display_row_with_properties(
        text,
        vec![(property_start, property_end, property_name, property_value)],
        role,
    )
}

fn render_buffer_display_row_with_properties(
    text: &str,
    properties: Vec<(usize, usize, Value, Value)>,
    role: GlyphRowRole,
) -> GlyphRow {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.insert(text);
        for (property_start, property_end, property_name, property_value) in properties {
            let start = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(property_start));
            let end = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(property_end));
            buffer.text_props_put_property_in_emacs_byte_range(
                EmacsByteRange::new(start, end),
                property_name,
                property_value,
            );
        }
    }

    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        resolver.default_face(),
        role,
        std::collections::HashMap::new(),
    );
    let mut source = crate::display_buffer_text_source::BufferTextSourceCursor::new(
        buf_id,
        &snapshot,
        CharPos0::ZERO,
        snapshot.layout_point_max_char_pos(),
        request.base_face_ref(),
    );

    DisplayRowItemSourceRenderRequest::new(request)
        .render(&mut renderer, &mut source, &resolver, &mut face_ids)
        .expect("buffer display source row")
        .row
}

#[test]
fn render_display_item_source_row_accepts_buffer_text_source() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("A中👨‍👩");
    }
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let mut source = crate::display_buffer_text_source::BufferTextSourceCursor::new(
        buf_id,
        buffer,
        neovm_core::buffer::CharPos0::new(0),
        buffer.layout_point_max_char_pos(),
        RenderFaceRef::FaceId(1),
    );

    let rendered = DisplayRowItemSourceRenderRequest::new(display_row_request_for_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        1,
        resolver.default_face(),
        GlyphRowRole::TabLine,
    ))
    .render(&mut renderer, &mut source, &resolver, &mut face_ids)
    .expect("display source row");

    assert_eq!(rendered.source_slots.len(), 5);
    assert_eq!(
        rendered.source_slots[0].source,
        crate::display_item::DisplaySourcePosition::buffer(
            buf_id,
            CharPos0::new(0),
            EmacsBytePos::new(0)
        )
    );
    assert_eq!(
        rendered.source_slots[1].source,
        crate::display_item::DisplaySourcePosition::buffer(
            buf_id,
            CharPos0::new(1),
            EmacsBytePos::new(1)
        )
    );
    assert_eq!(rendered.source_slots[0].width_cols, 1);
    assert_eq!(rendered.source_slots[1].width_cols, 2);

    let row = rendered.row;
    let glyphs = &row.glyphs[1];
    let cjk = glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("CJK glyph");

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert_eq!(row_text_expanding_stretches(&row), "A中👨‍👩");
    assert!(cjk.wide);
    assert!(glyphs.iter().any(|glyph| glyph.padding));
    assert!(
        glyphs.iter().any(
            |glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))
        )
    );
}

#[test]
fn render_lisp_string_row_records_xwidget_media_fragments() {
    let eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let xwidget = Value::make_xwidget(
        Value::symbol("webkit"),
        Value::string("Title"),
        Value::make_buffer(buf_id),
        96,
        54,
        1234,
    );
    let rendered_text = Value::string_with_text_properties(
        "AXB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("xwidget"),
                    Value::keyword("xwidget"),
                    xwidget,
                ]),
            ]),
        }],
    );
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    base_face.font_line_height = 16.0;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let spec = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 4.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &base_face,
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let rendered = DisplayRowLispStringRenderRequest::new(spec, rendered_text)
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row");

    let glyphs = &rendered.row.glyphs[1];
    assert_eq!(
        row_text_expanding_stretches(&rendered.row),
        "A            B"
    );
    assert!(matches!(
        glyphs[1].glyph_type,
        GlyphType::Stretch { width_cols: 12 }
    ));
    assert_eq!(
        rendered.media,
        vec![RenderedDisplayRowMedia {
            kind: RenderedDisplayRowMediaKind::Xwidget { xwidget_id: 1234 },
            x: 8.0,
            y: 4.0,
            col: 1,
            width: 96.0,
            height: 54.0,
        }]
    );
}

fn render_tab_line_with_media_host(
    rendered_text: Value,
    default_fg: u32,
    default_bg: u32,
) -> (RenderedDisplayRow, RecordingDisplayRowMediaHost) {
    let host = RecordingDisplayRowMediaHost::default();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, default_fg, default_bg, 14.0, None);
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    base_face.font_line_height = 16.0;
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let spec = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 4.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &base_face,
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );
    let rendered = DisplayRowLispStringRenderRequest::new(spec, rendered_text)
        .render_with_display_host(&mut renderer, &resolver, Some(&host), &mut face_ids)
        .expect("display source row");
    (rendered, host)
}

#[test]
fn render_lisp_string_row_resolves_image_display_property_through_display_host() {
    let _eval = Context::new();
    let rendered_text = Value::string_with_text_properties(
        "AXB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("image"),
                    Value::keyword("type"),
                    Value::symbol("png"),
                    Value::keyword("file"),
                    Value::string("/tmp/chrome.png"),
                ]),
            ]),
        }],
    );
    let (rendered, host) = render_tab_line_with_media_host(rendered_text, 0x00112233, 0x00445566);

    assert_eq!(
        rendered.media,
        vec![RenderedDisplayRowMedia {
            kind: RenderedDisplayRowMediaKind::Image { image_id: 42 },
            x: 8.0,
            y: 4.0,
            col: 1,
            width: 64.0,
            height: 32.0,
        }]
    );
    let requests = host.image_requests.lock().expect("image requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].fg_color, 0x00112233);
    assert_eq!(requests[0].bg_color, 0x00445566);
}

#[test]
fn render_lisp_string_row_resolves_video_display_property_through_display_host() {
    let _eval = Context::new();
    let rendered_text = Value::string_with_text_properties(
        "AVB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("video"),
                    Value::keyword("file"),
                    Value::string("/tmp/chrome.mp4"),
                    Value::keyword("width"),
                    Value::fixnum(120),
                    Value::keyword("height"),
                    Value::fixnum(45),
                    Value::keyword("loop"),
                    Value::symbol("t"),
                    Value::keyword("autoplay"),
                    Value::symbol("t"),
                ]),
            ]),
        }],
    );
    let (rendered, host) = render_tab_line_with_media_host(rendered_text, 0x00FFFFFF, 0x00000000);

    assert_eq!(
        rendered.media,
        vec![RenderedDisplayRowMedia {
            kind: RenderedDisplayRowMediaKind::Video {
                video_id: 84,
                loop_count: -1,
                autoplay: true,
            },
            x: 8.0,
            y: 4.0,
            col: 1,
            width: 120.0,
            height: 45.0,
        }]
    );
    assert_eq!(
        host.video_requests
            .lock()
            .expect("video requests lock")
            .len(),
        1
    );
}

#[test]
fn render_lisp_string_row_resolves_webkit_display_property_through_display_host() {
    let _eval = Context::new();
    let rendered_text = Value::string_with_text_properties(
        "AWB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("webkit"),
                    Value::keyword("uri"),
                    Value::string("https://example.invalid/"),
                    Value::keyword("width"),
                    Value::fixnum(80),
                    Value::keyword("height"),
                    Value::fixnum(50),
                ]),
            ]),
        }],
    );
    let (rendered, host) = render_tab_line_with_media_host(rendered_text, 0x00FFFFFF, 0x00000000);

    assert_eq!(
        rendered.media,
        vec![RenderedDisplayRowMedia {
            kind: RenderedDisplayRowMediaKind::Xwidget { xwidget_id: 99 },
            x: 8.0,
            y: 4.0,
            col: 1,
            width: 80.0,
            height: 50.0,
        }]
    );
    assert_eq!(
        host.webkit_requests
            .lock()
            .expect("webkit requests lock")
            .len(),
        1
    );
}

#[test]
fn display_row_buffer_and_lisp_sources_share_display_space_semantics() {
    let _eval = Context::new();
    let display_space = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("align-to"),
        Value::fixnum(4),
    ]);
    let lisp_row = render_lisp_display_row(
        Value::string_with_text_properties(
            "A B",
            vec![neovm_core::emacs_core::value::StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![Value::symbol("display"), display_space.clone()]),
            }],
        ),
        GlyphRowRole::HeaderLine,
    );
    let buffer_row = render_buffer_display_row_with_property(
        "A B",
        1,
        2,
        Value::symbol("display"),
        display_space,
        GlyphRowRole::HeaderLine,
    );

    assert_eq!(row_text_expanding_stretches(&buffer_row), "A   B");
    assert_eq!(
        row_text_expanding_stretches(&buffer_row),
        row_text_expanding_stretches(&lisp_row)
    );
    assert_eq!(
        buffer_row.glyphs[1]
            .iter()
            .filter(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { .. }))
            .count(),
        lisp_row.glyphs[1]
            .iter()
            .filter(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { .. }))
            .count()
    );
}

#[test]
fn display_row_buffer_and_lisp_sources_share_display_replacement_string_semantics() {
    let _eval = Context::new();
    let replacement = Value::string("YZ");
    let lisp_row = render_lisp_display_row(
        Value::string_with_text_properties(
            "axb",
            vec![neovm_core::emacs_core::value::StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![Value::symbol("display"), replacement]),
            }],
        ),
        GlyphRowRole::TabLine,
    );
    let buffer_row = render_buffer_display_row_with_property(
        "axb",
        1,
        2,
        Value::symbol("display"),
        replacement,
        GlyphRowRole::TabLine,
    );

    assert_eq!(row_text_expanding_stretches(&buffer_row), "aYZb");
    assert_eq!(
        row_text_expanding_stretches(&buffer_row),
        row_text_expanding_stretches(&lisp_row)
    );
    assert_eq!(
        row_text_glyph_types(&buffer_row),
        row_text_glyph_types(&lisp_row)
    );
}

#[test]
fn display_row_buffer_and_lisp_sources_share_face_property_semantics() {
    let _eval = Context::new();
    let face = Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]);
    let lisp_row = render_lisp_display_row(
        Value::string_with_text_properties(
            "AB",
            vec![neovm_core::emacs_core::value::StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![Value::symbol("face"), face.clone()]),
            }],
        ),
        GlyphRowRole::ModeLine,
    );
    let buffer_row = render_buffer_display_row_with_property(
        "AB",
        1,
        2,
        Value::symbol("face"),
        face,
        GlyphRowRole::ModeLine,
    );

    assert_eq!(row_text_expanding_stretches(&buffer_row), "AB");
    assert_eq!(row_text_face_ids(&buffer_row), row_text_face_ids(&lisp_row));
    let face_ids = row_text_face_ids(&buffer_row);
    assert_ne!(
        face_ids[0], face_ids[1],
        "buffer face property should split the row face like Lisp-string chrome"
    );
}

#[test]
fn display_row_buffer_and_lisp_sources_share_raise_property_semantics() {
    let _eval = Context::new();
    let raise = Value::list(vec![Value::symbol("raise"), Value::make_float(0.25)]);
    let lisp_row = render_lisp_display_row(
        Value::string_with_text_properties(
            "AB",
            vec![neovm_core::emacs_core::value::StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![Value::symbol("display"), raise.clone()]),
            }],
        ),
        GlyphRowRole::ModeLine,
    );
    let buffer_row = render_buffer_display_row_with_property(
        "AB",
        1,
        2,
        Value::symbol("display"),
        raise,
        GlyphRowRole::ModeLine,
    );

    assert_eq!(row_text_expanding_stretches(&buffer_row), "AB");
    assert_eq!(
        row_text_expanding_stretches(&buffer_row),
        row_text_expanding_stretches(&lisp_row)
    );
    assert_eq!(
        buffer_row.glyphs[1]
            .iter()
            .map(|glyph| glyph.vertical_offset_px)
            .collect::<Vec<_>>(),
        lisp_row.glyphs[1]
            .iter()
            .map(|glyph| glyph.vertical_offset_px)
            .collect::<Vec<_>>()
    );
    assert_eq!(buffer_row.glyphs[1][0].vertical_offset_px, 0.0);
    assert_eq!(buffer_row.glyphs[1][1].vertical_offset_px, -4.0);
}

#[test]
fn display_row_buffer_and_lisp_sources_share_height_property_semantics() {
    let _eval = Context::new();
    let height = Value::list(vec![Value::symbol("height"), Value::make_float(2.0)]);
    let lisp_row = render_lisp_display_row(
        Value::string_with_text_properties(
            "AB",
            vec![neovm_core::emacs_core::value::StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![Value::symbol("display"), height.clone()]),
            }],
        ),
        GlyphRowRole::ModeLine,
    );
    let buffer_row = render_buffer_display_row_with_property(
        "AB",
        1,
        2,
        Value::symbol("display"),
        height,
        GlyphRowRole::ModeLine,
    );

    assert_eq!(row_text_expanding_stretches(&buffer_row), "AB");
    assert_eq!(
        row_text_expanding_stretches(&buffer_row),
        row_text_expanding_stretches(&lisp_row)
    );
    assert_eq!(row_text_face_ids(&buffer_row), row_text_face_ids(&lisp_row));
    assert_ne!(
        buffer_row.glyphs[1][0].face_id,
        buffer_row.glyphs[1][1].face_id
    );
    assert_eq!(buffer_row.height_px, lisp_row.height_px);
    assert_eq!(buffer_row.ascent_px, lisp_row.ascent_px);
    assert_eq!(buffer_row.height_px, 32.0);
    assert_eq!(buffer_row.ascent_px, 24.0);
}

#[test]
fn display_row_buffer_and_lisp_sources_share_control_and_glyphless_semantics() {
    let _eval = Context::new();
    let text = "a\u{0001}\u{fff0}b";
    let lisp_row = render_lisp_display_row(Value::string(text), GlyphRowRole::HeaderLine);
    let buffer_row = render_buffer_display_row(text, GlyphRowRole::HeaderLine);

    assert_eq!(
        row_text_glyph_types(&buffer_row),
        row_text_glyph_types(&lisp_row)
    );
    assert!(
        row_text_glyph_types(&buffer_row)
            .iter()
            .any(|kind| matches!(kind, GlyphType::Glyphless { ch: '\u{fff0}' })),
        "glyphless buffer source chars should reach the same row builder path as Lisp strings"
    );
}

#[test]
fn render_display_item_source_row_uses_spec_tab_policy() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("\tX");
    }
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let mut source = crate::display_buffer_text_source::BufferTextSourceCursor::new(
        buf_id,
        buffer,
        neovm_core::buffer::CharPos0::new(0),
        buffer.layout_point_max_char_pos(),
        RenderFaceRef::FaceId(1),
    );

    let rendered = DisplayRowItemSourceRenderRequest::new(display_row_request_for_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(
                0.0,
                4,
                &[2],
            ),
        },
        1,
        resolver.default_face(),
        GlyphRowRole::TabLine,
    ))
    .render(&mut renderer, &mut source, &resolver, &mut face_ids)
    .expect("display source row");

    let glyphs = &rendered.row.glyphs[1];
    assert_eq!(glyphs[0].glyph_type, GlyphType::Stretch { width_cols: 2 });
    let emitted_width: f32 = glyphs.iter().map(|glyph| glyph.pixel_width).sum();
    assert!(
        (rendered.progress.end_x - emitted_width).abs() <= 0.01,
        "row progress should include the emitted tab stretch and following character"
    );
}

#[test]
fn render_lisp_string_row_uses_explicit_tab_policy() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let mut renderer = DisplayRowRenderer::new(&mut engine.font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let spec = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(
                0.0,
                4,
                &[2],
            ),
        },
        &mut face_ids,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let rendered = DisplayRowLispStringRenderRequest::new(spec, Value::string("\tX"))
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row");

    let glyphs = &rendered.row.glyphs[1];
    assert_eq!(glyphs[0].glyph_type, GlyphType::Stretch { width_cols: 2 });
}

#[test]
fn display_row_glyph_measurer_uses_face_specific_widths() {
    let mut base = base_face();
    base.font_char_width = 5.0;
    let mut wide = base.clone();
    wide.font_char_width = 9.0;
    let faces = vec![
        DisplayRowFace::from_resolved(1, &base),
        DisplayRowFace::from_resolved(2, &wide),
    ];
    let mut measurer = DisplayRowGlyphMeasurer::new(&faces, None, 5.0);

    assert_eq!(measurer.glyph_advance_px('a', 1, 1, 5.0), Some(5.0));
    assert_eq!(measurer.glyph_advance_px('中', 2, 2, 10.0), Some(18.0));
}

#[test]
fn display_row_glyph_measurer_preserves_fractional_gui_advances() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let faces = vec![DisplayRowFace::from_resolved(1, &base)];
    let mut measurer = DisplayRowGlyphMeasurer::new(&faces, None, 7.2);

    assert_eq!(measurer.glyph_advance_px('x', 1, 1, 7.2), Some(7.2));
}

#[test]
fn display_row_glyph_measurer_can_snap_terminal_advances() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let faces = vec![DisplayRowFace::from_resolved(1, &base)];
    let mut measurer = DisplayRowGlyphMeasurer::with_quantization(
        &faces,
        None,
        7.2,
        GlyphAdvanceQuantization::SnapToIntegerPixels,
    );

    assert_eq!(measurer.glyph_advance_px('x', 1, 1, 7.2), Some(7.0));
}

#[test]
fn display_row_glyph_measurement_face_measures_single_char_columns() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let face = DisplayRowFace::from_resolved(8, &base);
    let measurement_face = DisplayRowGlyphMeasurementFace::with_mode(
        face,
        DisplayRowMeasurementMode::FallbackMetrics,
        7.2,
        GlyphAdvanceQuantization::SnapToIntegerPixels,
    );
    let mut font_metrics = None;

    assert_eq!(
        measurement_face.advance_for_char(&mut font_metrics, '.', 7.2),
        7.0
    );
    assert_eq!(
        measurement_face.advance_for_char(&mut font_metrics, '中', 14.4),
        14.0
    );
}

#[test]
fn display_row_glyph_measurement_face_constructs_from_resolved_face_policy() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let measurement_face =
        DisplayRowMeasurementPolicy::for_frame(false).measurement_face(8, &base, None, 7.2);
    let mut font_metrics = None;

    assert_eq!(
        measurement_face.advance_for_char(&mut font_metrics, '.', 7.2),
        7.0
    );
}

#[test]
fn display_row_measurement_policy_builds_faces_from_frame_mode() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let tty_policy = DisplayRowMeasurementPolicy::for_frame(false);
    let gui_policy = DisplayRowMeasurementPolicy::for_frame(true);
    let mut font_metrics = None;

    let tty_face = tty_policy.measurement_face(8, &base, None, 7.2);
    let gui_face = gui_policy.measurement_face(8, &base, None, 7.2);

    assert_eq!(tty_face.advance_for_char(&mut font_metrics, '.', 7.2), 7.0);
    assert_eq!(gui_face.advance_for_char(&mut font_metrics, '.', 7.2), 7.2);
}

#[test]
fn display_row_fallback_metrics_builds_from_default_face_extents() {
    let fallback = DisplayRowFallbackMetrics::from_default_face_extents(7.5, 18.0, 13.0);

    assert_eq!(
        fallback,
        DisplayRowFallbackMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
        }
    );
}

#[test]
fn display_row_measurement_policy_builds_measured_face_with_space_width() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let policy = DisplayRowMeasurementPolicy::for_frame(false);
    let mut font_metrics = None;

    let active = DisplayRowActiveFaceState::new(
        base.clone(),
        policy.measured_face(
            8,
            &base,
            None,
            7.2,
            DisplayRowFallbackMetrics {
                char_width: 7.2,
                row_height: 16.0,
                ascent: 11.0,
            },
            &mut font_metrics,
        ),
    );

    assert_eq!(active.metrics().space_width, 7.0);
    assert_eq!(active.advance_for_char(&mut font_metrics, 'x', 7.2), 7.0);
    assert_eq!(active.advance_for_columns(&mut font_metrics, 'x', 2), 14.0);

    let text_run_measurement = active.text_run_measurement(&mut font_metrics, "a中");
    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(advances) =
        text_run_measurement
    else {
        panic!("active face should produce text-run measurement plans");
    };
    assert_eq!(
        advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset, advance.advance_px))
            .collect::<Vec<_>>(),
        vec![(0, 0, 7.0), (1, 1, 14.0)]
    );
}

#[test]
fn display_row_measurement_policy_builds_measured_face_with_line_metrics() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let metrics = crate::font_metrics::FontMetrics {
        ascent: 13.0,
        descent: 5.0,
        line_height: 18.0,
        char_width: 9.0,
    };
    let policy = DisplayRowMeasurementPolicy::for_frame(true);
    let mut font_metrics = None;

    let measured = policy.measured_face(
        8,
        &base,
        Some(metrics),
        7.2,
        DisplayRowFallbackMetrics {
            char_width: 7.2,
            row_height: 16.0,
            ascent: 11.0,
        },
        &mut font_metrics,
    );

    let measured_metrics = measured.metrics();
    assert_eq!(measured_metrics.char_width, 9.0);
    assert_eq!(measured_metrics.row_height, 18.0);
    assert_eq!(measured_metrics.ascent, 13.0);
}

#[test]
fn display_row_measured_face_exposes_face_identity() {
    let base = base_face();
    let policy = DisplayRowMeasurementPolicy::for_frame(false);
    let mut font_metrics = None;

    let measured = policy.measured_face(
        42,
        &base,
        None,
        7.2,
        DisplayRowFallbackMetrics {
            char_width: 7.2,
            row_height: 16.0,
            ascent: 11.0,
        },
        &mut font_metrics,
    );

    let active = DisplayRowActiveFaceState::new(base, measured);
    assert_eq!(active.face_id(), 42);
}

#[test]
fn display_row_measured_face_exposes_metrics_as_single_value() {
    let base = base_face();
    let policy = DisplayRowMeasurementPolicy::for_frame(false);
    let mut font_metrics = None;

    let measured = policy.measured_face(
        42,
        &base,
        None,
        7.2,
        DisplayRowFallbackMetrics {
            char_width: 7.2,
            row_height: 16.0,
            ascent: 11.0,
        },
        &mut font_metrics,
    );

    let metrics = measured.metrics();

    assert_eq!(metrics.char_width, 7.2);
    assert_eq!(metrics.row_height, 16.0);
    assert_eq!(metrics.ascent, 11.0);
    assert_eq!(metrics.space_width, 7.0);
}

#[test]
fn display_row_glyph_measurement_face_shapes_text_runs_as_measurement_plans() {
    let mut base = base_face();
    base.font_family = "monospace".to_string();
    base.font_size = 14.0;
    base.font_char_width = 8.0;
    let measurement_face =
        DisplayRowMeasurementPolicy::for_frame(true).measurement_face(8, &base, None, 8.0);
    let mut font_metrics = Some(FontMetricsService::new());

    let measurement = measurement_face.text_run_measurement(&mut font_metrics, "سلام");

    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(advances) =
        measurement
    else {
        panic!("complex script run should produce a measured text-run plan");
    };
    assert!(
        !advances.is_empty(),
        "complex script run should produce cluster advances"
    );
    assert!(
        advances.iter().all(|advance| advance.advance_px >= 0.0),
        "cluster advances should never be negative: {advances:?}"
    );
}

#[test]
fn display_text_run_measurement_plan_builds_from_shaped_glyphs() {
    fn shaped(cluster_start: usize, x_advance: f32) -> crate::font_metrics::ShapedGlyph {
        crate::font_metrics::ShapedGlyph {
            font_id: fontdb::ID::dummy(),
            glyph_id: 1,
            x: 0.0,
            y: 0.0,
            x_advance,
            cluster_start,
            cluster_end: cluster_start + 1,
        }
    }

    let measurement =
        crate::display_text_run_measurement::DisplayTextRunMeasurementPlan::from_shaped_glyphs(
            "aéb",
            [shaped(0, 2.0), shaped(1, 7.0), shaped(3, 8.5)],
            6.0,
            4.0,
            GlyphAdvanceQuantization::PreserveLogicalPixels,
        );

    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(advances) =
        measurement
    else {
        panic!("shaped glyphs should produce measured text-run advances");
    };
    assert_eq!(
        advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset, advance.advance_px))
            .collect::<Vec<_>>(),
        vec![(0, 0, 6.0), (1, 1, 7.0), (2, 3, 8.5)]
    );
}

#[test]
fn display_row_glyph_measurer_builds_measured_text_run_plan() {
    let mut base = base_face();
    base.font_family = "monospace".to_string();
    base.font_size = 14.0;
    base.font_char_width = 8.0;
    let faces = vec![DisplayRowFace::from_resolved(8, &base)];
    let mut font_metrics = FontMetricsService::new();
    let mut measurer = DisplayRowGlyphMeasurer::new(&faces, Some(&mut font_metrics), 8.0);

    let measurement = measurer.text_run_advances_px("abc", 8, 8.0);

    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(advances) =
        measurement
    else {
        panic!("font-backed measurer should produce a measured text-run plan");
    };
    assert_eq!(
        advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset))
            .collect::<Vec<_>>(),
        vec![(0, 0), (1, 1), (2, 2)]
    );
    assert!(
        advances.iter().all(|advance| advance.advance_px >= 8.0),
        "measured advances should respect the frame cell minimum: {advances:?}"
    );
}

#[test]
fn display_row_glyph_measurement_face_builds_text_run_measurement_plan() {
    let mut base = base_face();
    base.font_family = "monospace".to_string();
    base.font_size = 14.0;
    base.font_char_width = 8.0;
    let measurement_face =
        DisplayRowMeasurementPolicy::for_frame(true).measurement_face(8, &base, None, 8.0);
    let mut font_metrics = Some(FontMetricsService::new());

    let measurement = measurement_face.text_run_measurement(&mut font_metrics, "abc");

    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(advances) =
        measurement
    else {
        panic!("font-backed measurement face should produce a measured text-run plan");
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
fn display_row_glyph_measurement_face_builds_fallback_text_run_measurement_plan() {
    let mut base = base_face();
    base.font_char_width = 7.2;
    let measurement_face =
        DisplayRowMeasurementPolicy::for_frame(false).measurement_face(8, &base, None, 7.2);
    let mut font_metrics = None;

    let measurement = measurement_face.text_run_measurement(&mut font_metrics, "a中");

    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(advances) =
        measurement
    else {
        panic!("measurement face should fall back to char-advance text-run plans");
    };
    assert_eq!(
        advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset, advance.advance_px))
            .collect::<Vec<_>>(),
        vec![(0, 0, 7.0), (1, 1, 14.0)]
    );
}

#[test]
fn display_text_run_measurement_plan_builds_resolved_source_advance() {
    let measurement =
        crate::display_text_run_measurement::DisplayTextRunMeasurementPlan::from_resolved_source_advance(
            "\u{301}",
            0.0,
        );

    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(advances) =
        measurement
    else {
        panic!("resolved source advances should produce measured text-run plans");
    };
    assert_eq!(
        advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset, advance.advance_px))
            .collect::<Vec<_>>(),
        vec![(0, 0, 0.0)]
    );

    let wide_measurement =
        crate::display_text_run_measurement::DisplayTextRunMeasurementPlan::from_resolved_source_advance(
            "中", 14.0,
        );
    let crate::display_text_run_measurement::DisplayTextRunMeasurement::Measured(wide_advances) =
        wide_measurement
    else {
        panic!("resolved wide source advance should produce a measured text-run plan");
    };
    assert_eq!(
        wide_advances
            .iter()
            .map(|advance| (advance.char_offset, advance.byte_offset, advance.advance_px))
            .collect::<Vec<_>>(),
        vec![(0, 0, 14.0)]
    );
}

#[test]
fn display_row_baseline_tab_bar_preserves_lisp_string_face_properties() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "AB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::TabBar);
    let glyphs = &row.glyphs[1];

    assert_eq!(row.role, GlyphRowRole::TabBar);
    assert_eq!(row_text_expanding_stretches(&row), "AB");
    assert_eq!(glyphs.len(), 2);
    assert_ne!(
        glyphs[0].face_id, glyphs[1].face_id,
        "propertized tab-bar chars should keep separate face ids"
    );
}

#[test]
fn display_row_baseline_tab_bar_preserves_lisp_string_raise_property() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "AB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![Value::symbol("raise"), Value::make_float(0.25)]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::TabBar);
    let glyphs = &row.glyphs[1];

    assert_eq!(row.role, GlyphRowRole::TabBar);
    assert_eq!(row_text_expanding_stretches(&row), "AB");
    assert_eq!(glyphs.len(), 2);
    assert_eq!(glyphs[0].vertical_offset_px, 0.0);
    assert_eq!(glyphs[1].vertical_offset_px, -4.0);
}

#[test]
fn display_row_baseline_tab_bar_preserves_lisp_string_height_property() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "AB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![Value::symbol("height"), Value::make_float(2.0)]),
            ]),
        }],
    );

    let rendered = render_lisp_display_row_output(rendered, GlyphRowRole::TabBar);
    let row = &rendered.row;
    let glyphs = &row.glyphs[1];

    assert_eq!(row.role, GlyphRowRole::TabBar);
    assert_eq!(row_text_expanding_stretches(row), "AB");
    assert_eq!(glyphs.len(), 2);
    assert_ne!(
        glyphs[0].face_id, glyphs[1].face_id,
        "height display property should realize a separate face like GNU face_with_height"
    );
    let raised_face = rendered
        .faces
        .iter()
        .find(|face| face.id == glyphs[1].face_id)
        .expect("height-adjusted face");
    assert_eq!(raised_face.font_size, 28.0);
    assert_eq!(raised_face.font_ascent, 24);
    assert_eq!(row.height_px, 32.0);
    assert_eq!(row.ascent_px, 24.0);
    assert_eq!(rendered.progress.height, 32.0);
}

#[test]
fn display_row_baseline_mode_line_display_space_align_expands_to_spaces() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "A B",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::fixnum(4),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::ModeLine);

    assert_eq!(row.role, GlyphRowRole::ModeLine);
    assert_eq!(row_text_expanding_stretches(&row), "A   B");
}

#[test]
fn display_row_baseline_header_line_display_space_relative_width_expands_to_stretch() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "C R",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("relative-width"),
                    Value::fixnum(2),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::HeaderLine);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "C  R");
    assert!(
        row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 2 })),
        "relative-width display space should become a stretch glyph: {:?}",
        row.glyphs[1]
    );
}

#[test]
fn display_row_baseline_header_line_align_to_symbol_values() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "C ",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::list(vec![
                        Value::symbol("+"),
                        Value::symbol("header-line-indent-width"),
                        Value::fixnum(1),
                    ]),
                ]),
            ]),
        }],
    );
    let mut symbol_values = std::collections::HashMap::new();
    symbol_values.insert("header-line-indent-width".to_string(), Value::fixnum(0));

    let row =
        render_lisp_display_row_with_symbols(rendered, GlyphRowRole::HeaderLine, symbol_values);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "C");
}

#[test]
fn display_row_baseline_header_line_align_to_skips_multi_char_interval() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "X   Y",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 4,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::fixnum(4),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::HeaderLine);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "X   Y");
    assert!(
        row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 3 })),
        "multi-char display interval should become one stretch glyph: {:?}",
        row.glyphs[1]
    );
}

#[test]
fn display_row_baseline_header_line_align_to_after_multibyte_prefix_uses_character_offsets() {
    let _eval = Context::new();
    let rendered = Value::string_with_text_properties(
        "λC R",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 2,
            end: 3,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword("align-to"),
                    Value::fixnum(4),
                ]),
            ]),
        }],
    );

    let row = render_lisp_display_row(rendered, GlyphRowRole::HeaderLine);

    assert_eq!(row.role, GlyphRowRole::HeaderLine);
    assert_eq!(row_text_expanding_stretches(&row), "λC  R");
}

#[test]
fn render_lisp_string_row_uses_face_specific_glyph_widths() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let mut renderer = DisplayRowRenderer::new(&mut engine.font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut base_face = resolver.default_face().clone();
    base_face.font_char_width = 8.0;
    base_face.font_ascent = 12.0;
    let rendered = Value::string_with_text_properties(
        "AB",
        vec![neovm_core::emacs_core::value::StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![
                    Value::keyword("family"),
                    Value::string("JetBrains Mono"),
                    Value::keyword("height"),
                    Value::make_float(2.0),
                ]),
            ]),
        }],
    );
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let spec = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 32.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &base_face,
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let row = DisplayRowLispStringRenderRequest::new(spec, rendered)
        .render(&mut renderer, &resolver, &mut face_ids)
        .expect("display source row")
        .row;
    let glyphs = &row.glyphs[1];

    assert_eq!(glyphs.len(), 2);
    assert!(
        glyphs[1].pixel_width > glyphs[0].pixel_width,
        "face-height run should be measured wider than base run: {glyphs:?}"
    );
}

#[test]
fn display_row_lisp_string_render_request_uses_render_context() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let base_face = resolver.default_face().clone();
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &base_face,
        GlyphRowRole::TabBar,
        std::collections::HashMap::new(),
    );
    let mut context = DisplayRowRenderContext::new(&resolver, None, &mut face_ids);

    let rendered = DisplayRowLispStringRenderRequest::new(request, Value::string("ctx"))
        .render_with_context(&mut renderer, &mut context)
        .expect("rendered context row");

    assert_eq!(rendered.row.role, GlyphRowRole::TabBar);
    assert_eq!(row_text_expanding_stretches(&rendered.row), "ctx");
}

#[test]
fn display_row_render_executor_renders_lisp_string_request() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let base_face = resolver.default_face().clone();
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        &base_face,
        GlyphRowRole::TabBar,
        std::collections::HashMap::new(),
    );
    let mut executor =
        DisplayRowRenderExecutor::new(&mut font_metrics, &resolver, None, &mut face_ids);

    let rendered = executor
        .render_lisp_string_request(DisplayRowLispStringRenderRequest::new(
            request,
            Value::string("exec"),
        ))
        .expect("executor rendered lisp string row");

    assert_eq!(rendered.row.role, GlyphRowRole::TabBar);
    assert_eq!(row_text_expanding_stretches(&rendered.row), "exec");
}

#[test]
fn display_row_tab_line_wide_char_uses_shared_wide_glyph() {
    let _eval = Context::new();
    let row = render_lisp_display_row(Value::string("A中B"), GlyphRowRole::TabLine);
    let glyphs = &row.glyphs[1];
    let cjk = glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("CJK glyph");

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert_eq!(row_text_expanding_stretches(&row), "A中B");
    assert!(
        cjk.wide,
        "tab-line CJK should use the shared wide glyph path: {glyphs:?}"
    );
    assert!(
        glyphs.iter().any(|glyph| glyph.padding),
        "tab-line CJK should retain a padding cell like main buffer text: {glyphs:?}"
    );
}

#[test]
fn display_row_tab_line_zwj_emoji_sequence_uses_shared_cluster() {
    let _eval = Context::new();
    let row = render_lisp_display_row(Value::string("👨‍👩"), GlyphRowRole::TabLine);
    let glyphs = &row.glyphs[1];

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert_eq!(row_text_expanding_stretches(&row), "👨‍👩");
    assert!(
        glyphs
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
        "tab-line ZWJ emoji should use the shared cluster path: {glyphs:?}"
    );
}

#[test]
fn display_row_lisp_chrome_roles_share_wide_and_cluster_builder() {
    let _eval = Context::new();

    for role in [
        GlyphRowRole::ModeLine,
        GlyphRowRole::HeaderLine,
        GlyphRowRole::TabLine,
        GlyphRowRole::TabBar,
    ] {
        let row = render_lisp_display_row(Value::string("A中👨‍👩"), role);
        let glyphs = &row.glyphs[1];
        let cjk = glyphs
            .iter()
            .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
            .expect("CJK glyph");

        assert_eq!(row.role, role);
        assert_eq!(row_text_expanding_stretches(&row), "A中👨‍👩");
        assert!(
            cjk.wide,
            "Lisp-string chrome role {role:?} should use the shared wide-glyph path: {glyphs:?}"
        );
        assert!(
            glyphs.iter().any(|glyph| glyph.padding),
            "Lisp-string chrome role {role:?} should retain CJK padding cells: {glyphs:?}"
        );
        assert!(
            glyphs
                .iter()
                .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
            "Lisp-string chrome role {role:?} should use the shared cluster path: {glyphs:?}"
        );
    }
}

#[test]
fn display_row_tab_line_rtl_text_is_logical_order_at_render() {
    // Slice 5: render produces LOGICAL-order rows; the single bidi finalizer is
    // the matrix-row install (`end_current_row`). A render-only chrome row keeps
    // logical order and is not yet flagged reversed. The end-to-end reorder is
    // verified by install_rendered_display_row_finalizes_bidi_at_install and
    // (cross-row-kind) matrix_builder rtl_text_and_chrome_rows_reorder_identically.
    let _eval = Context::new();
    let row = render_lisp_display_row(Value::string("אב"), GlyphRowRole::TabLine);

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert!(!row.reversed_p);
    assert_eq!(row_text_expanding_stretches(&row), "אב");
}

#[test]
fn display_row_fragment_keeps_bidi_unfinalized_for_current_row_append() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );
    let base_face = request.base_face_ref();
    let mut source =
        crate::display_source::LispStringSourceCursor::new(1, Value::string("אב"), base_face)
            .expect("lisp string source");
    let mut state = DisplayRowSourceState::default();

    let fragment = DisplayRowItemSourceRenderRequest::new(request)
        .render_fragment_step_with_display_host(
            &mut renderer,
            &mut source,
            &mut state,
            &resolver,
            None,
            &mut face_ids,
        )
        .expect("unfinalized row fragment")
        .rendered
        .row;

    assert!(!fragment.reversed_p);
    assert_eq!(row_text_expanding_stretches(&fragment), "אב");

    // Slice 5: the non-fragment step path also defers bidi finalization to row
    // install now, so it likewise yields logical order here — the matrix-row
    // install (`end_current_row`) is the sole finalizer for both render entries.
    let step_path = render_lisp_display_row(Value::string("אב"), GlyphRowRole::TabLine);
    assert!(!step_path.reversed_p);
    assert_eq!(row_text_expanding_stretches(&step_path), "אב");
}

#[test]
fn display_row_renderer_can_render_source_fragment_into_existing_row() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    let request = display_row_request_from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut face_ids,
        resolver.default_face(),
        GlyphRowRole::Text,
        std::collections::HashMap::new(),
    )
    .with_render_bounds(DisplayRowRenderBounds {
        start: DisplayRowPosition { x_px: 8.0, col: 1 },
        max_x: DisplayRowMaxX::Bounded(240.0),
    });
    let base_face_id = request.base_face_id();
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    crate::glyph_row_writer::push_char_to_row(&mut row, 'e', base_face_id, 0, 8.0);
    let mut source = crate::display_source::LispStringSourceCursor::new(
        1,
        Value::string("\u{301}"),
        RenderFaceRef::FaceId(base_face_id),
    )
    .expect("lisp string source");
    let mut state = DisplayRowSourceState::default();

    let result = DisplayRowItemSourceRenderRequest::new(request)
        .render_fragment_step_into_row_with_display_host(
            &mut renderer,
            &mut row,
            &mut source,
            &mut state,
            &resolver,
            None,
            &mut face_ids,
        )
        .expect("row render fragment");

    assert_eq!(result.stop, DisplayRowRenderStop::SourceExhausted);
    assert_eq!(
        result.progress,
        DisplayRowOutputProgress {
            end_x: 8.0,
            end_col: 1,
            y: 0.0,
            height: 16.0,
        }
    );
    let text = &row.glyphs[GlyphArea::Text.index()];
    assert_eq!(text.len(), 1);
    assert!(matches!(
        &text[0].glyph_type,
        GlyphType::Composite { text } if text.as_ref() == "e\u{301}"
    ));
}

#[test]
fn mock_display_row_matrix_install_preserves_row_metadata() {
    let mut row = GlyphRow::new(GlyphRowRole::ModeLine);
    row.enabled = true;
    row.pixel_y = 40.0;
    row.height_px = 18.0;
    row.ascent_px = 13.0;
    row.start_charpos = 7;
    row.end_charpos = 8;
    row.glyphs[GlyphArea::Text.index()].push(Glyph::char('M', 3, 7));

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 2, 10, Rect::new(0.0, 16.0, 80.0, 40.0), true);
    install_mock_display_row_in_matrix_row(&mut builder, 1, &row);
    builder.end_window();

    let state = builder.finish(10, 2, 8.0, 16.0);
    let installed = &state.window_matrices[0].matrix.rows[1];
    assert_eq!(installed.role, GlyphRowRole::ModeLine);
    assert_eq!(installed.pixel_y, 24.0);
    assert_eq!(installed.height_px, 18.0);
    assert_eq!(installed.ascent_px, 13.0);
    assert_eq!(installed.start_charpos, 7);
    assert_eq!(installed.end_charpos, 8);
    assert!(matches!(
        installed.glyphs[GlyphArea::Text.index()][0].glyph_type,
        GlyphType::Char { ch: 'M' }
    ));
}

#[test]
fn install_measured_display_row_clips_window_chrome_media_to_measured_row() {
    let mut row = GlyphRow::new(GlyphRowRole::TabLine);
    row.enabled = true;
    row.pixel_y = 4.0;
    row.height_px = 54.0;
    row.ascent_px = 42.0;
    let rendered = RenderedDisplayRow {
        row,
        progress: DisplayRowOutputProgress {
            end_x: 0.0,
            end_col: 0,
            y: 4.0,
            height: 54.0,
        },
        source_slots: Vec::new(),
        faces: Vec::new(),
        media: vec![RenderedDisplayRowMedia {
            kind: RenderedDisplayRowMediaKind::Xwidget { xwidget_id: 1234 },
            x: 8.0,
            y: 4.0,
            col: 1,
            width: 96.0,
            height: 54.0,
        }],
    };
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let window_bounds = Rect::new(0.0, 0.0, 200.0, 80.0);
    let row_bounds = Rect::new(0.0, 4.0, 200.0, 54.0);
    builder.begin_window_with_text_bounds(
        77,
        1,
        10,
        window_bounds,
        Rect::new(10.0, 20.0, 160.0, 64.0),
        true,
    );

    let measured = MeasuredDisplayRow::new(
        DisplayRowOwner::WindowChrome {
            window_id: 77,
            kind: WindowChromeKind::TabLine,
        },
        0,
        row_bounds,
        rendered,
        DisplayRowBoundsPolicy::PreserveAllocatedMinimum,
    );
    DisplayRowInstaller::new(&mut builder).install_measured(&measured);
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let xwidget = state.xwidgets.first().expect("xwidget side item");
    assert_eq!(xwidget.window_id, 77);
    assert_eq!(xwidget.row_role, GlyphRowRole::TabLine);
    assert_eq!(xwidget.clip_rect, Some(row_bounds));
    assert_eq!(
        xwidget.slot_id,
        Some(neomacs_display_protocol::frame_glyphs::DisplaySlotId {
            window_id: 77,
            row: 0,
            col: 1,
        })
    );
    assert_eq!(xwidget.xwidget_id, 1234);
    assert_eq!(xwidget.x, 8.0);
    assert_eq!(xwidget.y, 4.0);
    assert_eq!(xwidget.width, 96.0);
    assert_eq!(xwidget.height, 54.0);
}

#[test]
fn measured_display_row_promotes_bounds_from_rendered_row_metrics() {
    let mut row = GlyphRow::new(GlyphRowRole::TabLine);
    row.enabled = true;
    row.height_px = 24.0;
    row.ascent_px = 20.0;
    let measured = MeasuredDisplayRow::new(
        DisplayRowOwner::WindowChrome {
            window_id: 77,
            kind: WindowChromeKind::TabLine,
        },
        0,
        Rect::new(10.0, 6.0, 120.0, 17.0),
        RenderedDisplayRow {
            row,
            progress: DisplayRowOutputProgress {
                end_x: 24.0,
                end_col: 3,
                y: 6.0,
                height: 24.0,
            },
            source_slots: Vec::new(),
            faces: Vec::new(),
            media: Vec::new(),
        },
        DisplayRowBoundsPolicy::PreserveAllocatedMinimum,
    );

    assert_eq!(measured.bounds.height, 24.0);
    assert_eq!(measured.row_height(), 24.0);
    assert_eq!(measured.row_ascent(), 20.0);
}

#[test]
fn measured_display_row_content_policy_ignores_allocated_row_height() {
    let mut row = GlyphRow::new(GlyphRowRole::TabBar);
    row.enabled = true;
    row.height_px = 120.0;
    row.ascent_px = 13.0;
    let mut face = neomacs_display_protocol::face::Face::default();
    face.id = 8;
    face.font_ascent = 13;
    face.font_descent = 4;
    let measured = MeasuredDisplayRow::new(
        DisplayRowOwner::FrameChrome {
            kind: FrameChromeKind::TabBar,
        },
        0,
        Rect::new(0.0, 0.0, 640.0, 120.0),
        RenderedDisplayRow {
            row,
            progress: DisplayRowOutputProgress {
                end_x: 24.0,
                end_col: 1,
                y: 0.0,
                height: 120.0,
            },
            source_slots: Vec::new(),
            faces: vec![face],
            media: vec![RenderedDisplayRowMedia {
                kind: RenderedDisplayRowMediaKind::Image { image_id: 77 },
                x: 0.0,
                y: 0.0,
                col: 0,
                width: 32.0,
                height: 24.0,
            }],
        },
        DisplayRowBoundsPolicy::MeasureContent,
    );

    assert_eq!(measured.bounds.height, 24.0);
    assert_eq!(measured.row_height(), 24.0);
}
