use super::*;
use crate::display_item::{DisplayItemKind, DisplaySourcePosition, RenderFaceRef};
use crate::display_row::DisplayRowGeometry;
use crate::display_row_builder::{DisplayRowPosition, DisplayTabPolicy};
use crate::neovm_bridge::LayoutBufferSnapshot;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::types::{Color, Rect};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::emacs_core::{Context, Value};

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
fn layout_string_face_resolver_records_pending_faces_without_builder() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut string_face_cache = std::collections::HashMap::new();
    let mut current_face_id = 20;
    let mut pending_faces = Vec::new();
    let mut resolver = LayoutStringFaceResolver {
        face_resolver: &face_resolver,
        base_face,
        string_face_cache: &mut string_face_cache,
        current_face_id: &mut current_face_id,
        pending_faces: &mut pending_faces,
    };
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
fn next_layout_string_source_item_installs_pending_faces() {
    let _eval = Context::new();
    let table = neovm_core::face::FaceTable::new();
    let face_resolver =
        crate::neovm_bridge::FaceResolver::new(&table, 0x00ffffff, 0x000000, 14.0, None);
    let base_face = face_resolver.default_face();
    let mut current_face_id = 20;
    let mut string_face_cache = std::collections::HashMap::new();
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 20, Rect::new(0.0, 0.0, 160.0, 16.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
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
    let row_layout = DisplayRowGeometry {
        y: 0.0,
        width: 80.0,
        height: 16.0,
        char_width: 8.0,
        ascent: 12.0,
        tab_policy: DisplayTabPolicy::every(8),
    }
    .to_layout(
        GlyphRowRole::Text,
        8.0,
        12.0,
        RenderFaceRef::FaceId(0),
        std::collections::HashMap::new(),
    );
    let mut append_cursor = crate::display_row_builder::DisplayRowAppendCursor::new(
        DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    );

    let item = next_layout_string_source_item(
        &mut builder,
        &mut source,
        &face_resolver,
        base_face,
        &mut string_face_cache,
        &mut current_face_id,
    )
    .expect("source item");

    assert_eq!(item.face, RenderFaceRef::FaceId(20));
    assert_eq!(
        builder.faces().get(&20).map(|face| face.foreground),
        Some(Color::from_pixel(0x00ff0000))
    );

    let progress = append_cursor
        .append_item_to_current_matrix_row(&mut builder, &row_layout, item)
        .expect("append progress");

    assert_eq!(progress.end.x_px, 8.0);
    assert_eq!(append_cursor.position().col, 1);
    builder
        .with_current_row_mut(|row| {
            assert_eq!(row.glyphs[1][0].face_id, 20);
        })
        .expect("current row");
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
