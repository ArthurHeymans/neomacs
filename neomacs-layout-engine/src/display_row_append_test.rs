use super::*;
use crate::display_item::{
    DisplayImageItem, DisplayItemKind, DisplayLength, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, DisplayVideoItem, DisplayXwidgetItem, RenderFaceRef,
};
use crate::display_row::{
    DisplayRowGeometry, DisplayRowRenderBounds, DisplayRowRenderer, DisplayRowSourceState,
    DisplayRowSpec,
};
use crate::display_row_builder::{DisplayRowPosition, DisplayTabPolicy, FixedGlyphAdvances};
use crate::neovm_bridge::LayoutBufferSnapshot;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::GlyphType;
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, LispCharPos1};
use neovm_core::emacs_core::eval::{
    DisplayHost, GuiFrameHostRequest, ImageResolveRequest, ResolvedImage,
};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::emacs_core::{Context, Value};
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
fn render_face_ref_id_uses_fallback_for_inherit() {
    assert_eq!(render_face_ref_id(RenderFaceRef::FaceId(12), 7), 12);
    assert_eq!(render_face_ref_id(RenderFaceRef::Inherit, 7), 7);
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
    let mut next_face_id = 8;
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
            .render_display_item_source_row_fragment_step_with_display_host(
                DisplayRowSpec {
                    geometry: DisplayRowGeometry {
                        y: 0.0,
                        width: 160.0,
                        height: 16.0,
                        char_width: 8.0,
                        ascent: 12.0,
                        tab_policy: DisplayTabPolicy::every(8),
                    },
                    render_bounds: DisplayRowRenderBounds {
                        start: DisplayRowPosition { x_px: 16.0, col: 2 },
                        max_x_px: 160.0,
                    },
                    base_face_id: 7,
                    base_face: &base_face,
                    role: GlyphRowRole::Text,
                    symbol_values: std::collections::HashMap::new(),
                },
                &mut source,
                &mut source_state,
                &face_resolver,
                None,
                &mut next_face_id,
            )
            .expect("rendered fragment")
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
fn display_row_append_surface_builds_positioned_specs() {
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

    let spec = surface
        .frame(
            DisplayRowAppendPlacement {
                row: 3,
                y: 20.0,
                glyph_y: 22.0,
            },
            DisplayRowAppendMetrics {
                height: 16.0,
                ascent: 11.0,
                char_width: 9.0,
                space_width: 7.0,
                default_row_height: 14.0,
            },
        )
        .at(DisplayRowPosition { x_px: 18.0, col: 2 }, 42)
        .append_spec(DisplayRowAppendKind::SourceText);

    assert_eq!(spec.position, DisplayRowPosition { x_px: 18.0, col: 2 });
    assert_eq!(spec.max_x, 128.0);
    assert_eq!(spec.layout.role, GlyphRowRole::Text);
    assert_eq!(spec.layout.base_face, RenderFaceRef::FaceId(42));
    assert_eq!(
        spec.layout.tab_policy,
        DisplayRowGeometry {
            y: 20.0,
            width: 120.0,
            height: 16.0,
            char_width: 9.0,
            ascent: 11.0,
            tab_policy,
        }
        .tab_policy
    );
    assert_eq!(spec.output.row, 3);
    assert_eq!(spec.output.row_y, 20.0);
    assert_eq!(spec.output.glyph_y, 22.0);
    assert_eq!(spec.output.height, 16.0);
}

#[test]
fn display_row_append_context_derives_layout_output_and_bounds() {
    let tab_policy = DisplayTabPolicy::from_tab_width_and_stops(8.0, 4, &[6, 10]);
    let context = DisplayRowAppendContext {
        row: 3,
        glyph_y: 22.0,
        x: 8.0,
        col: 0,
        geometry: DisplayRowGeometry {
            y: 20.0,
            width: 120.0,
            height: 16.0,
            char_width: 9.0,
            ascent: 11.0,
            tab_policy: tab_policy.clone(),
        },
        default_row_height: 14.0,
        content_x: 8.0,
        text_width: 150.0,
        line_number_width: 10.0,
        face_space_width: 7.0,
        face_id: 42,
    };

    let ordinary: DisplayRowAppendSpec = context.append_spec(DisplayRowAppendKind::SourceText);
    assert_eq!(ordinary.position, DisplayRowPosition { x_px: 8.0, col: 0 });
    assert_eq!(ordinary.max_x, 128.0);
    assert_eq!(ordinary.layout.char_width_px, 9.0);
    assert_eq!(ordinary.output.row, 3);
    assert_eq!(ordinary.output.row_y, 20.0);
    assert_eq!(ordinary.output.glyph_y, 22.0);
    assert_eq!(ordinary.output.height, 16.0);

    let tab = context.append_spec(DisplayRowAppendKind::Tab);
    assert_eq!(tab.max_x, f32::INFINITY);
    assert_eq!(tab.layout.char_width_px, 7.0);
    assert_eq!(tab.output.height, 14.0);

    let control = context.append_spec(DisplayRowAppendKind::ControlChar);
    assert_eq!(control.max_x, 148.0);
    assert_eq!(control.layout.char_width_px, 9.0);
    assert_eq!(control.output.height, 14.0);

    let mapped = context.append_spec(DisplayRowAppendKind::SourceMappedText);
    assert_eq!(mapped.max_x, 128.0);
    assert_eq!(mapped.output.height, 14.0);

    let glyphless = context.append_spec(DisplayRowAppendKind::Glyphless);
    assert_eq!(glyphless.max_x, 128.0);
    assert_eq!(glyphless.output.height, 16.0);

    let replacement = context.append_spec(DisplayRowAppendKind::DisplayReplacement);
    assert_eq!(replacement.max_x, 128.0);
    assert_eq!(replacement.layout.char_width_px, 9.0);
    assert_eq!(replacement.output.height, 16.0);

    let replacement_string = context.append_spec(DisplayRowAppendKind::DisplayReplacementString);
    assert_eq!(replacement_string.max_x, 128.0);
    assert_eq!(replacement_string.layout.char_width_px, 7.0);
    assert_eq!(replacement_string.output.height, 16.0);
}

#[test]
fn display_row_append_frame_builds_positioned_context() {
    let tab_policy = DisplayTabPolicy::every(4);
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 3,
            y: 20.0,
            glyph_y: 22.0,
        },
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

    let spec = frame
        .at(DisplayRowPosition { x_px: 18.0, col: 2 }, 42)
        .append_spec(DisplayRowAppendKind::SourceText);

    assert_eq!(spec.position, DisplayRowPosition { x_px: 18.0, col: 2 });
    assert_eq!(spec.max_x, 128.0);
    assert_eq!(spec.layout.base_face, RenderFaceRef::FaceId(42));
    assert_eq!(spec.output.row, 3);
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

    let frame = surface.frame(
        DisplayRowAppendPlacement {
            row: 3,
            y: 20.0,
            glyph_y: 22.0,
        },
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
fn display_row_append_frame_from_parts_preserves_geometry_and_area() {
    let tab_policy = DisplayTabPolicy::every(4);
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 3,
            y: 20.0,
            glyph_y: 22.0,
        },
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
    let mut current_face_id = 20;
    let mut pending_faces = Vec::new();
    let params = crate::display_source_resolver::DisplaySourceResolveParams {
        face_resolver: &face_resolver,
        display_host: None,
        base_face,
        canonical_face: face_resolver.default_face(),
        base_face_id: 0,
        fallback_char_width: 8.0,
        fallback_ascent: 12.0,
        fallback_row_height: 16.0,
    };
    let mut resolver = crate::display_source_resolver::DisplaySourcePropertyResolver::new(
        params,
        &mut resolve_state,
        &mut current_face_id,
        &mut pending_faces,
    );
    let face_value = Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]);

    let face = crate::display_source::DisplayItemFaceResolver::resolve_face_ref(
        &mut resolver,
        RenderFaceRef::FaceId(0),
        face_value,
    );

    assert_eq!(face, RenderFaceRef::FaceId(20));
    assert_eq!(current_face_id, 21);
    assert_eq!(pending_faces.len(), 1);
    assert_eq!(pending_faces[0].face_id, 20);
    assert_eq!(pending_faces[0].resolved.fg, 0x00ff0000);
}

#[test]
fn resolve_next_display_source_item_returns_item_and_pending_faces() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut current_face_id = 20;
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
        crate::display_source_resolver::DisplaySourceResolveParams {
            face_resolver: &face_resolver,
            display_host: None,
            base_face,
            canonical_face: face_resolver.default_face(),
            base_face_id: 0,
            fallback_char_width: 8.0,
            fallback_ascent: 12.0,
            fallback_row_height: 16.0,
        },
        &mut resolve_state,
        &mut current_face_id,
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
    let mut current_face_id = 20;
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
        crate::display_source_resolver::DisplaySourceResolveParams {
            face_resolver: &face_resolver,
            display_host: None,
            base_face,
            canonical_face: face_resolver.default_face(),
            base_face_id: 0,
            fallback_char_width: 8.0,
            fallback_ascent: 12.0,
            fallback_row_height: 16.0,
        },
        &mut resolve_state,
        &mut current_face_id,
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
    let mut current_face_id = 20;
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
                    &mut current_face_id,
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
    assert_eq!(current_face_id, 21);
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
    let mut current_face_id = 20;
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
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
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
        0,
        &mut current_face_id,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    );

    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
    assert_eq!(current_face_id, 21);
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
fn append_buffer_text_char_to_text_row_appends_source_char() {
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
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("append-buffer-char", 320, 120, buf_id);
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
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
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

    let (_progress, end) = append_buffer_text_char_to_text_row(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        buf_id,
        &snapshot,
        CharPos0::new(0),
        7,
        'a',
        8.0,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    )
    .expect("appended buffer char");

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
fn append_buffer_text_item_to_text_row_and_emit_builds_buffer_source_item() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("append-buffer-text-item", 320, 120, buf_id);
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
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
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
    let source = crate::display_source::BufferTextItemSource::new(
        buf_id,
        CharPos0::new(0),
        EmacsBytePos::new(0),
        CharPos0::new(1),
        EmacsBytePos::new(1),
    );

    let (_progress, end) = append_buffer_text_item_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        source,
        7,
        DisplayItemKind::ControlChar { ch: '\u{0001}' },
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
    )
    .expect("appended buffer text item");

    assert_eq!(end, DisplayRowPosition { x_px: 16.0, col: 2 });
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

struct TextUntilRowBreakPolicy;

impl DisplayRowSourceAppendPolicy for TextUntilRowBreakPolicy {
    fn decision_for(
        &mut self,
        item: &crate::display_item::DisplayItem,
    ) -> DisplayRowSourceAppendDecision {
        if matches!(item.kind, DisplayItemKind::RowBreak(_)) {
            return DisplayRowSourceAppendDecision::Stop;
        }
        let Some(kind) = DisplayRowAppendKind::from_display_item_kind(&item.kind) else {
            return DisplayRowSourceAppendDecision::Skip;
        };
        DisplayRowSourceAppendDecision::Append {
            kind,
            on_clipped: DisplayRowAppendClipBehavior::Stop,
        }
    }
}

#[test]
fn append_display_item_source_to_text_row_uses_policy_decisions() {
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
    let mut current_face_id = 20;
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let source = crate::display_source::LispStringSourceCursor::new(
        1,
        Value::string("a\nb"),
        RenderFaceRef::FaceId(7),
    )
    .expect("string source");
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
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
    let mut policy = TextUntilRowBreakPolicy;

    let end = append_display_item_source_to_text_row(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        source,
        &face_resolver,
        base_face,
        7,
        &mut current_face_id,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        &mut policy,
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
    let mut current_face_id = 20;
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
                    Value::string("/tmp/append-lisp-string.png"),
                ]),
            ]),
        }],
    );
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 6.0,
        },
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
        &mut current_face_id,
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
    assert_eq!(image.y, 6.0);
    assert_eq!(image.width, 64.0);
    assert_eq!(image.height, 32.0);
    let requests = requests.lock().expect("image requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].fg_color, 0x00112233);
    assert_eq!(requests[0].bg_color, 0x00445566);
}

struct SourceMappedTextWidthByFace {
    scratch: FixedGlyphAdvances,
}

impl SourceMappedTextWidthByFace {
    fn new() -> Self {
        Self {
            scratch: FixedGlyphAdvances::new(),
        }
    }
}

impl DisplayRowItemMeasurer for SourceMappedTextWidthByFace {
    fn measurement_for<'a>(
        &'a mut self,
        item: &crate::display_item::DisplayItem,
        face_id: u32,
    ) -> DisplayRowItemMeasurement<'a> {
        self.scratch = FixedGlyphAdvances::new();
        let DisplayItemKind::SourceMappedText(text) = &item.kind else {
            return DisplayRowItemMeasurement::Default;
        };
        for ch in text.text.chars() {
            self.scratch
                .insert(ch, face_id, if face_id == 20 { 13.0 } else { 11.0 });
        }
        DisplayRowItemMeasurement::Measured(&mut self.scratch)
    }
}

#[test]
fn append_display_replacement_string_source_to_text_row_walks_source_faces_and_measurements() {
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
    let mut current_face_id = 20;
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
    let string_source =
        crate::display_source::LispStringSourceCursor::new(1, value, RenderFaceRef::FaceId(7))
            .expect("string source");
    let replacement_source =
        crate::display_source::BufferDisplayReplacementSource::new(buf_id, 0, 0);
    let source = crate::display_source::BufferDisplayReplacementStringSource::new(
        replacement_source,
        string_source,
    );
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
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
    let mut measurer = SourceMappedTextWidthByFace::new();

    let end = append_display_replacement_string_source_to_text_row(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        source,
        &face_resolver,
        base_face,
        7,
        &mut current_face_id,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
        &mut measurer,
    );

    assert_eq!(end, DisplayRowPosition { x_px: 24.0, col: 2 });
    assert_eq!(current_face_id, 21);
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
fn append_display_replacement_item_to_text_row_uses_face_fallback_without_emitting() {
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
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
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::synthetic(9, 0, 1),
        RenderFaceRef::Inherit,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(13.0)),
            height: Some(DisplayLength::Pixels(16.0)),
            ascent: Some(DisplayLength::Pixels(12.0)),
        }),
    );

    let (progress, end) = append_display_replacement_item_to_text_row(
        &mut builder,
        item,
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
fn append_display_replacement_item_to_text_row_and_emit_advances_output() {
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
    let base_face = face_resolver.default_face();

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
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
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::synthetic(9, 0, 1),
        RenderFaceRef::FaceId(3),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(13.0)),
            height: Some(DisplayLength::Pixels(16.0)),
            ascent: Some(DisplayLength::Pixels(12.0)),
        }),
    );

    let (_progress, end) = append_display_replacement_item_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        item,
        &face_resolver,
        base_face,
        7,
        frame,
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
fn append_display_item_to_text_row_and_emit_infers_kind_and_face_fallback() {
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

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 0.0,
            glyph_y: 0.0,
        },
        DisplayRowAppendArea {
            content_x: 0.0,
            width: 80.0,
            text_width: 80.0,
            line_number_width: 0.0,
        },
        DisplayRowAppendMetrics {
            height: 16.0,
            ascent: 12.0,
            char_width: 8.0,
            space_width: 8.0,
            default_row_height: 10.0,
        },
        DisplayTabPolicy::every(8),
    );
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::new(
            DisplaySourcePosition::buffer(buf_id, CharPos0::new(0), EmacsBytePos::new(0)),
            DisplaySourcePosition::buffer(buf_id, CharPos0::new(1), EmacsBytePos::new(1)),
        ),
        RenderFaceRef::Inherit,
        DisplayItemKind::SourceMappedText(crate::display_item::DisplaySourceMappedText::new("x")),
    );

    let (_progress, end) = append_display_item_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        item,
        7,
        frame,
        DisplayRowPosition { x_px: 0.0, col: 0 },
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
    assert_eq!(
        output_emitter
            .point_for_lisp_buffer_pos(neovm_core::buffer::LispCharPos1::ONE)
            .expect("display point")
            .height,
        10
    );
}

#[test]
fn append_display_item_to_text_row_and_emit_installs_xwidget_replacements() {
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
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 4.0,
            glyph_y: 6.0,
        },
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
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::synthetic(9, 0, 1),
        RenderFaceRef::FaceId(3),
        DisplayItemKind::Xwidget(DisplayXwidgetItem {
            xwidget_id: 1234,
            width: 96.0,
            height: 54.0,
        }),
    );

    let (progress, end) = append_display_item_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        item,
        7,
        frame,
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
    assert_eq!(xwidget.y, 6.0);
    assert_eq!(xwidget.width, 96.0);
    assert_eq!(xwidget.height, 54.0);
}

#[test]
fn append_display_item_to_text_row_and_emit_installs_image_replacements() {
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
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 4.0,
            glyph_y: 6.0,
        },
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
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::synthetic(9, 0, 1),
        RenderFaceRef::FaceId(3),
        DisplayItemKind::Image(DisplayImageItem {
            image_id: 42,
            width: 64.0,
            height: 32.0,
        }),
    );

    let (progress, end) = append_display_item_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        item,
        7,
        frame,
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
    assert_eq!(image.y, 6.0);
    assert_eq!(image.width, 64.0);
    assert_eq!(image.height, 32.0);
}

#[test]
fn append_display_item_to_text_row_and_emit_installs_video_replacements() {
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
    let frame = DisplayRowAppendFrame::from_parts(
        DisplayRowAppendPlacement {
            row: 0,
            y: 4.0,
            glyph_y: 6.0,
        },
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
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::synthetic(9, 0, 1),
        RenderFaceRef::FaceId(3),
        DisplayItemKind::Video(DisplayVideoItem {
            video_id: 88,
            width: 80.0,
            height: 45.0,
            loop_count: -1,
            autoplay: true,
        }),
    );

    let (progress, end) = append_display_item_to_text_row_and_emit(
        &mut builder,
        &mut output_emitter,
        &mut eval,
        item,
        7,
        frame,
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
    assert_eq!(video.y, 6.0);
    assert_eq!(video.width, 80.0);
    assert_eq!(video.height, 45.0);
    assert_eq!(video.loop_count, -1);
    assert!(video.autoplay);
}

#[test]
fn display_row_append_spec_appends_item_to_matrix_row() {
    let context = DisplayRowAppendContext {
        row: 0,
        glyph_y: 0.0,
        x: 0.0,
        col: 0,
        geometry: DisplayRowGeometry {
            y: 0.0,
            width: 80.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: DisplayTabPolicy::every(8),
        },
        default_row_height: 16.0,
        content_x: 0.0,
        text_width: 80.0,
        line_number_width: 0.0,
        face_space_width: 8.0,
        face_id: 7,
    };
    let spec = context.append_spec(DisplayRowAppendKind::SourceText);
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let item = crate::display_item::DisplayItem::new(
        crate::display_item::SourceSpan::new(
            crate::display_item::DisplaySourcePosition::buffer(
                BufferId(1),
                CharPos0::new(0),
                EmacsBytePos::new(0),
            ),
            crate::display_item::DisplaySourcePosition::buffer(
                BufferId(1),
                CharPos0::new(1),
                EmacsBytePos::new(1),
            ),
        ),
        RenderFaceRef::FaceId(7),
        crate::display_item::DisplayItemKind::TextRun(crate::display_item::DisplayTextRun::new(
            "a",
        )),
    );

    let (progress, position) =
        append_display_row_spec_item(&mut builder, &spec, item).expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 0.0, col: 0 });
    assert_eq!(progress.end, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(position, DisplayRowPosition { x_px: 8.0, col: 1 });
    builder
        .with_current_row_mut(|row| {
            assert_eq!(row.glyphs[1][0].face_id, 7);
        })
        .expect("current row");
}
