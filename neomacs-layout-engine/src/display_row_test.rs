use super::*;
use crate::neovm_bridge::{FaceResolver, LayoutBufferSnapshot, LayoutBufferView};
use neomacs_display_protocol::Rect;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{GlyphRow, GlyphType};
use neovm_core::buffer::{CharPos0, EmacsByteRange};
use neovm_core::emacs_core::eval::{
    DisplayHost, GuiFrameHostRequest, ImageResolveRequest, ResolvedImage, ResolvedVideo,
    ResolvedWebKit, VideoResolveRequest, WebKitResolveRequest,
};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;
use std::sync::Mutex;

fn base_face() -> crate::neovm_bridge::ResolvedFace {
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    resolver.default_face().clone()
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
    assert_eq!(rendered.font_char_width, 1.0);
    assert_eq!(rendered.font_ascent, 1.0);
    assert_eq!(rendered.font_descent, 0);
}

#[test]
fn insert_resolved_display_row_face_applies_metric_overrides() {
    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    let face = base_face();

    insert_resolved_display_row_face(
        &mut builder,
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
fn display_row_renderer_renders_lisp_string_without_layout_engine() {
    let _eval = Context::new();
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let rendered = renderer
        .render_lisp_string_row(spec, Value::string("A中"), &resolver, &mut next_face_id)
        .expect("display source row");

    assert_eq!(row_text_expanding_stretches(&rendered.row), "A中");
    assert_eq!(rendered.row.role, GlyphRowRole::TabLine);
    assert_eq!(rendered.progress.end_col, 3);
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
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        resolver.default_face(),
        role,
        symbol_values,
    );
    renderer
        .render_lisp_string_row(spec, rendered, &resolver, &mut next_face_id)
        .expect("display source row")
        .row
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
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buf_id,
        &snapshot,
        CharPos0::ZERO,
        snapshot.layout_point_max_char_pos(),
        RenderFaceRef::FaceId(1),
    );
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        resolver.default_face(),
        role,
        std::collections::HashMap::new(),
    );

    renderer
        .render_display_item_source_row(spec, &mut source, &resolver, &mut next_face_id)
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
    let mut next_face_id = 1;
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buf_id,
        buffer,
        neovm_core::buffer::CharPos0::new(0),
        buffer.layout_point_max_char_pos(),
        RenderFaceRef::FaceId(1),
    );

    let rendered = renderer
        .render_display_item_source_row(
            DisplayRowSpec {
                geometry: DisplayRowGeometry {
                    y: 0.0,
                    width: 240.0,
                    height: 16.0,
                    char_width: 8.0,
                    ascent: 12.0,
                    tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
                },
                base_face_id: 1,
                base_face: resolver.default_face(),
                role: GlyphRowRole::TabLine,
                symbol_values: std::collections::HashMap::new(),
            },
            &mut source,
            &resolver,
            &mut next_face_id,
        )
        .expect("display source row");

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
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 4.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        &base_face,
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let rendered = renderer
        .render_lisp_string_row(spec, rendered_text, &resolver, &mut next_face_id)
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
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 4.0,
            width: 240.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        &base_face,
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );
    let rendered = renderer
        .render_lisp_string_row_with_display_host(
            spec,
            rendered_text,
            &resolver,
            Some(&host),
            &mut next_face_id,
        )
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
    let mut next_face_id = 1;
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buf_id,
        buffer,
        neovm_core::buffer::CharPos0::new(0),
        buffer.layout_point_max_char_pos(),
        RenderFaceRef::FaceId(1),
    );

    let rendered = renderer
        .render_display_item_source_row(
            DisplayRowSpec {
                geometry: DisplayRowGeometry {
                    y: 0.0,
                    width: 240.0,
                    height: 16.0,
                    char_width: 8.0,
                    ascent: 12.0,
                    tab_policy:
                        crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(
                            0.0,
                            4,
                            &[2],
                        ),
                },
                base_face_id: 1,
                base_face: resolver.default_face(),
                role: GlyphRowRole::TabLine,
                symbol_values: std::collections::HashMap::new(),
            },
            &mut source,
            &resolver,
            &mut next_face_id,
        )
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
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
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
        &mut next_face_id,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let rendered = engine
        .render_lisp_string_row(spec, Value::string("\tX"), &resolver, &mut next_face_id)
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
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 240.0,
            height: 32.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        &base_face,
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );

    let row = engine
        .render_lisp_string_row(spec, rendered, &resolver, &mut next_face_id)
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
fn display_row_baseline_tab_line_rtl_text_is_reordered_after_row_build() {
    let _eval = Context::new();
    let row = render_lisp_display_row(Value::string("אב"), GlyphRowRole::TabLine);

    assert_eq!(row.role, GlyphRowRole::TabLine);
    assert!(
        row.reversed_p,
        "pure RTL chrome row should be marked reversed"
    );
    assert_eq!(row_text_expanding_stretches(&row), "בא");
}

#[test]
fn install_rendered_display_row_preserves_prebuilt_bidi_metadata() {
    let _eval = Context::new();
    let mut engine = crate::engine::LayoutEngine::new();
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00FFFFFF, 0x00000000, 14.0, None);
    let mut next_face_id = 1;
    let spec = DisplayRowSpec::from_base_face(
        DisplayRowGeometry {
            y: 0.0,
            width: 80.0,
            height: 16.0,
            char_width: 8.0,
            ascent: 12.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        &mut next_face_id,
        resolver.default_face(),
        GlyphRowRole::TabLine,
        std::collections::HashMap::new(),
    );
    let rendered = engine
        .render_lisp_string_row(spec, Value::string("אב"), &resolver, &mut next_face_id)
        .expect("display source row");

    assert!(rendered.row.reversed_p);
    assert_eq!(row_text_expanding_stretches(&rendered.row), "בא");

    let mut builder = crate::matrix_builder::GlyphMatrixBuilder::new();
    builder.begin_window(1, 1, 10, Rect::new(0.0, 0.0, 80.0, 16.0), true);
    install_rendered_display_row(&mut builder, &rendered, 0);
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let row = &state.window_matrices[0].matrix.rows[0];
    assert!(row.reversed_p);
    assert_eq!(row_text_expanding_stretches(row), "בא");
}

#[test]
fn install_rendered_display_row_installs_media_fragments_in_current_window() {
    let mut row = GlyphRow::new(GlyphRowRole::TabLine);
    row.enabled = true;
    row.height_px = 16.0;
    row.ascent_px = 12.0;
    let rendered = RenderedDisplayRow {
        row,
        progress: DisplayRowOutputProgress {
            end_x: 0.0,
            end_col: 0,
            y: 4.0,
            height: 16.0,
        },
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
    let text_bounds = Rect::new(10.0, 20.0, 160.0, 64.0);
    builder.begin_window_with_text_bounds(
        77,
        1,
        10,
        Rect::new(0.0, 0.0, 200.0, 80.0),
        text_bounds,
        true,
    );

    install_rendered_display_row(&mut builder, &rendered, 0);
    builder.end_window();

    let state = builder.finish(10, 1, 8.0, 16.0);
    let xwidget = state.xwidgets.first().expect("xwidget side item");
    assert_eq!(xwidget.window_id, 77);
    assert_eq!(xwidget.row_role, GlyphRowRole::TabLine);
    assert_eq!(xwidget.clip_rect, Some(text_bounds));
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
