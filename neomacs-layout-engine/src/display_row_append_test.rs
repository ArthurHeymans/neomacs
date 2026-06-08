use super::*;
use crate::display_item::RenderFaceRef;
use crate::display_row::DisplayRowGeometry;
use crate::display_row_builder::{DisplayRowPosition, DisplayTabPolicy};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::types::Rect;
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos};

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
