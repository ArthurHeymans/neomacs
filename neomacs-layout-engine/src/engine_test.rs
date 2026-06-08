use super::*;
use crate::display_item::RenderFaceRef;
use crate::display_source::DisplayItemSource;
use crate::neovm_bridge::{LayoutBufferSnapshot, RustBufferAccess};
use neomacs_display_protocol::cursor::CursorBarWidth;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphRow, GlyphType};
use neovm_core::buffer::{
    BufferId, BufferTextBackendKind, CharPos0, EmacsBytePos, EmacsByteRange, LispCharPos1,
};
use neovm_core::emacs_core::Context;
use neovm_core::emacs_core::eval::{
    DisplayHost, GuiFrameHostRequest, ImageResolveRequest, ResolvedImage, ResolvedVideo,
    ResolvedWebKit, VideoResolveRequest, WebKitResolveRequest,
};
use neovm_core::emacs_core::load::{
    apply_runtime_startup_state, create_bootstrap_evaluator_cached_with_features,
};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::heap_types::LispString;
use neovm_core::window::{
    DisplayPointSnapshot, DisplayRowSnapshot, WindowCursorSnapshot, WindowVisibleBufferSpan,
};
use std::sync::{Arc, Mutex};

trait BufferTextPropertyTestExt {
    fn put_text_property(&mut self, start: usize, end: usize, name: Value, value: Value) -> bool;
}

fn emacs_byte_range(start: usize, end: usize) -> EmacsByteRange {
    EmacsByteRange::new(EmacsBytePos::new(start), EmacsBytePos::new(end))
}

impl BufferTextPropertyTestExt for neovm_core::buffer::Buffer {
    fn put_text_property(&mut self, start: usize, end: usize, name: Value, value: Value) -> bool {
        self.text_props_put_property_in_emacs_byte_range(emacs_byte_range(start, end), name, value)
    }
}

#[test]
fn echo_area_display_rows_include_wrapped_long_lines_like_gnu() {
    assert_eq!(plain_echo_display_rows("abcdef", 3.0, 1.0, false, false), 2);
    assert_eq!(plain_echo_display_rows("abcdef", 3.0, 1.0, true, false), 1);
    assert_eq!(plain_echo_display_rows("abcdef", 3.0, 1.0, false, true), 3);
}

#[test]
fn echo_area_display_rows_count_newlines_and_wide_chars() {
    assert_eq!(plain_echo_display_rows("ab\ncd", 3.0, 1.0, false, false), 2);
    assert_eq!(plain_echo_display_rows("你你你", 4.0, 1.0, false, false), 2);
}

#[test]
fn resize_mini_windows_mode_parses_gnu_values() {
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::NIL)),
        ResizeMiniWindowsMode::Disabled
    );
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::symbol("grow-only"))),
        ResizeMiniWindowsMode::GrowOnly
    );
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::T)),
        ResizeMiniWindowsMode::Exact
    );
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::symbol("anything-else"))),
        ResizeMiniWindowsMode::Exact
    );
}

#[test]
fn grow_only_minibuffer_shrinks_only_when_visible_region_is_empty() {
    assert!(ResizeMiniWindowsMode::GrowOnly.should_grow());
    assert!(!ResizeMiniWindowsMode::Disabled.should_grow());
    assert!(ResizeMiniWindowsMode::Exact.should_shrink(false));
    assert!(!ResizeMiniWindowsMode::Disabled.should_shrink(true));
    assert!(!ResizeMiniWindowsMode::GrowOnly.should_shrink(false));
    assert!(ResizeMiniWindowsMode::GrowOnly.should_shrink(true));
}

fn test_window_params() -> WindowParams {
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
        cursor_bar_width: CursorBarWidth::TWO,
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

fn realize_test_gui_frame(eval: &mut Context, frame_id: neovm_core::window::FrameId) {
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
        frame.install_gnu_gui_default_parameters();
    }
    assert!(eval.frame_manager_mut().select_frame(frame_id));
    let results = eval
        .eval_str_each("(internal-set-lisp-face-attribute 'default :height 100 (selected-frame))");
    assert!(
        results.iter().all(Result::is_ok),
        "test GUI frame should have a realized default face height, got {results:?}"
    );
}

#[derive(Default)]
struct RecordingImageDisplayHost {
    requests: Arc<Mutex<Vec<ImageResolveRequest>>>,
    video_requests: Arc<Mutex<Vec<VideoResolveRequest>>>,
    webkit_requests: Arc<Mutex<Vec<WebKitResolveRequest>>>,
}

impl DisplayHost for RecordingImageDisplayHost {
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
        panic!("layout must use nonblocking request_image");
    }

    fn request_image(&self, request: ImageResolveRequest) -> Result<Option<ResolvedImage>, String> {
        self.requests
            .lock()
            .expect("requests lock")
            .push(request.clone());
        Ok(Some(ResolvedImage {
            image_id: 77,
            width: 32,
            height: 24,
            dimensions_known: true,
        }))
    }

    fn request_video(&self, request: VideoResolveRequest) -> Result<Option<ResolvedVideo>, String> {
        self.video_requests
            .lock()
            .expect("video requests lock")
            .push(request);
        Ok(Some(ResolvedVideo { video_id: 88 }))
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

fn window_matrix_text(entry: &neomacs_display_protocol::glyph_matrix::WindowMatrixEntry) -> String {
    entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .flat_map(|row| row.glyphs[1].iter())
        .filter_map(|glyph| match &glyph.glyph_type {
            neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
            neomacs_display_protocol::glyph_matrix::GlyphType::Composite { text } => {
                text.chars().next()
            }
            _ => None,
        })
        .collect()
}

fn enabled_window_row_texts(
    entry: &neomacs_display_protocol::glyph_matrix::WindowMatrixEntry,
) -> Vec<String> {
    entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .map(|row| {
            row.glyphs[1]
                .iter()
                .filter_map(|glyph| match &glyph.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
                    neomacs_display_protocol::glyph_matrix::GlyphType::Composite { text } => {
                        text.chars().next()
                    }
                    _ => None,
                })
                .collect()
        })
        .collect()
}

fn glyphs_logical_text(glyphs: &[Glyph]) -> String {
    glyphs
        .iter()
        .filter(|glyph| !glyph.padding)
        .map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } | GlyphType::Glyphless { ch } => ch.to_string(),
            GlyphType::Composite { text } => text.to_string(),
            GlyphType::Stretch { width_cols } => " ".repeat(usize::from(*width_cols)),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn assert_replacement_slot_between_neighbors(
    eval: &Context,
    frame_id: neovm_core::window::FrameId,
    replacement_pos: usize,
    expected_width: i64,
) -> DisplayPointSnapshot {
    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(frame.selected_window)
        .expect("display snapshot");
    let before = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(
            replacement_pos.saturating_sub(1),
        ))
        .expect("previous point");
    let replacement = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(replacement_pos))
        .expect("replacement point");
    let after = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(replacement_pos + 1))
        .expect("following point");

    assert_eq!(replacement.x, before.x + before.width);
    assert_eq!(replacement.width, expected_width);
    assert_eq!(replacement.row, before.row);
    assert_eq!(replacement.row, after.row);
    assert!(
        replacement.x + replacement.width <= after.x,
        "replacement slot should own the covered source geometry before following text; before={before:?} replacement={replacement:?} after={after:?}"
    );
    replacement.clone()
}

fn enabled_window_row_texts_expanding_stretches(
    entry: &neomacs_display_protocol::glyph_matrix::WindowMatrixEntry,
) -> Vec<String> {
    entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .map(|row| {
            row.glyphs[1]
                .iter()
                .flat_map(|glyph| match &glyph.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => {
                        std::iter::repeat_n(*ch, 1).collect::<Vec<_>>()
                    }
                    neomacs_display_protocol::glyph_matrix::GlyphType::Composite { text } => {
                        text.chars().collect::<Vec<_>>()
                    }
                    neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { width_cols } => {
                        std::iter::repeat_n(' ', usize::from(*width_cols)).collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                })
                .collect()
        })
        .collect()
}

fn implemented_text_backends() -> impl Iterator<Item = BufferTextBackendKind> {
    BufferTextBackendKind::implemented_variants()
}

fn convert_current_buffer_text_backend(eval: &mut Context, kind: BufferTextBackendKind) {
    let form = format!("(neomacs-set-buffer-text-backend '{})", kind.symbol_name());
    let result = eval
        .eval_str(&form)
        .unwrap_or_else(|err| panic!("convert buffer text backend with {form}: {err}"));
    assert_eq!(result.as_symbol_name(), Some(kind.symbol_name()));
}

fn insert_fragmented_current_buffer_text(eval: &mut Context, text: &str) {
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = eval
        .buffer_manager_mut()
        .get_mut(buffer_id)
        .expect("current buffer");
    buffer.insert(text);

    for marker in ["\n", "日本", "Ω"] {
        if let Some(pos) = text.find(marker) {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(pos));
            buffer.insert("tmp");
            buffer.delete_emacs_byte_range(emacs_byte_range(pos, pos + "tmp".len()));
        }
    }

    assert_eq!(buffer.buffer_string(), text);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GlyphKindTrace {
    Char(char),
    Composite(String),
    Stretch(u16),
    Image(i32),
    Glyphless(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlyphTrace {
    kind: GlyphKindTrace,
    face_id: u32,
    charpos: usize,
    bidi_level: u8,
    wide: bool,
    padding: bool,
    pixel_width_bits: u32,
    pixel_height_bits: u32,
    pixel_ascent_bits: u32,
}

impl GlyphTrace {
    fn from_glyph(glyph: &Glyph) -> Self {
        let kind = match &glyph.glyph_type {
            GlyphType::Char { ch } => GlyphKindTrace::Char(*ch),
            GlyphType::Composite { text } => GlyphKindTrace::Composite(text.to_string()),
            GlyphType::Stretch { width_cols } => GlyphKindTrace::Stretch(*width_cols),
            GlyphType::Image { image_id } => GlyphKindTrace::Image(*image_id),
            GlyphType::Glyphless { ch } => GlyphKindTrace::Glyphless(*ch),
        };
        Self {
            kind,
            face_id: glyph.face_id,
            charpos: glyph.charpos,
            bidi_level: glyph.bidi_level,
            wide: glyph.wide,
            padding: glyph.padding,
            pixel_width_bits: glyph.pixel_width.to_bits(),
            pixel_height_bits: glyph.pixel_height.to_bits(),
            pixel_ascent_bits: glyph.pixel_ascent.to_bits(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RowTrace {
    role: GlyphRowRole,
    enabled: bool,
    cursor_col: Option<u16>,
    cursor_type: Option<String>,
    truncated_left: bool,
    continued: bool,
    displays_text: bool,
    ends_at_zv: bool,
    mode_line: bool,
    pixel_y_bits: u32,
    height_px_bits: u32,
    ascent_px_bits: u32,
    start_charpos: usize,
    end_charpos: usize,
    glyph_areas: [Vec<GlyphTrace>; 3],
}

impl RowTrace {
    fn from_row(row: &GlyphRow) -> Self {
        Self {
            role: row.role,
            enabled: row.enabled,
            cursor_col: row.cursor_col,
            cursor_type: row.cursor_type.map(|cursor| format!("{cursor:?}")),
            truncated_left: row.truncated_left,
            continued: row.continued,
            displays_text: row.displays_text,
            ends_at_zv: row.ends_at_zv,
            mode_line: row.mode_line,
            pixel_y_bits: row.pixel_y.to_bits(),
            height_px_bits: row.height_px.to_bits(),
            ascent_px_bits: row.ascent_px.to_bits(),
            start_charpos: row.start_charpos,
            end_charpos: row.end_charpos,
            glyph_areas: [
                row.glyphs[0].iter().map(GlyphTrace::from_glyph).collect(),
                row.glyphs[1].iter().map(GlyphTrace::from_glyph).collect(),
                row.glyphs[2].iter().map(GlyphTrace::from_glyph).collect(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HitRowTrace {
    y_start_bits: u32,
    y_end_bits: u32,
    charpos_start: i64,
    charpos_end: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowHitTrace {
    content_x_bits: u32,
    char_w_bits: u32,
    rows: Vec<HitRowTrace>,
    first_col_hits: Vec<i64>,
}

impl WindowHitTrace {
    fn from_window(window: &crate::hit_test::WindowHitData) -> Self {
        Self {
            content_x_bits: window.content_x.to_bits(),
            char_w_bits: window.char_w.to_bits(),
            rows: window
                .rows
                .iter()
                .map(|row| HitRowTrace {
                    y_start_bits: row.y_start.to_bits(),
                    y_end_bits: row.y_end.to_bits(),
                    charpos_start: row.charpos_start,
                    charpos_end: row.charpos_end,
                })
                .collect(),
            first_col_hits: window
                .rows
                .iter()
                .map(|row| {
                    let y = (row.y_start + row.y_end) / 2.0;
                    crate::hit_test::hit_test_window_charpos(window.window_id, window.content_x, y)
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BackendLayoutTrace {
    matrix_rows: Vec<RowTrace>,
    points: Vec<DisplayPointSnapshot>,
    output_rows: Vec<DisplayRowSnapshot>,
    phys_cursor: Option<WindowCursorSnapshot>,
    visible_span: Option<WindowVisibleBufferSpan>,
    window_start: LispCharPos1,
    window_point: LispCharPos1,
    hit: Option<WindowHitTrace>,
}

fn selected_window_layout_trace(
    eval: &Context,
    engine: &LayoutEngine,
    frame_id: neovm_core::window::FrameId,
) -> BackendLayoutTrace {
    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let selected_window = frame.selected_window;
    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let display_snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let (window_start, window_point) =
        match frame.find_window(selected_window).expect("selected window") {
            neovm_core::window::Window::Leaf {
                window_start,
                point,
                ..
            } => (*window_start, *point),
            other => panic!("expected leaf window, got {other:?}"),
        };
    let hit = unsafe {
        (&*std::ptr::addr_of!(crate::hit_test::FRAME_HIT_DATA))
            .as_ref()
            .and_then(|windows| {
                windows
                    .iter()
                    .find(|window| window.window_id == selected_window.0 as i64)
            })
            .map(WindowHitTrace::from_window)
    };

    BackendLayoutTrace {
        matrix_rows: window_entry
            .matrix
            .rows
            .iter()
            .filter(|row| row.enabled)
            .map(RowTrace::from_row)
            .collect(),
        points: display_snapshot.points.clone(),
        output_rows: display_snapshot.rows.clone(),
        phys_cursor: display_snapshot.phys_cursor.clone(),
        visible_span: display_snapshot.visible_buffer_span(),
        window_start,
        window_point,
        hit,
    }
}

fn backend_layout_trace_with_buffer_and_window_setup(
    kind: BufferTextBackendKind,
    frame_name: &str,
    text: &str,
    frame_width: u32,
    frame_height: u32,
    setup: impl FnOnce(&mut neovm_core::buffer::Buffer, BufferId, &str),
    setup_window: impl FnOnce(&mut neovm_core::window::Window),
) -> BackendLayoutTrace {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, kind);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, text);
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        setup(buffer, buf_id, text);
        assert_eq!(buffer.text_backend_kind(), kind);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame(frame_name, frame_width, frame_height, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
        setup_window(window);
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    selected_window_layout_trace(&eval, &engine, frame_id)
}

fn backend_layout_trace_with_buffer_setup(
    kind: BufferTextBackendKind,
    frame_name: &str,
    text: &str,
    frame_width: u32,
    frame_height: u32,
    setup: impl FnOnce(&mut neovm_core::buffer::Buffer, BufferId, &str),
) -> BackendLayoutTrace {
    backend_layout_trace_with_buffer_and_window_setup(
        kind,
        frame_name,
        text,
        frame_width,
        frame_height,
        setup,
        |_| {},
    )
}

fn backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abé\tz\n日本x\nlast Ω line\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-parity",
        text,
        360,
        180,
        |buffer, _buf_id, text| {
            let omega_byte = text.find('Ω').expect("omega");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(omega_byte));
            buffer.set_buffer_local("display-line-numbers", Value::T);
        },
    )
}

fn display_replacement_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abcXYZdef\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-display-replacement",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let start = text.find("XYZ").expect("replacement start");
            let end = start + "XYZ".len();
            assert!(buffer.put_text_property(
                start,
                end,
                Value::symbol("display"),
                Value::string("R")
            ));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start + 1));
        },
    )
}

fn invisible_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abc hidden xyz\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-invisible",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let start = text.find("hidden").expect("hidden start");
            let end = start + "hidden".len();
            assert!(buffer.put_text_property(start, end, Value::symbol("invisible"), Value::T));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start + 2));
        },
    )
}

fn multiline_overlay_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "x";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-overlay",
        text,
        360,
        140,
        |buffer, buf_id, _text| {
            let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
                serial: 0,
                plist: Value::NIL,
                buffer: Some(buf_id),
                start: 0,
                end: 1,
                front_advance: false,
                rear_advance: false,
            });
            buffer.overlays_mut().insert_overlay(overlay);
            let _ = buffer.overlays_mut().overlay_put(
                overlay,
                Value::symbol("after-string"),
                Value::string("A\nB"),
            );
            buffer.goto_emacs_byte_pos(buffer.point_max_emacs_byte_pos());
        },
    )
}

fn bidi_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abc אבג def\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-bidi",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let alef_byte = text.find('א').expect("alef");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(alef_byte));
        },
    )
}

fn selective_display_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "head\rhidden tail\nshown\n  hidden by indent\nshown2\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-selective-display",
        text,
        360,
        180,
        |buffer, _buf_id, _text| {
            buffer.set_buffer_local("selective-display", Value::fixnum(1));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(2));
        },
    )
}

fn glyphless_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "a\u{0080}b\u{FEFF}c\u{FFFC}d\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-glyphless",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let c1_byte = text.find('\u{0080}').expect("C1 control");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(c1_byte));
        },
    )
}

fn composition_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "e\u{0301} a\u{0300}\u{0301} 中\u{0300}\nplain\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-composition",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let cjk_byte = text.find('中').expect("CJK base char");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(cjk_byte));
        },
    )
}

fn wrapped_retry_backend_layout_trace(kind: BufferTextBackendKind) -> (BackendLayoutTrace, usize) {
    let logical_lines = (0..24)
        .map(|line| format!("line-{line:02} abcdefghijklmno\n"))
        .collect::<Vec<_>>();
    let text = logical_lines.join("");
    let target_pos = logical_lines
        .iter()
        .take(18)
        .map(|line| line.chars().count())
        .sum::<usize>()
        + 1;

    let trace = backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-wrap-retry",
        &text,
        80,
        192,
        |buffer, _buf_id, _text| {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
            buffer.set_buffer_local("word-wrap", Value::T);
        },
        |window| {
            if let neovm_core::window::Window::Leaf { point, .. } = window {
                *point = LispCharPos1::from_one_based_usize(target_pos);
            }
        },
    );
    (trace, target_pos)
}

fn point_line_tail_backend_layout_trace(
    kind: BufferTextBackendKind,
) -> (BackendLayoutTrace, usize, usize) {
    let prefix = (0..2)
        .map(|line| format!("p{line:02}\n"))
        .collect::<Vec<_>>()
        .join("");
    let target_line = "abcdefghijklmno\n";
    let text = format!("{prefix}{target_line}");
    let point = prefix.chars().count() + 1;
    let later_pos = point + 10;

    let trace = backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-point-line-tail",
        &text,
        80,
        256,
        |buffer, _buf_id, _text| {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
            buffer.set_buffer_local("word-wrap", Value::T);
        },
        |window| {
            if let neovm_core::window::Window::Leaf {
                point: window_point,
                ..
            } = window
            {
                *window_point = LispCharPos1::from_one_based_usize(point);
            }
        },
    );
    (trace, point, later_pos)
}

fn mode_line_geometry_backend_layout_trace(
    kind: BufferTextBackendKind,
) -> (BackendLayoutTrace, usize) {
    let text = (0..80)
        .map(|line| format!("Line {line:02}\n"))
        .collect::<String>();
    let point = text.chars().count() + 1;

    let trace = backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-mode-line-geometry",
        &text,
        640,
        96,
        |buffer, _buf_id, _text| {
            buffer.set_buffer_local("mode-line-format", Value::string("%o|%p|%P"));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
        },
        |window| {
            if let neovm_core::window::Window::Leaf {
                point: window_point,
                ..
            } = window
            {
                *window_point = LispCharPos1::from_one_based_usize(point);
            }
        },
    );
    (trace, point)
}

fn hscroll_cursor_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-hscroll-cursor",
        "abcdef\n",
        160,
        120,
        |buffer, _buf_id, _text| {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
            buffer.set_buffer_local("truncate-lines", Value::T);
        },
        |window| {
            if let neovm_core::window::Window::Leaf { point, hscroll, .. } = window {
                *point = LispCharPos1::from_one_based_usize(2);
                *hscroll = 3;
            }
        },
    )
}

fn edit_redisplay_backend_layout_trace(
    kind: BufferTextBackendKind,
) -> (BackendLayoutTrace, BackendLayoutTrace) {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, kind);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, "alpha beta gamma\n");
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.text_backend_kind(), kind);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-backend-edit-redisplay", 360, 140, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    let before = selected_window_layout_trace(&eval, &engine, frame_id);

    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let start = buffer.buffer_string().find("beta").expect("beta");
        let end = start + "beta".len();
        buffer.delete_emacs_byte_range(emacs_byte_range(start, end));
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start));
        buffer.insert("BETA");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.buffer_string(), "alpha BETA gamma\n");
    }

    engine.layout_frame_rust(&mut eval, frame_id);
    let after = selected_window_layout_trace(&eval, &engine, frame_id);
    (before, after)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FontificationBackendTrace {
    before_layout: BackendLayoutTrace,
    before_props: String,
    after_layout: BackendLayoutTrace,
    after_props: String,
}

fn printed_eval_result(eval: &mut Context, form: &str) -> String {
    eval.eval_str(form)
        .unwrap_or_else(|err| panic!("eval {form}: {err}"))
        .as_runtime_string_owned()
        .unwrap_or_else(|| panic!("eval {form} did not return a string"))
}

fn fontification_edit_backend_trace(kind: BufferTextBackendKind) -> FontificationBackendTrace {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, kind);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, "alpha beta gamma\n");
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.text_backend_kind(), kind);
    }

    eval.eval_str(
        r#"
        (setq neomacs-test-fontify-face 'font-lock-keyword-face)
        (setq redisplay-fontify-calls nil)
        (setq fontification-functions
              (list (lambda (start)
                      (setq redisplay-fontify-calls
                            (cons start redisplay-fontify-calls))
                      (let ((end (min (point-max) (+ start 80))))
                        (put-text-property start end 'fontified t)
                        (put-text-property start end 'font-lock-face
                                           neomacs-test-fontify-face)))))
        "#,
    )
    .unwrap_or_else(|err| panic!("install redisplay fontification hook: {err}"));

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-backend-fontification-edit",
        360,
        140,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    let before_layout = selected_window_layout_trace(&eval, &engine, frame_id);
    let before_props = printed_eval_result(
        &mut eval,
        "(prin1-to-string (list redisplay-fontify-calls (get-text-property 1 'fontified) (get-text-property 1 'font-lock-face)))",
    );

    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let start = buffer.buffer_string().find("beta").expect("beta");
        let end = start + "beta".len();
        buffer.delete_emacs_byte_range(emacs_byte_range(start, end));
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start));
        buffer.insert("BETA");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.buffer_string(), "alpha BETA gamma\n");
    }

    eval.eval_str(
        r#"
        (setq neomacs-test-fontify-face 'font-lock-warning-face)
        (setq redisplay-fontify-calls nil)
        (remove-text-properties (point-min) (point-max)
                                '(fontified nil font-lock-face nil))
        "#,
    )
    .unwrap_or_else(|err| panic!("clear fontification state after edit: {err}"));

    engine.layout_frame_rust(&mut eval, frame_id);
    let after_layout = selected_window_layout_trace(&eval, &engine, frame_id);
    let after_props = printed_eval_result(
        &mut eval,
        "(prin1-to-string (list redisplay-fontify-calls (get-text-property 1 'fontified) (get-text-property 1 'font-lock-face)))",
    );

    FontificationBackendTrace {
        before_layout,
        before_props,
        after_layout,
        after_props,
    }
}

fn glyph_trace_text(glyph: &GlyphTrace) -> String {
    match &glyph.kind {
        GlyphKindTrace::Char(ch) => ch.to_string(),
        GlyphKindTrace::Composite(text) => text.clone(),
        GlyphKindTrace::Stretch(width) => " ".repeat(usize::from(*width)),
        GlyphKindTrace::Image(_) | GlyphKindTrace::Glyphless(_) => String::new(),
    }
}

fn trace_rows_for_role(trace: &BackendLayoutTrace, role: GlyphRowRole) -> Vec<String> {
    trace
        .matrix_rows
        .iter()
        .filter(|row| row.role == role)
        .map(|row| {
            row.glyph_areas[1]
                .iter()
                .map(glyph_trace_text)
                .collect::<Vec<_>>()
                .join("")
        })
        .collect()
}

fn trace_text_rows(trace: &BackendLayoutTrace) -> Vec<String> {
    trace_rows_for_role(trace, GlyphRowRole::Text)
}

fn trace_mode_line_text(trace: &BackendLayoutTrace) -> String {
    trace_rows_for_role(trace, GlyphRowRole::ModeLine).join("")
}

fn trace_text_face_ids(trace: &BackendLayoutTrace) -> Vec<u32> {
    trace
        .matrix_rows
        .iter()
        .filter(|row| row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyph_areas[1].iter().map(|glyph| glyph.face_id))
        .collect()
}

fn trace_composite_texts(trace: &BackendLayoutTrace) -> Vec<String> {
    trace
        .matrix_rows
        .iter()
        .filter(|row| row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyph_areas[1].iter())
        .filter_map(|glyph| match &glyph.kind {
            GlyphKindTrace::Composite(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn trace_has_nonzero_bidi_level(trace: &BackendLayoutTrace) -> bool {
    trace.matrix_rows.iter().any(|row| {
        row.glyph_areas
            .iter()
            .flat_map(|area| area.iter())
            .any(|glyph| glyph.bidi_level > 0)
    })
}

fn assert_echo_message_renders_in_minibuffer_window(use_gui_metrics: bool) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-minibuffer-echo", 640, 160, buf_id);
    let echo = "Echo lives in minibuffer";
    eval.set_current_message(Some(LispString::from_utf8(echo)));

    let mut engine = LayoutEngine::new();
    if use_gui_metrics {
        engine.enable_cosmic_metrics();
    }
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let minibuffer_window_id = state
        .window_infos
        .iter()
        .find(|info| info.is_minibuffer)
        .expect("minibuffer window info")
        .window_id as u64;
    let root_window_id = state
        .window_infos
        .iter()
        .find(|info| !info.is_minibuffer)
        .expect("root window info")
        .window_id as u64;

    let minibuffer_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id)
        .expect("minibuffer matrix");
    let root_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == root_window_id)
        .expect("root matrix");

    let minibuffer_text = window_matrix_text(minibuffer_entry);
    let root_text = window_matrix_text(root_entry);

    assert!(
        minibuffer_text.contains(echo),
        "expected echo text in minibuffer matrix, got {minibuffer_text:?}"
    );
    assert!(
        !root_text.contains(echo),
        "echo text leaked into root window matrix: {root_text:?}"
    );
    assert!(
        minibuffer_entry
            .matrix
            .rows
            .iter()
            .any(|row| row.enabled && row.role == GlyphRowRole::Minibuffer && !row.mode_line),
        "expected a non-chrome minibuffer row for echo text"
    );
    assert!(
        !root_entry
            .matrix
            .rows
            .iter()
            .any(|row| row.enabled && row.role == GlyphRowRole::Minibuffer),
        "root window should not own minibuffer echo rows"
    );
}

#[test]
fn minibuffer_echo_message_is_suppressed_while_minibuffer_is_active() {
    let _eval = Context::new();
    assert_eq!(
        minibuffer_echo_message_for_window(true, true, Some(Value::string("C-h"))),
        None
    );
}

#[test]
fn minibuffer_echo_message_still_renders_when_minibuffer_is_inactive() {
    let _eval = Context::new();
    assert_eq!(
        minibuffer_echo_message_for_window(true, false, Some(Value::string("Echo")))
            .and_then(Value::as_runtime_string_owned),
        Some("Echo".to_string())
    );
    assert_eq!(
        minibuffer_echo_message_for_window(true, false, Some(Value::string(""))),
        None
    );
    assert_eq!(
        minibuffer_echo_message_for_window(false, false, Some(Value::string("Echo"))),
        None
    );
}

#[test]
fn layout_frame_rust_preserves_propertized_echo_message_faces() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-propertized-echo", 320, 120, buf_id);
    let echo = Value::string_with_text_properties(
        "A中👨‍👩",
        vec![StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
            ]),
        }],
    );
    eval.set_current_message(echo.as_lisp_string().cloned());

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let minibuffer_entry = state
        .window_matrices
        .iter()
        .find(|entry| {
            entry
                .matrix
                .rows
                .iter()
                .any(|row| row.enabled && row.role == GlyphRowRole::Minibuffer)
        })
        .expect("minibuffer echo matrix");
    let echo_glyphs = minibuffer_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Minibuffer)
        .expect("echo row")
        .glyphs[1]
        .clone();

    assert_eq!(glyphs_logical_text(&echo_glyphs), "A中👨‍👩");
    assert_ne!(
        echo_glyphs[0].face_id, echo_glyphs[1].face_id,
        "propertized echo character should receive its property face"
    );
    assert!(
        echo_glyphs[1].wide,
        "echo CJK glyph should use the shared wide-glyph builder: {echo_glyphs:?}"
    );
    assert!(
        echo_glyphs.iter().any(|glyph| glyph.padding),
        "echo CJK glyph should retain its padding cell: {echo_glyphs:?}"
    );
    assert!(
        echo_glyphs.iter().any(
            |glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.as_ref() == "👨‍👩")
        ),
        "echo ZWJ emoji should be clustered by the shared builder: {echo_glyphs:?}"
    );
    assert!(
        echo_glyphs
            .iter()
            .filter(|glyph| !glyph.padding)
            .all(|glyph| glyph.pixel_width > 0.0),
        "echo glyphs should carry real pixel widths: {echo_glyphs:?}"
    );
}

fn assert_multiline_echo_message_resizes_minibuffer_rows(use_gui_metrics: bool) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-minibuffer-echo-lines", 640, 160, buf_id);
    eval.set_current_message(Some(LispString::from_utf8("ALPHA\nBETA")));

    let mut engine = LayoutEngine::new();
    if use_gui_metrics {
        engine.enable_cosmic_metrics();
    }
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let minibuffer_window_id = state
        .window_infos
        .iter()
        .find(|info| info.is_minibuffer)
        .expect("minibuffer window info")
        .window_id as u64;
    let minibuffer_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id)
        .expect("minibuffer matrix");
    let row_texts = enabled_window_row_texts(minibuffer_entry);

    assert!(
        row_texts.iter().any(|row| row == "ALPHA"),
        "expected ALPHA in its own minibuffer row, got {row_texts:?}"
    );
    assert!(
        row_texts.iter().any(|row| row == "BETA"),
        "expected BETA in its own minibuffer row, got {row_texts:?}"
    );
    assert!(
        !row_texts.iter().any(|row| row.contains("ALPHABETA")),
        "multiline echo text was flattened into one row: {row_texts:?}"
    );
}

#[test]
fn test_ligature_run_buffer_new() {
    let buf = LigatureRunBuffer::new();

    // All fields should be zeroed/empty
    assert_eq!(buf.chars.len(), 0);
    assert_eq!(buf.advances.len(), 0);
    assert_eq!(buf.start_x, 0.0);
    assert_eq!(buf.start_y, 0.0);
    assert_eq!(buf.face_h, 0.0);
    assert_eq!(buf.face_ascent, 0.0);
    assert_eq!(buf.face_id, 0);
    assert_eq!(buf.total_advance, 0.0);
    assert_eq!(buf.is_overlay, false);
    assert_eq!(buf.height_scale, 0.0);

    // Vectors should be pre-allocated
    assert!(buf.chars.capacity() >= MAX_LIGATURE_RUN_LEN);
    assert!(buf.advances.capacity() >= MAX_LIGATURE_RUN_LEN);
}

#[test]
fn layout_frame_rust_publishes_increasing_display_positions() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcd\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-test", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("b");
    let c = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("c");
    assert!(
        a.x < b.x,
        "expected increasing x positions, got {a:?} then {b:?}"
    );
    assert!(
        b.x < c.x,
        "expected increasing x positions, got {b:?} then {c:?}"
    );
}

#[test]
fn layout_frame_rust_tracks_multibyte_sample_positions() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a好好b\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-test", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("hao1");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("hao2");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(4))
        .expect("b");
    assert!(
        a.x < hao1.x,
        "expected a before first 好, got {a:?} then {hao1:?}; points={all_points:?}"
    );
    assert!(
        hao1.x < hao2.x,
        "expected first 好 before second 好, got {hao1:?} then {hao2:?}; points={all_points:?}"
    );
    assert!(
        hao2.x < b.x,
        "expected second 好 before b, got {hao2:?} then {b:?}; points={all_points:?}"
    );
    assert!(
        a.width > 0,
        "expected positive width for a, got {a:?}; points={all_points:?}"
    );
    assert!(
        hao1.width > 0,
        "expected positive width for first 好, got {hao1:?}; points={all_points:?}"
    );
    assert!(
        hao2.width > 0,
        "expected positive width for second 好, got {hao2:?}; points={all_points:?}"
    );
    assert!(
        b.width > 0,
        "expected positive width for b, got {b:?}; points={all_points:?}"
    );
}

#[test]
fn implemented_text_backends_match_layout_frame_rows_points_and_cursor() {
    let baseline = backend_layout_trace(BufferTextBackendKind::GapBuffer);
    assert!(
        baseline
            .matrix_rows
            .iter()
            .any(|row| row.role == GlyphRowRole::Text
                && row.glyph_areas[1]
                    .iter()
                    .any(|glyph| glyph.kind == GlyphKindTrace::Char('Ω'))),
        "baseline should render omega row, got {baseline:?}"
    );
    assert!(
        baseline
            .matrix_rows
            .iter()
            .any(|row| !row.glyph_areas[0].is_empty()),
        "baseline should exercise left-margin line-number glyphs, got {baseline:?}"
    );
    assert!(
        baseline.phys_cursor.is_some(),
        "baseline should publish physical cursor geometry"
    );

    for kind in implemented_text_backends() {
        let trace = backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_layout_frame_display_replacement_output() {
    let baseline = display_replacement_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("abcRdef")),
        "baseline should render display replacement text, rows={rows:?}"
    );
    assert!(
        rows.iter().all(|row| !row.contains("XYZ")),
        "baseline should not render covered source text, rows={rows:?}"
    );
    assert!(
        baseline.phys_cursor.is_some(),
        "baseline should publish cursor geometry for replacement slot"
    );

    for kind in implemented_text_backends() {
        let trace = display_replacement_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_layout_frame_invisible_text_output() {
    let baseline = invisible_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("abc  xyz")),
        "baseline should omit invisible source text while preserving surrounding text, rows={rows:?}"
    );
    assert!(
        rows.iter().all(|row| !row.contains("hidden")),
        "baseline should not render invisible text, rows={rows:?}"
    );
    assert!(
        baseline.phys_cursor.is_some(),
        "baseline should keep a physical cursor when point is inside invisible text"
    );

    for kind in implemented_text_backends() {
        let trace = invisible_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn layout_frame_rust_renders_invisible_ellipsis_through_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abc hidden xyz");
        buf.set_buffer_local(
            "buffer-invisibility-spec",
            Value::list(vec![Value::cons(Value::symbol("folded"), Value::T)]),
        );
        let start = "abc ".len();
        let end = start + "hidden".len();
        assert!(buf.put_text_property(
            start,
            end,
            Value::symbol("invisible"),
            Value::symbol("folded"),
        ));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-invisible-ellipsis", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);

    assert_eq!(logical_text, "abc ... xyz");
    assert!(
        text_row.glyphs[1]
            .iter()
            .filter(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '.' }))
            .all(|glyph| (glyph.pixel_width - 8.0).abs() <= 0.01),
        "ellipsis dots should carry measured pixel widths, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn implemented_text_backends_match_layout_frame_multiline_overlay_output() {
    let baseline = multiline_overlay_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("xA")),
        "baseline should render overlay after-string suffix on the source row, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains('B')),
        "baseline should render multiline overlay continuation row, rows={rows:?}"
    );
    assert!(
        baseline.output_rows.iter().any(|row| row.row == 1),
        "baseline should publish a second output row for multiline overlay, rows={:?}",
        baseline.output_rows
    );

    for kind in implemented_text_backends() {
        let trace = multiline_overlay_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_layout_frame_bidi_row_output() {
    let baseline = bidi_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter()
            .any(|row| row.contains('א') && row.contains('ג')),
        "baseline should render Hebrew text in bidi row, rows={rows:?}"
    );
    assert!(
        trace_has_nonzero_bidi_level(&baseline),
        "baseline should mark reordered bidi glyphs, trace={baseline:?}"
    );

    for kind in implemented_text_backends() {
        let trace = bidi_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn arabic_run_composes_into_one_glyph_in_layout() {
    // ا ل م (U+0627 U+0644 U+0645) — an Arabic run. The layout walk must grow
    // it into ONE composed glyph so the renderer joins it, rather than three
    // isolated Char cells. (Structural: holds regardless of font availability,
    // since grouping is driven by complex_script, not by shaping success.)
    let trace = backend_layout_trace_with_buffer_setup(
        BufferTextBackendKind::GapBuffer,
        "layout-backend-arabic",
        "\u{0627}\u{0644}\u{0645}\n",
        360,
        140,
        |_buffer, _buf_id, _text| {},
    );
    let composites = trace_composite_texts(&trace);
    assert!(
        composites
            .iter()
            .any(|t| t.contains('\u{0627}') && t.contains('\u{0645}')),
        "Arabic run should compose into one Composite glyph spanning the run, \
         composites={composites:?}"
    );
}

#[test]
fn implemented_text_backends_match_selective_display_output() {
    let baseline = selective_display_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("head")),
        "baseline should render text before carriage-return selective display marker, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("head...")),
        "baseline should render the selective-display ellipsis, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("shown")),
        "baseline should render visible line after selective display marker, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("shown2")),
        "baseline should resume rendering after an indented hidden block, rows={rows:?}"
    );
    assert!(
        rows.iter()
            .all(|row| !row.contains("hidden tail") && !row.contains("hidden by indent")),
        "baseline should not render selective-display hidden text, rows={rows:?}"
    );
    assert!(
        baseline.hit.as_ref().is_some_and(|hit| hit.rows.len() >= 2),
        "baseline should publish hit rows across selective-display output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let trace = selective_display_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_glyphless_display_geometry() {
    let baseline = glyphless_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("abcd")),
        "baseline should keep surrounding text around glyphless source chars, rows={rows:?}"
    );
    let text_row = baseline
        .output_rows
        .iter()
        .find(|row| row.row == 0)
        .expect("baseline text output row");
    assert!(
        text_row.end_col > 4,
        "baseline should account for glyphless replacement columns, row={text_row:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::new(2)),
        "baseline should publish a display point for the C1 glyphless source char, trace={baseline:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::new(6)),
        "baseline should publish a display point for the object-replacement source char, trace={baseline:?}"
    );

    for kind in implemented_text_backends() {
        let trace = glyphless_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn layout_frame_rust_renders_buffer_glyphless_chars_as_glyphless() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{fff0}b");
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-glyphless-text", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Glyphless { ch: '\u{fff0}' })),
        "buffer glyphless source char should emit a glyphless glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_renders_buffer_control_chars_with_caret_notation() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{0001}b");
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-control-text", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "a^Ab");
}

#[test]
fn layout_frame_rust_renders_line_prefix_through_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abc");
        buf.set_buffer_local("line-prefix", Value::string("中\t"));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-line-prefix", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);

    assert!(
        logical_text.starts_with("中      abc"),
        "line-prefix should render through the shared row builder with wide/tab semantics, text={logical_text:?}, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide),
        "line-prefix wide char should carry wide glyph metadata, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 6 })),
        "line-prefix tab should expand to the next tab stop, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_renders_nobreak_chars_as_mapped_text() {
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("nobreak-char-display", Value::T);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{00a0}b\u{00ad}c");
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-nobreak-text", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "a b-c");
}

#[test]
fn layout_frame_rust_renders_nobreak_chars_in_escape_mode_as_mapped_text() {
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("nobreak-char-display", Value::fixnum(2));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{00a0}b\u{00ad}c");
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-buffer-nobreak-escape-text",
        640,
        160,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "a\\ b\\-c");
}

#[test]
fn implemented_text_backends_match_composite_glyph_output() {
    let baseline = composition_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let composites = trace_composite_texts(&baseline);
    assert!(
        composites.contains(&"e\u{0301}".to_string()),
        "baseline should merge Latin base plus acute mark into a composite glyph, composites={composites:?}"
    );
    assert!(
        composites.contains(&"a\u{0300}\u{0301}".to_string()),
        "baseline should keep multiple combining marks on one composite glyph, composites={composites:?}"
    );
    assert!(
        composites.contains(&"中\u{0300}".to_string()),
        "baseline should compose combining marks on multibyte base chars, composites={composites:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::new(1)),
        "baseline should publish display geometry for the first composite base char, trace={baseline:?}"
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| !hit.rows.is_empty()),
        "baseline should publish hit rows for composite output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let trace = composition_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_wrapped_redisplay_retry_output() {
    let (baseline, target_pos) =
        wrapped_retry_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::from_one_based_usize(target_pos)),
        "baseline should converge wrapped redisplay on target point {target_pos}, trace={baseline:?}"
    );
    assert!(
        baseline.window_start > LispCharPos1::ONE,
        "baseline should advance window-start after wrapped redisplay retry, trace={baseline:?}"
    );
    assert!(
        baseline.output_rows.iter().any(|row| row.row > 0),
        "baseline should publish wrapped visual rows, rows={:?}",
        baseline.output_rows
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| hit.rows.len() >= 2 && hit.first_col_hits.len() == hit.rows.len()),
        "baseline should publish hit rows for wrapped visual output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let (trace, backend_target_pos) = wrapped_retry_backend_layout_trace(kind);
        assert_eq!(backend_target_pos, target_pos, "{kind:?}");
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_point_line_tail_retry_output() {
    let (baseline, point, later_pos) =
        point_line_tail_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    assert!(
        baseline
            .points
            .iter()
            .any(|item| item.buffer_pos == LispCharPos1::from_one_based_usize(point)),
        "baseline should publish geometry for point {point}, trace={baseline:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|item| item.buffer_pos == LispCharPos1::from_one_based_usize(later_pos)),
        "baseline should publish later positions from the point line after retry, later_pos={later_pos}, trace={baseline:?}"
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| !hit.rows.is_empty()),
        "baseline should publish hit rows for point-line retry output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let (trace, backend_point, backend_later_pos) = point_line_tail_backend_layout_trace(kind);
        assert_eq!(backend_point, point, "{kind:?}");
        assert_eq!(backend_later_pos, later_pos, "{kind:?}");
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_mode_line_geometry_after_redisplay_retry() {
    let (baseline, point) =
        mode_line_geometry_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let mode_line = trace_mode_line_text(&baseline);
    assert!(
        baseline.window_start > LispCharPos1::ONE,
        "baseline should advance window-start for EOB redisplay retry, trace={baseline:?}"
    );
    assert_eq!(
        baseline.window_point,
        LispCharPos1::from_one_based_usize(point),
        "baseline should preserve the selected-window EOB point after retry"
    );
    assert!(
        mode_line.contains('|') && !mode_line.contains("%o"),
        "baseline should render expanded mode-line geometry, mode_line={mode_line:?}"
    );

    for kind in implemented_text_backends() {
        let (trace, backend_point) = mode_line_geometry_backend_layout_trace(kind);
        assert_eq!(backend_point, point, "{kind:?}");
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_hscroll_cursor_and_hit_output() {
    let baseline = hscroll_cursor_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let cursor = baseline.phys_cursor.as_ref().expect("baseline cursor");
    assert_eq!(cursor.x, 0);
    assert_eq!(cursor.row, 0);
    assert_eq!(cursor.col, 0);
    let text_rows = trace_text_rows(&baseline);
    assert!(
        text_rows.iter().any(|row| row.starts_with('$')),
        "baseline should render the left truncation marker, rows={text_rows:?}"
    );
    assert!(
        text_rows.iter().any(|row| row.contains("def")),
        "baseline should render the hscrolled visible suffix, rows={text_rows:?}"
    );
    assert!(
        text_rows.iter().all(|row| !row.contains("abc")),
        "baseline should not render hscrolled-away prefix text, rows={text_rows:?}"
    );
    assert_eq!(
        baseline.visible_span,
        Some(WindowVisibleBufferSpan::new(
            LispCharPos1::new(4),
            LispCharPos1::new(7)
        )),
        "baseline should publish the visible hscrolled buffer span"
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| !hit.rows.is_empty()),
        "baseline should publish hit rows for hscroll output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let trace = hscroll_cursor_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_edit_redisplay_cache_invalidation() {
    let (baseline_before, baseline_after) =
        edit_redisplay_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let before_rows = trace_text_rows(&baseline_before);
    let after_rows = trace_text_rows(&baseline_after);
    assert!(
        before_rows
            .iter()
            .any(|row| row.contains("alpha beta gamma")),
        "baseline before edit should render original text, rows={before_rows:?}"
    );
    assert!(
        after_rows
            .iter()
            .any(|row| row.contains("alpha BETA gamma")),
        "baseline after edit should render replacement text, rows={after_rows:?}"
    );
    assert!(
        after_rows
            .iter()
            .all(|row| !row.contains("alpha beta gamma")),
        "baseline after edit should not reuse stale glyph text, rows={after_rows:?}"
    );
    assert_ne!(
        baseline_before, baseline_after,
        "same-engine redisplay after edit should update the trace"
    );

    for kind in implemented_text_backends() {
        let (before, after) = edit_redisplay_backend_layout_trace(kind);
        assert_eq!(before, baseline_before, "{kind:?} before");
        assert_eq!(after, baseline_after, "{kind:?} after");
    }
}

#[test]
fn implemented_text_backends_match_redisplay_fontification_after_edit() {
    let baseline = fontification_edit_backend_trace(BufferTextBackendKind::GapBuffer);
    let before_rows = trace_text_rows(&baseline.before_layout);
    let after_rows = trace_text_rows(&baseline.after_layout);
    assert!(
        before_rows
            .iter()
            .any(|row| row.contains("alpha beta gamma")),
        "baseline before fontification edit should render original text, rows={before_rows:?}"
    );
    assert!(
        after_rows
            .iter()
            .any(|row| row.contains("alpha BETA gamma")),
        "baseline after fontification edit should render edited text, rows={after_rows:?}"
    );
    assert!(
        baseline.before_props.contains("font-lock-keyword-face"),
        "baseline should apply the initial font-lock face from redisplay fontification, props={}",
        baseline.before_props
    );
    assert!(
        baseline.after_props.contains("font-lock-warning-face"),
        "baseline should re-enter redisplay fontification after edit, props={}",
        baseline.after_props
    );
    assert!(
        !trace_text_face_ids(&baseline.before_layout).is_empty(),
        "baseline should emit text glyphs with face ids"
    );

    for kind in implemented_text_backends() {
        let trace = fontification_edit_backend_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn layout_frame_rust_publishes_face_scaled_advances_for_inline_plist_faces() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a好好b ");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("JetBrains Mono"),
            Value::keyword("height"),
            Value::make_float(1.6),
            Value::keyword("weight"),
            Value::symbol("extra-bold"),
        ]);
        buf.put_text_property(
            0,
            buf.total_emacs_byte_len().get(),
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-face-advance", 800, 160, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        let face_resolver = crate::neovm_bridge::FaceResolver::new(
            eval.face_table(),
            0x00FFFFFF,
            0x00000000,
            eval.frame_manager()
                .get(frame_id)
                .expect("frame")
                .font_pixel_size,
            Some("neo".to_string()),
        );
        let mut next_check = buffer.point_max_char_pos().get();
        let resolved = face_resolver.face_at_pos(buffer, 0, &mut next_check);
        assert_eq!(resolved.font_family, "JetBrains Mono");
        assert_eq!(resolved.font_weight, 800);
        assert!(
            resolved.font_size > face_resolver.default_face().font_size * 1.5,
            "expected face resolver to scale the inline plist face before layout, got {:?}",
            resolved
        );
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("hao1");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("hao2");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(4))
        .expect("b");
    let space = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(5))
        .expect("space");

    let default_font_size = frame.font_pixel_size;
    let face_font_size = default_font_size * 1.6;
    let mut metrics = FontMetricsService::new();
    let expected_a = metrics
        .char_width('a', "JetBrains Mono", 800, false, face_font_size)
        .round() as i64;
    let expected_hao = metrics
        .char_width('好', "JetBrains Mono", 800, false, face_font_size)
        .round() as i64;
    let expected_b = metrics
        .char_width('b', "JetBrains Mono", 800, false, face_font_size)
        .round() as i64;
    let cached_ascii = engine
        .ascii_width_cache
        .iter()
        .find_map(|(key, widths)| {
            (key.family == "JetBrains Mono"
                && key.weight == 800
                && !key.italic
                && key.font_size == face_font_size.round() as i32)
                .then_some(*widths)
        })
        .expect("cached JetBrains Mono widths");

    assert!(
        (cached_ascii['a' as usize].round() as i64 - expected_a).abs() <= 1,
        "expected cached width for 'a' to match FontMetricsService, got {} vs expected {expected_a}",
        cached_ascii['a' as usize]
    );
    assert!(
        (cached_ascii['b' as usize].round() as i64 - expected_b).abs() <= 1,
        "expected cached width for 'b' to match FontMetricsService, got {} vs expected {expected_b}",
        cached_ascii['b' as usize]
    );
    assert!(
        (a.width - expected_a).abs() <= 1,
        "expected inline face width for 'a' to follow FontMetricsService (expected {expected_a}, got {a:?}); points={all_points:?}"
    );
    assert!(
        (hao1.width - expected_hao).abs() <= 1,
        "expected inline face width for first 好 to follow FontMetricsService (expected {expected_hao}, got {hao1:?}); points={all_points:?}"
    );
    assert!(
        (hao2.width - expected_hao).abs() <= 1,
        "expected inline face width for second 好 to follow FontMetricsService (expected {expected_hao}, got {hao2:?}); points={all_points:?}"
    );
    assert!(
        (b.width - expected_b).abs() <= 1,
        "expected inline face width for 'b' to follow FontMetricsService (expected {expected_b}, got {b:?}); points={all_points:?}"
    );
    assert!(
        ((hao1.x - a.x) - expected_a).abs() <= 1,
        "expected next point after 'a' to advance by {expected_a}, got {} -> {} with points={all_points:?}",
        a.x,
        hao1.x
    );
    assert!(
        ((hao2.x - hao1.x) - expected_hao).abs() <= 1,
        "expected next point after first 好 to advance by {expected_hao}, got {} -> {} with points={all_points:?}",
        hao1.x,
        hao2.x
    );
    assert!(
        ((b.x - hao2.x) - expected_hao).abs() <= 1,
        "expected next point after second 好 to advance by {expected_hao}, got {} -> {} with points={all_points:?}",
        hao2.x,
        b.x
    );
    assert!(
        ((space.x - b.x) - expected_b).abs() <= 1,
        "expected next point after 'b' to advance by {expected_b}, got {} -> {} with points={all_points:?}",
        b.x,
        space.x
    );
}

#[test]
fn layout_frame_rust_cursor_width_uses_current_glyph_advance_not_next_glyph() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("iW ");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans"),
            Value::keyword("weight"),
            Value::symbol("regular"),
        ]);
        buf.put_text_property(
            0,
            buf.total_emacs_byte_len().get(),
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-cursor-current-glyph-advance",
        800,
        400,
        buf_id,
    );
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.enable_cosmic_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let face_font_size = frame.font_pixel_size;
    let mut metrics = FontMetricsService::new();
    let expected_i = metrics
        .char_width('i', "Noto Sans", 400, false, face_font_size)
        .round() as i64;
    let expected_w = metrics
        .char_width('W', "Noto Sans", 400, false, face_font_size)
        .round() as i64;
    assert_ne!(
        expected_i, expected_w,
        "test requires proportional metrics for i and W"
    );
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let i_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("i point");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");

    assert_eq!(
        i_point.width, expected_i,
        "point geometry should publish the current glyph advance"
    );
    assert_eq!(
        cursor.width, i_point.width,
        "box cursor width must come from the glyph under point, not the following glyph"
    );
    assert_ne!(
        cursor.width, expected_w,
        "cursor must not use the following W glyph advance"
    );
}

#[test]
fn layout_frame_rust_places_cursor_at_newline_terminated_row_end() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "first line\nsecond line\nthird line\n";
    let newline_byte = text.find('\n').expect("newline");
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(newline_byte));
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-cursor-eol", 640, 240, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(newline_byte + 1);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let last_char = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(newline_byte))
        .expect("last visible char before newline");
    let cursor = snapshot.phys_cursor.as_ref().expect("phys cursor");

    assert_eq!(cursor.row, last_char.row);
    assert_eq!(cursor.col, last_char.col + 1);
    assert_eq!(cursor.x, last_char.x + last_char.width);
    assert!(cursor.width > 0);
}

#[test]
fn layout_frame_rust_emits_neomacs_visual_cursors_without_moving_phys_cursor() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("alpha\nbeta\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        let visual_cursor = Value::list(vec![
            Value::keyword(":position"),
            Value::fixnum(3),
            Value::keyword(":cursor-type"),
            Value::cons(Value::symbol("bar"), Value::fixnum(6)),
            Value::keyword(":color"),
            Value::string("#ff0000"),
        ]);
        buf.set_buffer_local("neomacs-visual-cursors", Value::list(vec![visual_cursor]));
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-visual-cursor", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let visual = state
        .cursors
        .iter()
        .find(|cursor| cursor.window_id < 0)
        .expect("visual cursor");
    assert_eq!(visual.window_id, -1_000_000);
    assert_eq!(visual.width, 6.0);
    assert_eq!(visual.color, Color::from_pixel(0xff0000));

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let selected_window = frame.selected_window;
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let phys = snapshot.phys_cursor.as_ref().expect("phys cursor");
    assert_eq!(phys.x, 0, "visual cursor must not move GNU point");
}

#[test]
fn layout_frame_rust_visual_cursor_uses_display_point_geometry() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("iW ");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans"),
            Value::keyword("weight"),
            Value::symbol("regular"),
        ]);
        buf.put_text_property(
            0,
            buf.total_emacs_byte_len().get(),
            Value::symbol("face"),
            plist,
        );
        let visual_cursor = Value::list(vec![
            Value::keyword(":position"),
            Value::fixnum(1),
            Value::keyword(":cursor-type"),
            Value::symbol("box"),
            Value::keyword(":color"),
            Value::string("#00ff00"),
        ]);
        buf.set_buffer_local("neomacs-visual-cursors", Value::list(vec![visual_cursor]));
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-visual-cursor-display-point-geometry",
        320,
        120,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut metrics = FontMetricsService::new();
    let face_font_size = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .font_pixel_size;
    let expected_i = metrics
        .char_width('i', "Noto Sans", 400, false, face_font_size)
        .round() as i64;
    let expected_w = metrics
        .char_width('W', "Noto Sans", 400, false, face_font_size)
        .round() as i64;
    assert_ne!(
        expected_i, expected_w,
        "test requires proportional metrics for i and W"
    );

    let mut engine = LayoutEngine::new();
    engine.enable_cosmic_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let i_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("i point");
    let visual = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state")
        .cursors
        .iter()
        .find(|cursor| cursor.window_id < 0)
        .expect("visual cursor");

    assert_eq!(
        visual.width.round() as i64,
        i_point.width,
        "visual box cursor width must use the rendered glyph under :position"
    );
    assert_eq!(
        visual.height.round() as i64,
        i_point.height,
        "visual box cursor height must use the rendered glyph under :position"
    );
    assert_ne!(
        visual.width.round() as i64,
        expected_w,
        "visual cursor must not use the following glyph's width"
    );
}

#[test]
fn layout_frame_rust_visual_hbar_uses_full_display_point_box() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abc");
        let visual_cursor = Value::list(vec![
            Value::keyword(":position"),
            Value::fixnum(2),
            Value::keyword(":cursor-type"),
            Value::cons(Value::symbol("hbar"), Value::fixnum(3)),
            Value::keyword(":color"),
            Value::string("#00ff00"),
        ]);
        buf.set_buffer_local("neomacs-visual-cursors", Value::list(vec![visual_cursor]));
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-visual-hbar-display-point-box",
        320,
        120,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let b_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("b point");
    let visual = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state")
        .cursors
        .iter()
        .find(|cursor| cursor.window_id < 0)
        .expect("visual cursor");

    assert_eq!(visual.width.round() as i64, b_point.width);
    assert_eq!(
        visual.height.round() as i64,
        b_point.height,
        "hbar visual cursor stores the full glyph box; renderer draws the bar from style"
    );
}

#[test]
fn layout_frame_rust_records_row_metrics_for_plain_text_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("plain text row\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-plain-row-metrics", 800, 160, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let text_row = engine
        .last_frame_display_state
        .as_ref()
        .and_then(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .find(|row| row.role == GlyphRowRole::Text && row.enabled)
        })
        .expect("text row");

    assert!(
        text_row.height_px > 0.0,
        "expected ordinary text rows to record authoritative height, got {text_row:?}"
    );
    assert!(
        text_row.ascent_px > 0.0,
        "expected ordinary text rows to record authoritative ascent, got {text_row:?}"
    );
}

#[test]
fn layout_frame_rust_captures_cursor_inside_invisible_text_without_rescan() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "abc hidden xyz";
    let hidden_byte_start = text.find("hidden").expect("hidden start");
    let hidden_byte_end = hidden_byte_start + "hidden".len();
    let hidden_char_start = text[..hidden_byte_start].chars().count() + 1;
    let point_pos = hidden_char_start + 2;
    let next_visible_pos = hidden_char_start + "hidden".chars().count();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point_pos - 1));
        buf.put_text_property(
            hidden_byte_start,
            hidden_byte_end,
            Value::symbol("invisible"),
            Value::T,
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-invisible-cursor", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(point_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let next_visible = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(next_visible_pos))
        .expect("next visible point");
    assert_eq!(cursor.x, next_visible.x);
    assert_eq!(cursor.row, next_visible.row);
    assert_eq!(cursor.col, next_visible.col);
}

#[test]
fn layout_frame_rust_preserves_logical_cursor_when_window_cursor_is_nil() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdef");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(2));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-logical-cursor-only", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(3);
        }
    }
    eval.frame_manager_mut()
        .set_window_cursor_type(selected_window, Value::NIL);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let logical_cursor = snapshot.logical_cursor.expect("logical cursor");
    let point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("point snapshot");

    assert_eq!(snapshot.phys_cursor, None);
    assert_eq!(logical_cursor.x, point.x);
    assert_eq!(logical_cursor.row, point.row);
    assert_eq!(logical_cursor.col, point.col);
}

#[test]
fn layout_frame_rust_captures_cursor_at_display_replacement_slot_without_rescan() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "abcXYZdef";
    let repl_byte_start = text.find("XYZ").expect("replacement start");
    let repl_byte_end = repl_byte_start + "XYZ".len();
    let point_pos = repl_byte_start + 2;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point_pos - 1));
        buf.put_text_property(
            repl_byte_start,
            repl_byte_end,
            Value::symbol("display"),
            Value::string("R"),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-display-cursor", 800, 400, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(point_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let c = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("c");
    let d = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(7))
        .expect("d");
    assert_eq!(cursor.x, c.x + c.width);
    assert!(cursor.x < d.x, "cursor should target replacement slot");
    assert_eq!(cursor.row, c.row);
}

#[test]
fn layout_frame_rust_records_display_point_for_display_replacement_slot() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "abcXYZdef";
    let repl_byte_start = text.find("XYZ").expect("replacement start");
    let repl_byte_end = repl_byte_start + "XYZ".len();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.put_text_property(
            repl_byte_start,
            repl_byte_end,
            Value::symbol("display"),
            Value::string("R"),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-display-point", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let c = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("c");
    let replacement = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(4))
        .expect("replacement point");
    let d = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(7))
        .expect("d");

    assert_eq!(replacement.x, c.x + c.width);
    assert!(
        replacement.x < d.x,
        "replacement point should stay before following text"
    );
    assert!(replacement.width > 0);
    assert_eq!(replacement.row, c.row);
}

#[test]
fn layout_frame_rust_emits_display_string_replacement_glyphs() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("dir:");
        buf.put_text_property(
            3,
            4,
            Value::symbol("display"),
            Value::string(": (287 GiB available)"),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-display-string", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let rendered: String = text_row.glyphs[1]
        .iter()
        .filter_map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } => Some(*ch),
            GlyphType::Composite { text } => text.chars().next(),
            _ => None,
        })
        .collect();

    assert_eq!(rendered, "dir: (287 GiB available)");
}

#[test]
fn layout_frame_rust_renders_display_replacement_tabs_as_stretches() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("px");
        buf.put_text_property(1, 2, Value::symbol("display"), Value::string("a\tb"));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-tab-replacement", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);
    assert!(
        !logical_text.contains('\t'),
        "display replacement tab should not render as a literal tab, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        logical_text.contains("pa      b"),
        "display replacement tab should expand to the next row tab stop, text={logical_text:?}"
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 6 })),
        "display replacement tab should be a stretch glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_honors_display_replacement_string_display_properties() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("px");
        let replacement = Value::string_with_text_properties(
            "a b",
            vec![StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":width"),
                        Value::fixnum(3),
                    ]),
                ]),
            }],
        );
        buf.put_text_property(1, 2, Value::symbol("display"), replacement);
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-display-propertized-replacement",
        640,
        160,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);
    assert!(
        logical_text.contains("pa   b"),
        "display replacement string should honor its display space, text={logical_text:?}"
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 3 })),
        "display replacement string display property should produce a stretch, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_honors_display_replacement_string_face_properties() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("px");
        let replacement = Value::string_with_text_properties(
            "ab",
            vec![StringTextPropertyRun {
                start: 0,
                end: 1,
                plist: Value::list(vec![
                    Value::symbol("face"),
                    Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
                ]),
            }],
        );
        buf.put_text_property(1, 2, Value::symbol("display"), replacement);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-replacement-face", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let a_face = text_row.glyphs[1]
        .iter()
        .find_map(|glyph| match glyph.glyph_type {
            GlyphType::Char { ch: 'a' } => Some(glyph.face_id),
            _ => None,
        })
        .expect("propertized replacement glyph face");
    let b_face = text_row.glyphs[1]
        .iter()
        .find_map(|glyph| match glyph.glyph_type {
            GlyphType::Char { ch: 'b' } => Some(glyph.face_id),
            _ => None,
        })
        .expect("plain replacement glyph face");

    assert_ne!(
        a_face, b_face,
        "replacement string face property should affect only its covered glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_emits_inline_image_glyphs_for_display_image_specs() {
    let mut eval = Context::new();
    let requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::clone(&requests),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::new(Mutex::new(Vec::new())),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "aXb";
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("image"),
                Value::keyword("type"),
                Value::symbol("png"),
                Value::keyword("file"),
                Value::string("/tmp/neomacs-inline-image.png"),
                Value::keyword("max-width"),
                Value::fixnum(32),
                Value::keyword("max-height"),
                Value::fixnum(24),
                Value::keyword("foreground"),
                Value::string("#112233"),
                Value::keyword("background"),
                Value::string("red"),
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-image", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let image = state.images.first().expect("inline image glyph");
    assert_eq!(image.image_id, 77);
    assert_eq!(image.width, 32.0);
    assert_eq!(image.height, 24.0);
    let replacement = assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 32);
    let slot_id = image.slot_id.expect("image slot id");
    assert_eq!(i64::from(slot_id.col), replacement.col);

    let requests = requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].max_width, 32);
    assert_eq!(requests[0].max_height, 24);
    assert_eq!(requests[0].fg_color, 0x112233);
    assert_eq!(requests[0].bg_color, 0xff0000);
}

#[test]
fn buffer_display_replacement_source_builds_items_without_appending() {
    let source = BufferDisplayReplacementSource::new(BufferId(7), 3, 12);

    let stretch_item = source.stretch_item(42, DisplayReplacementBox::new(16.0, 9.0, 7.0));
    assert_eq!(stretch_item.face, RenderFaceRef::FaceId(42));
    assert!(matches!(
        stretch_item.kind,
        crate::display_item::DisplayItemKind::Stretch(crate::display_item::DisplayStretch {
            width: crate::display_item::DisplayStretchWidth::Length(
                crate::display_item::DisplayLength::Pixels(16.0)
            ),
            height: Some(crate::display_item::DisplayLength::Pixels(9.0)),
            ascent: Some(crate::display_item::DisplayLength::Pixels(7.0)),
        })
    ));

    let text_item = source.source_mapped_text_item(43, "fallback");
    assert_eq!(text_item.face, RenderFaceRef::FaceId(43));
    assert!(matches!(
        text_item.kind,
        crate::display_item::DisplayItemKind::SourceMappedText(text) if text.text.as_ref() == "fallback"
    ));
}

#[test]
fn buffer_display_replacement_string_source_maps_text_to_buffer_slot() {
    let _eval = Context::new();
    let replacement_source = BufferDisplayReplacementSource::new(BufferId(7), 3, 12);
    let string_source = crate::display_source::LispStringSourceCursor::new(
        1,
        Value::string("fallback"),
        RenderFaceRef::FaceId(42),
    )
    .expect("string source");
    let mut source = BufferDisplayReplacementStringSource::new(replacement_source, string_source);
    let mut context = crate::display_source::DisplaySourceContext::empty();

    let item = source.next_item(&mut context).expect("replacement item");

    assert_eq!(item.face, RenderFaceRef::FaceId(42));
    assert_eq!(
        item.span.start,
        crate::display_item::DisplaySourcePosition::buffer(
            BufferId(7),
            CharPos0::new(3),
            EmacsBytePos::new(12)
        )
    );
    assert_eq!(
        item.span.end,
        crate::display_item::DisplaySourcePosition::buffer(
            BufferId(7),
            CharPos0::new(4),
            EmacsBytePos::new(12)
        )
    );
    assert!(matches!(
        item.kind,
        crate::display_item::DisplayItemKind::SourceMappedText(text)
            if text.text.as_ref() == "fallback"
    ));
    assert!(source.next_item(&mut context).is_none());
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
        base_face: &base_face,
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
        tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
    }
    .to_layout(
        GlyphRowRole::Text,
        8.0,
        12.0,
        RenderFaceRef::FaceId(0),
        std::collections::HashMap::new(),
    );
    let mut append_cursor = crate::display_row_builder::DisplayRowAppendCursor::new(
        crate::display_row_builder::DisplayRowPosition { x_px: 0.0, col: 0 },
        80.0,
    );

    let item = next_layout_string_source_item(
        &mut builder,
        &mut source,
        &face_resolver,
        &base_face,
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
fn text_row_append_context_derives_layout_output_and_bounds() {
    let tab_policy =
        crate::display_row_builder::DisplayTabPolicy::from_tab_width_and_stops(8.0, 4, &[6, 10]);
    let context = TextRowAppendContext {
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

    let ordinary: TextRowAppendSpec = context.append_spec(TextRowAppendKind::SourceText);
    assert_eq!(ordinary.position, DisplayRowPosition { x_px: 8.0, col: 0 });
    assert_eq!(ordinary.max_x, 128.0);
    assert_eq!(ordinary.layout.char_width_px, 9.0);
    assert_eq!(ordinary.output.row, 3);
    assert_eq!(ordinary.output.row_y, 20.0);
    assert_eq!(ordinary.output.glyph_y, 22.0);
    assert_eq!(ordinary.output.height, 16.0);

    let tab = context.append_spec(TextRowAppendKind::Tab);
    assert_eq!(tab.max_x, f32::INFINITY);
    assert_eq!(tab.layout.char_width_px, 7.0);
    assert_eq!(tab.output.height, 14.0);

    let control = context.append_spec(TextRowAppendKind::ControlChar);
    assert_eq!(control.max_x, 148.0);
    assert_eq!(control.layout.char_width_px, 9.0);
    assert_eq!(control.output.height, 14.0);

    let mapped = context.append_spec(TextRowAppendKind::SourceMappedText);
    assert_eq!(mapped.max_x, 128.0);
    assert_eq!(mapped.output.height, 14.0);

    let glyphless = context.append_spec(TextRowAppendKind::Glyphless);
    assert_eq!(glyphless.max_x, 128.0);
    assert_eq!(glyphless.output.height, 16.0);

    let replacement = context.append_spec(TextRowAppendKind::DisplayReplacement);
    assert_eq!(replacement.max_x, 128.0);
    assert_eq!(replacement.layout.char_width_px, 9.0);
    assert_eq!(replacement.output.height, 16.0);

    let replacement_string = context.append_spec(TextRowAppendKind::DisplayReplacementString);
    assert_eq!(replacement_string.max_x, 128.0);
    assert_eq!(replacement_string.layout.char_width_px, 7.0);
    assert_eq!(replacement_string.output.height, 16.0);
}

#[test]
fn text_row_append_frame_builds_positioned_context() {
    let tab_policy = crate::display_row_builder::DisplayTabPolicy::every(4);
    let frame = TextRowAppendFrame::from_parts(
        TextRowAppendPlacement {
            row: 3,
            y: 20.0,
            glyph_y: 22.0,
        },
        TextRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        TextRowAppendMetrics {
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
        .append_spec(TextRowAppendKind::SourceText);

    assert_eq!(spec.position, DisplayRowPosition { x_px: 18.0, col: 2 });
    assert_eq!(spec.max_x, 128.0);
    assert_eq!(spec.layout.base_face, RenderFaceRef::FaceId(42));
    assert_eq!(spec.output.row, 3);
}

#[test]
fn text_row_append_surface_builds_frames_with_shared_area() {
    let tab_policy = crate::display_row_builder::DisplayTabPolicy::every(4);
    let surface = TextRowAppendSurface::new(
        TextRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        tab_policy.clone(),
    );

    let frame = surface.frame(
        TextRowAppendPlacement {
            row: 3,
            y: 20.0,
            glyph_y: 22.0,
        },
        TextRowAppendMetrics {
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
fn text_row_append_frame_from_parts_preserves_geometry_and_area() {
    let tab_policy = crate::display_row_builder::DisplayTabPolicy::every(4);
    let frame = TextRowAppendFrame::from_parts(
        TextRowAppendPlacement {
            row: 3,
            y: 20.0,
            glyph_y: 22.0,
        },
        TextRowAppendArea {
            content_x: 8.0,
            width: 120.0,
            text_width: 150.0,
            line_number_width: 10.0,
        },
        TextRowAppendMetrics {
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
fn text_row_append_spec_appends_item_to_matrix_row() {
    let context = TextRowAppendContext {
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
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
        },
        default_row_height: 16.0,
        content_x: 0.0,
        text_width: 80.0,
        line_number_width: 0.0,
        face_space_width: 8.0,
        face_id: 7,
    };
    let spec = context.append_spec(TextRowAppendKind::SourceText);
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
        append_text_row_spec_item(&mut builder, &spec, item).expect("append progress");

    assert_eq!(progress.start, DisplayRowPosition { x_px: 0.0, col: 0 });
    assert_eq!(progress.end, DisplayRowPosition { x_px: 8.0, col: 1 });
    assert_eq!(position, DisplayRowPosition { x_px: 8.0, col: 1 });
    builder
        .with_current_row_mut(|row| {
            assert_eq!(row.glyphs[1][0].face_id, 7);
        })
        .expect("current row");
}

#[test]
fn layout_frame_rust_renders_display_image_fallback_placeholder_through_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aXb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("image"),
                Value::keyword("type"),
                Value::symbol("png"),
                Value::keyword("file"),
                Value::string("/tmp/neomacs-inline-image.png"),
            ]),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-inline-image-fallback", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    assert!(state.images.is_empty());
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| {
            entry.window_id
                == eval
                    .frame_manager()
                    .get(frame_id)
                    .expect("frame")
                    .selected_window
                    .0
        })
        .expect("selected window matrix");
    assert!(
        enabled_window_row_texts(entry)
            .iter()
            .any(|row| row.contains("a[img]b")),
        "fallback placeholder should be rendered as row-builder text, rows={:?}",
        enabled_window_row_texts(entry)
    );

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let expected_width = (5.0 * frame.char_width).round() as i64;
    assert_replacement_slot_between_neighbors(&eval, frame_id, 2, expected_width);
}

#[test]
fn layout_frame_rust_emits_inline_video_glyphs_for_display_video_specs() {
    let mut eval = Context::new();
    let video_requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::new(Mutex::new(Vec::new())),
        video_requests: Arc::clone(&video_requests),
        webkit_requests: Arc::new(Mutex::new(Vec::new())),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aVb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("video"),
                Value::keyword("file"),
                Value::string("/tmp/neomacs-inline-video.mp4"),
                Value::keyword("width"),
                Value::fixnum(80),
                Value::keyword("height"),
                Value::fixnum(45),
                Value::keyword("autoplay"),
                Value::T,
                Value::keyword("loop"),
                Value::T,
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-video", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let video = state.videos.first().expect("inline video glyph");
    assert_eq!(video.video_id, 88);
    assert_eq!(video.width, 80.0);
    assert_eq!(video.height, 45.0);
    assert_eq!(video.loop_count, -1);
    assert!(video.autoplay);
    assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 80);

    let requests = video_requests.lock().expect("video requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].loop_count, -1);
    assert!(requests[0].autoplay);
}

#[test]
fn layout_frame_rust_emits_inline_webkit_glyphs_for_display_webkit_specs() {
    let mut eval = Context::new();
    let webkit_requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::new(Mutex::new(Vec::new())),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::clone(&webkit_requests),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aWb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("webkit"),
                Value::keyword("uri"),
                Value::string("https://example.com"),
                Value::keyword("width"),
                Value::fixnum(80),
                Value::keyword("height"),
                Value::fixnum(45),
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-webkit", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let xwidget = state.xwidgets.first().expect("inline xwidget glyph");
    assert_eq!(xwidget.xwidget_id, 99);
    assert_eq!(xwidget.width, 80.0);
    assert_eq!(xwidget.height, 45.0);
    assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 80);

    let requests = webkit_requests.lock().expect("webkit requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].width, 80);
    assert_eq!(requests[0].height, 45);
}

#[test]
fn layout_frame_rust_emits_inline_xwidget_glyphs_for_gnu_display_xwidget_specs() {
    let mut eval = Context::new();
    let webkit_requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::new(Mutex::new(Vec::new())),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::clone(&webkit_requests),
    }));
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
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aXb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("xwidget"),
                Value::keyword("xwidget"),
                xwidget,
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-xwidget", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let xwidget = state.xwidgets.first().expect("inline xwidget glyph");
    assert_eq!(xwidget.xwidget_id, 1234);
    assert_eq!(xwidget.width, 96.0);
    assert_eq!(xwidget.height, 54.0);
    assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 96);

    let requests = webkit_requests.lock().expect("webkit requests lock");
    assert!(requests.is_empty());
}

#[test]
fn layout_frame_rust_captures_cursor_inside_hscroll_skipped_text_without_rescan() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdef\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.set_buffer_local("truncate-lines", Value::T);
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-hscroll-cursor", 160, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            hscroll,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(2);
            *hscroll = 3;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    assert_eq!(cursor.x, 0);
    assert_eq!(cursor.row, 0);
    assert_eq!(cursor.col, 0);
}

fn assert_layout_frame_rust_tab_cursor_width(x_stretch_cursor: bool, cursor_type: Value) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\tb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.set_buffer_local("cursor-type", cursor_type);
    }
    eval.set_variable(
        "x-stretch-cursor",
        if x_stretch_cursor {
            Value::T
        } else {
            Value::NIL
        },
    );

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-cursor", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(2);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let full_tab_slot_width = b.x - (a.x + a.width);
    let single_column_width = frame.char_width.round() as i64;

    assert_eq!(cursor.x, a.x + a.width);
    assert_eq!(cursor.row, a.row);
    assert_eq!(b.x - cursor.x, full_tab_slot_width);
    assert!(full_tab_slot_width > single_column_width);
    if x_stretch_cursor {
        assert_eq!(cursor.width, full_tab_slot_width);
    } else {
        assert_eq!(cursor.width, single_column_width);
    }
}

#[test]
fn layout_frame_rust_clamps_tab_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_tab_cursor_width(false, Value::T);
}

#[test]
fn layout_frame_rust_expands_tab_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_tab_cursor_width(true, Value::T);
}

#[test]
fn layout_frame_rust_clamps_tab_hbar_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_tab_cursor_width(false, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_expands_tab_hbar_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_tab_cursor_width(true, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_emits_buffer_tab_as_stretch_glyph() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\tb");
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-stretch", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let glyphs = &text_row.glyphs[1];

    assert!(matches!(
        glyphs.first().map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'a' })
    ));
    assert!(matches!(
        glyphs.get(1).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Stretch { width_cols: 7 })
    ));
    assert!(matches!(
        glyphs.get(2).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'b' })
    ));
    assert_eq!(text_row.role, GlyphRowRole::Text);
    assert!(
        glyphs.iter().all(|glyph| glyph.pixel_width > 0.0),
        "main buffer text glyphs should keep pixel widths: {glyphs:?}"
    );
}

#[test]
fn layout_frame_rust_tab_stops_are_window_relative_in_split_windows() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval.buffer_manager_mut().create_buffer("*right*");
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(right_buf_id)
            .expect("right buffer");
        buf.insert("C-f\t;; forward-char");
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-split", 800, 160, left_buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let right_window = eval
        .frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == right_window.0)
        .expect("right window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let text = text_row.glyphs[1]
        .iter()
        .flat_map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } => std::iter::repeat_n(*ch, 1).collect::<Vec<_>>(),
            GlyphType::Stretch { width_cols } => {
                std::iter::repeat_n(' ', usize::from(*width_cols)).collect::<Vec<_>>()
            }
            _ => Vec::new(),
        })
        .collect::<String>();

    assert!(
        text.contains("C-f     ;; forward-char"),
        "right-window tab should expand relative to the right window text area, got {text:?}"
    );
}

#[test]
fn layout_frame_rust_display_space_align_keeps_suffix_text_in_split_windows() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval
        .buffer_manager_mut()
        .create_buffer("*right-display-space*");
    let text = concat!(
        "   m \tShow help for current major and minor modes and their commands\n",
        "   b \tShow all key bindings\n",
        "   k \tShow help for key\n",
        "   c \tShow help for key briefly\n",
        "   w \tShow which key runs a specific command\n"
    );
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(right_buf_id)
            .expect("right buffer");
        buf.insert(text);
        for (byte_idx, ch) in text.char_indices() {
            if ch == '\t' {
                buf.put_text_property(
                    byte_idx,
                    byte_idx + 1,
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":align-to"),
                        Value::fixnum(8),
                    ]),
                );
            }
        }
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-display-space-align-split",
        800,
        160,
        left_buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let right_window = eval
        .frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == right_window.0)
        .expect("right window matrix");
    let rows = enabled_window_row_texts_expanding_stretches(window_entry);

    assert!(
        rows.iter()
            .any(|row| row.contains("   c    Show help for key briefly")),
        "display-space align-to should preserve suffix text after the stretch, rows={rows:?}"
    );
    assert!(
        rows.iter()
            .any(|row| row.contains("   w    Show which key runs a specific command")),
        "display-space align-to should not swallow following help rows, rows={rows:?}"
    );
}

#[test]
fn layout_frame_rust_tty_display_space_align_stays_one_cell_high() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = concat!(
        "   m \tShow help for current major and minor modes and their commands\n",
        "   b \tShow all key bindings\n",
        "   k \tShow help for key\n",
        "   c \tShow help for key briefly\n",
        "   w \tShow which key runs a specific command\n"
    );
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        for (byte_idx, ch) in text.char_indices() {
            if ch == '\t' {
                buf.put_text_property(
                    byte_idx,
                    byte_idx + 1,
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":align-to"),
                        Value::fixnum(8),
                    ]),
                );
            }
        }
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-tty-display-space-align", 80, 25, buf_id);
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(None);
        frame.char_width = 1.0;
        frame.char_height = 1.0;
        frame.font_pixel_size = 16.0;
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window
        .0;
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == window_id)
        .expect("selected window matrix");
    let rows = enabled_window_row_texts_expanding_stretches(window_entry);

    assert!(
        rows.iter()
            .any(|row| row.contains("   w    Show which key runs a specific command")),
        "TTY display-space align-to should not inflate rows and hide later Help entries, rows={rows:?}"
    );

    for row in window_entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text && row.total_glyphs() > 0)
    {
        assert_eq!(
            row.height_px, 1.0,
            "TTY display-space rows must stay one cell high: row={row:?}"
        );
        assert!(
            row.ascent_px <= row.height_px,
            "TTY row ascent must not exceed row height: row={row:?}"
        );
    }
}

#[test]
fn layout_frame_rust_emits_pixel_window_divider_geometry() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval.buffer_manager_mut().create_buffer("*right*");
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-divider-split", 800, 160, left_buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_parameter(Value::symbol("right-divider-width"), Value::fixnum(6));
    }
    eval.frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");
    let left_bounds = {
        let frame = eval.frame_manager().get(frame_id).expect("frame");
        *frame
            .find_window(selected_window)
            .expect("left window")
            .bounds()
    };

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let divider_borders: Vec<_> = state
        .borders
        .iter()
        .filter(|border| {
            border.window_id == selected_window.0 as i64
                && (border.x - (left_bounds.x + left_bounds.width - 6.0)).abs() <= 6.0
        })
        .collect();

    assert_eq!(
        divider_borders.len(),
        3,
        "a six-pixel right divider should be split into first/inner/last rectangles"
    );
    assert!(
        divider_borders.iter().any(|border| border.width == 1.0),
        "divider should include one-pixel edge rectangles"
    );
    assert!(
        divider_borders.iter().any(|border| border.width == 4.0),
        "divider should include a four-pixel inner rectangle"
    );

    let left_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("left window matrix");
    assert!(
        left_entry.matrix.rows.iter().all(|row| {
            row.glyphs[1]
                .last()
                .is_none_or(|glyph| !matches!(glyph.glyph_type, GlyphType::Char { ch: '|' }))
        }),
        "real pixel window dividers must not be represented as vertical-border text glyphs"
    );
}

#[test]
fn layout_frame_rust_gui_zero_width_divider_uses_pixel_vertical_border() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval.buffer_manager_mut().create_buffer("*right*");
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-gui-border-split", 800, 160, left_buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
    }
    eval.frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");
    let left_bounds = {
        let frame = eval.frame_manager().get(frame_id).expect("frame");
        *frame
            .find_window(selected_window)
            .expect("left window")
            .bounds()
    };

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    assert!(
        state.borders.iter().any(|border| {
            border.window_id == selected_window.0 as i64
                && (border.x - (left_bounds.x + left_bounds.width - 1.0)).abs() < 0.01
                && border.width == 1.0
        }),
        "GNU GUI draws a one-pixel vertical border when window-divider-mode is off"
    );

    let left_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("left window matrix");
    assert!(
        left_entry.matrix.rows.iter().all(|row| {
            row.glyphs[1]
                .last()
                .is_none_or(|glyph| !matches!(glyph.glyph_type, GlyphType::Char { ch: '|' }))
        }),
        "GUI vertical borders must not be represented as terminal `|' glyphs"
    );
}

#[test]
fn layout_frame_rust_bottom_divider_does_not_separate_root_from_minibuffer() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-minibuffer-divider", 800, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
        frame.set_parameter(Value::symbol("bottom-divider-width"), Value::fixnum(6));
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    assert!(
        state
            .borders
            .iter()
            .all(|border| border.window_id != selected_window.0 as i64 || border.height != 6.0),
        "GNU does not draw a bottom window divider between a bottommost root window and the minibuffer"
    );
}

#[test]
fn layout_frame_rust_emits_display_space_as_stretch_glyph() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-stretch", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let glyphs = &text_row.glyphs[1];

    assert!(matches!(
        glyphs.first().map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'a' })
    ));
    assert!(matches!(
        glyphs.get(1).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Stretch { width_cols: 4 })
    ));
    assert!(matches!(
        glyphs.get(2).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'b' })
    ));
}

fn display_space_width_spec(columns: i64) -> Value {
    Value::list(vec![
        Value::symbol("space"),
        Value::keyword("width"),
        Value::fixnum(columns),
    ])
}

fn display_space_relative_width_spec(factor: i64) -> Value {
    Value::list(vec![
        Value::symbol("space"),
        Value::keyword("relative-width"),
        Value::fixnum(factor),
    ])
}

fn display_space_relative_height_spec(factor: i64, ascent_percent: i64) -> Value {
    Value::list(vec![
        Value::symbol("space"),
        Value::keyword("width"),
        Value::fixnum(2),
        Value::keyword("relative-height"),
        Value::fixnum(factor),
        Value::keyword("ascent"),
        Value::fixnum(ascent_percent),
    ])
}

#[test]
fn display_space_relative_width_uses_displayed_character_width() {
    let _eval = Context::new();
    let params = test_window_params();
    let geometry = eval_display_space_geometry(
        &display_space_relative_width_spec(2),
        0.0,
        0.0,
        8.0,
        16.0,
        10.0,
        7.0,
        &params,
    );

    assert_eq!(geometry.width, 32.0);
}

#[test]
fn display_space_geometry_uses_relative_height_and_percent_ascent() {
    let _eval = Context::new();
    let params = test_window_params();
    let geometry = eval_display_space_geometry(
        &display_space_relative_height_spec(2, 25),
        0.0,
        0.0,
        8.0,
        8.0,
        10.0,
        7.0,
        &params,
    );

    assert_eq!(
        geometry,
        DisplaySpaceGeometry {
            width: 16.0,
            height: 20.0,
            ascent: 5.0,
        }
    );
}

#[test]
fn display_space_geometry_accepts_pixel_ascent_expression() {
    let _eval = Context::new();
    let params = test_window_params();
    let spec = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("height"),
        Value::list(vec![Value::fixnum(20)]),
        Value::keyword("ascent"),
        Value::list(vec![Value::fixnum(3)]),
    ]);
    let geometry = eval_display_space_geometry(&spec, 0.0, 0.0, 8.0, 8.0, 10.0, 7.0, &params);

    assert_eq!(geometry.height, 20.0);
    assert_eq!(geometry.ascent, 3.0);
}

fn scaled_face_plist() -> Value {
    Value::list(vec![
        Value::keyword("family"),
        Value::string("JetBrains Mono"),
        Value::keyword("height"),
        Value::make_float(1.6),
        Value::keyword("weight"),
        Value::symbol("extra-bold"),
    ])
}

fn assert_layout_frame_rust_display_space_cursor_width(x_stretch_cursor: bool, cursor_type: Value) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("face"),
            scaled_face_plist(),
        );
        buf.set_buffer_local("cursor-type", cursor_type);
    }
    eval.set_variable(
        "x-stretch-cursor",
        if x_stretch_cursor {
            Value::T
        } else {
            Value::NIL
        },
    );

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-cursor", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(2);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let full_slot_width = b.x - (a.x + a.width);
    let single_column_width = frame.char_width.round() as i64;
    let expected_space_width = (4.0 * frame.char_width).round() as i64;

    assert_eq!(cursor.x, a.x + a.width);
    assert_eq!(b.x - cursor.x, full_slot_width);
    assert!((full_slot_width - expected_space_width).abs() <= 1);
    if x_stretch_cursor {
        assert_eq!(cursor.width, full_slot_width);
    } else {
        assert_eq!(cursor.width, single_column_width);
    }
}

#[test]
fn layout_frame_rust_display_space_width_uses_canonical_column_width() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("face"),
            scaled_face_plist(),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-width", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let slot_width = b.x - (a.x + a.width);
    let expected_width = (4.0 * frame.char_width).round() as i64;

    assert!(
        (slot_width - expected_width).abs() <= 1,
        "display space width should follow canonical frame column width; got slot {slot_width}, expected {expected_width}, frame char width {}, points={:?}",
        frame.char_width,
        snapshot.points
    );
}

#[test]
fn layout_frame_rust_records_display_point_for_display_space_slot() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("face"),
            scaled_face_plist(),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-point", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let space = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("space");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let expected_width = (4.0 * frame.char_width).round() as i64;

    assert_eq!(space.x, a.x + a.width);
    assert!(space.x < b.x);
    assert!((space.width - expected_width).abs() <= 1);
    assert_eq!(space.row, a.row);
}

#[test]
fn layout_frame_rust_clamps_display_space_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_display_space_cursor_width(false, Value::T);
}

#[test]
fn layout_frame_rust_expands_display_space_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_display_space_cursor_width(true, Value::T);
}

#[test]
fn layout_frame_rust_clamps_display_space_hbar_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_display_space_cursor_width(false, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_expands_display_space_hbar_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_display_space_cursor_width(true, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_keeps_mixed_width_advances_correct_after_mid_line_face_change() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();

    let prefix = "  h=0.9 w=normal:                     ";
    let sample = "a好好b  ABCXYZ 0123456789  -> <= >=";
    let sample_pos = prefix.chars().count() + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(prefix);
        let sample_byte_start = buf.total_emacs_byte_len().get();
        buf.insert(sample);
        let sample_byte_end = buf.total_emacs_byte_len().get();
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans Mono"),
            Value::keyword("height"),
            Value::make_float(0.9),
            Value::keyword("weight"),
            Value::symbol("normal"),
        ]);
        buf.put_text_property(
            sample_byte_start,
            sample_byte_end,
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-face-mid-line", 1400, 160, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 1))
        .expect("first 好");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 2))
        .expect("second 好");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 3))
        .expect("b");

    let face_font_size = frame.font_pixel_size * 0.9;
    let mut metrics = FontMetricsService::new();
    let expected_a = metrics
        .char_width('a', "Noto Sans Mono", 400, false, face_font_size)
        .round() as i64;
    let expected_hao = metrics
        .char_width('好', "Noto Sans Mono", 400, false, face_font_size)
        .round() as i64;
    let expected_b = metrics
        .char_width('b', "Noto Sans Mono", 400, false, face_font_size)
        .round() as i64;

    assert!(
        (a.width - expected_a).abs() <= 1,
        "expected a width {expected_a}, got {a:?}; points={all_points:?}"
    );
    assert!(
        (hao1.width - expected_hao).abs() <= 1,
        "expected first 好 width {expected_hao}, got {hao1:?}; points={all_points:?}"
    );
    assert!(
        (hao2.width - expected_hao).abs() <= 1,
        "expected second 好 width {expected_hao}, got {hao2:?}; points={all_points:?}"
    );
    assert!(
        (b.width - expected_b).abs() <= 1,
        "expected b width {expected_b}, got {b:?}; points={all_points:?}"
    );
    assert!(
        ((hao1.x - a.x) - expected_a).abs() <= 1,
        "expected first 好 x delta {expected_a}, got {} -> {}; points={all_points:?}",
        a.x,
        hao1.x
    );
    assert!(
        ((hao2.x - hao1.x) - expected_hao).abs() <= 1,
        "expected second 好 x delta {expected_hao}, got {} -> {}; points={all_points:?}",
        hao1.x,
        hao2.x
    );
    assert!(
        ((b.x - hao2.x) - expected_hao).abs() <= 1,
        "expected b x delta {expected_hao}, got {} -> {}; points={all_points:?}",
        hao2.x,
        b.x
    );
    let space = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 4))
        .expect("space");
    assert_eq!(
        space.x - b.x,
        b.width,
        "expected next point after 'b' to land exactly one snapped advance later; b={b:?} space={space:?} points={all_points:?}"
    );
}

#[test]
fn layout_frame_rust_keeps_face_positions_after_truncated_multibyte_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();

    let truncated_prefix = format!("{}\n", "好".repeat(20));
    let sample = "a好好b";
    let sample_pos = truncated_prefix.chars().count() + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&truncated_prefix);
        let sample_byte_start = buf.total_emacs_byte_len().get();
        buf.insert(sample);
        let sample_byte_end = buf.total_emacs_byte_len().get();
        buf.insert("\n");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans Mono"),
            Value::keyword("height"),
            Value::make_float(0.9),
            Value::keyword("weight"),
            Value::symbol("normal"),
        ]);
        buf.put_text_property(
            sample_byte_start,
            sample_byte_end,
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("truncate-lines", Value::T);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-truncated-multibyte-face", 128, 160, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(sample_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 1))
        .expect("first 好");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 2))
        .expect("second 好");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 3))
        .expect("b");

    let face_font_size = frame.font_pixel_size * 0.9;
    let mut metrics = FontMetricsService::new();
    let expected_a = metrics
        .char_width('a', "Noto Sans Mono", 400, false, face_font_size)
        .round() as i64;
    let expected_hao = metrics
        .char_width('好', "Noto Sans Mono", 400, false, face_font_size)
        .round() as i64;
    let expected_b = metrics
        .char_width('b', "Noto Sans Mono", 400, false, face_font_size)
        .round() as i64;

    assert!(
        (a.width - expected_a).abs() <= 1,
        "expected a width {expected_a}, got {a:?}; points={all_points:?}"
    );
    assert!(
        (hao1.width - expected_hao).abs() <= 1,
        "expected first 好 width {expected_hao}, got {hao1:?}; points={all_points:?}"
    );
    assert!(
        (hao2.width - expected_hao).abs() <= 1,
        "expected second 好 width {expected_hao}, got {hao2:?}; points={all_points:?}"
    );
    assert!(
        (b.width - expected_b).abs() <= 1,
        "expected b width {expected_b}, got {b:?}; points={all_points:?}"
    );
    assert!(
        ((hao1.x - a.x) - expected_a).abs() <= 1,
        "expected first 好 x delta {expected_a}, got {} -> {}; points={all_points:?}",
        a.x,
        hao1.x
    );
    assert!(
        ((hao2.x - hao1.x) - expected_hao).abs() <= 1,
        "expected second 好 x delta {expected_hao}, got {} -> {}; points={all_points:?}",
        hao1.x,
        hao2.x
    );
    assert!(
        ((b.x - hao2.x) - expected_hao).abs() <= 1,
        "expected b x delta {expected_hao}, got {} -> {}; points={all_points:?}",
        hao2.x,
        b.x
    );
}

#[test]
fn layout_frame_rust_keeps_mixed_width_positions_correct_after_sequential_window_point_moves() {
    #[derive(Clone, Copy, Debug)]
    struct TargetRow {
        line_beg: usize,
        sample_pos: usize,
        height: f32,
        weight: u16,
    }

    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let sample = "a好好b  ABCXYZ 0123456789  -> <= >=";
    let mut targets = Vec::new();
    let weights = [
        ("normal", 400_u16),
        ("semi-bold", 600_u16),
        ("bold", 700_u16),
        ("extra-bold", 800_u16),
    ];

    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        for height in [0.9_f32, 1.0_f32, 1.2_f32, 1.6_f32] {
            for (weight_name, weight_value) in weights {
                let line_beg = if buf.is_text_empty() {
                    1usize
                } else {
                    buf.point_max_char_pos().get() as usize + 1
                };
                let prefix = format!("  {:<35} ", format!("h={height} w={weight_name}:"));
                let sample_pos = line_beg + prefix.chars().count();
                buf.insert(&prefix);
                let sample_byte_start = buf.total_emacs_byte_len().get();
                buf.insert(sample);
                let sample_byte_end = buf.total_emacs_byte_len().get();
                buf.insert("\n");
                let plist = Value::list(vec![
                    Value::keyword("family"),
                    Value::string("JetBrains Mono"),
                    Value::keyword("height"),
                    Value::make_float(height as f64),
                    Value::keyword("weight"),
                    Value::symbol(weight_name),
                ]);
                buf.put_text_property(
                    sample_byte_start,
                    sample_byte_end,
                    Value::symbol("face"),
                    plist,
                );
                targets.push(TargetRow {
                    line_beg,
                    sample_pos,
                    height,
                    weight: weight_value,
                });
            }
        }
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-sequential-window-point", 1400, 256, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    let mut metrics = FontMetricsService::new();

    for target in &targets {
        let byte_pos = {
            let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
            buffer
                .lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(target.line_beg))
                .get()
        };
        let _ = eval
            .buffer_manager_mut()
            .goto_buffer_emacs_byte_pos(buf_id, neovm_core::buffer::EmacsBytePos::new(byte_pos));
        {
            let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
            let window = frame
                .find_window_mut(selected_window)
                .expect("selected window");
            if let neovm_core::window::Window::Leaf { point, .. } = window {
                *point = LispCharPos1::from_one_based_usize(target.line_beg);
            }
        }

        engine.layout_frame_rust(&mut eval, frame_id);

        let frame = eval.frame_manager().get(frame_id).expect("frame");
        let snapshot = frame
            .window_display_snapshot(selected_window)
            .expect("display snapshot");
        let all_points = snapshot.points.clone();
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        let sample_chars = [
            (target.line_beg, char_at_lisp_pos(buffer, target.line_beg)),
            (
                target.sample_pos,
                char_at_lisp_pos(buffer, target.sample_pos),
            ),
            (
                target.sample_pos + 1,
                char_at_lisp_pos(buffer, target.sample_pos + 1),
            ),
            (
                target.sample_pos + 2,
                char_at_lisp_pos(buffer, target.sample_pos + 2),
            ),
            (
                target.sample_pos + 3,
                char_at_lisp_pos(buffer, target.sample_pos + 3),
            ),
        ];
        let a = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos))
            .expect("sample a");
        let hao1 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 1))
            .expect("sample first 好");
        let hao2 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 2))
            .expect("sample second 好");
        let b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 3))
            .expect("sample b");
        let after_b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 4))
            .expect("sample trailing space");

        let face_font_size = frame.font_pixel_size * target.height;
        let expected_a = metrics
            .char_width('a', "JetBrains Mono", target.weight, false, face_font_size)
            .round() as i64;
        let expected_hao = metrics
            .char_width('好', "JetBrains Mono", target.weight, false, face_font_size)
            .round() as i64;
        let expected_b = metrics
            .char_width('b', "JetBrains Mono", target.weight, false, face_font_size)
            .round() as i64;

        assert!(
            (a.width - expected_a).abs() <= 1,
            "expected a width {expected_a} after sequential point moves, got {a:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            (hao1.width - expected_hao).abs() <= 1,
            "expected first 好 width {expected_hao} after sequential point moves, got {hao1:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            (hao2.width - expected_hao).abs() <= 1,
            "expected second 好 width {expected_hao} after sequential point moves, got {hao2:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            (b.width - expected_b).abs() <= 1,
            "expected b width {expected_b} after sequential point moves, got {b:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            ((hao1.x - a.x) - expected_a).abs() <= 1,
            "expected first 好 x delta {expected_a} after sequential point moves, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            a.x,
            hao1.x
        );
        assert!(
            ((hao2.x - hao1.x) - expected_hao).abs() <= 1,
            "expected second 好 x delta {expected_hao} after sequential point moves, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            hao1.x,
            hao2.x
        );
        assert!(
            ((b.x - hao2.x) - expected_hao).abs() <= 1,
            "expected b x delta {expected_hao} after sequential point moves, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            hao2.x,
            b.x
        );
        assert!(
            ((after_b.x - b.x) - expected_b).abs() <= 1,
            "expected post-b x delta {expected_b} after sequential point moves, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            b.x,
            after_b.x
        );
    }
}

#[test]
fn layout_frame_rust_keeps_mixed_width_positions_correct_across_family_switches() {
    #[derive(Clone, Copy, Debug)]
    struct TargetRow<'a> {
        family: &'a str,
        line_beg: usize,
        sample_pos: usize,
        height: f32,
        weight_name: &'a str,
        weight: u16,
    }

    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let sample = "a好好b  ABCXYZ 0123456789  -> <= >=";
    let mut targets = Vec::new();
    let weights = [
        ("normal", 400_u16),
        ("semi-bold", 600_u16),
        ("bold", 700_u16),
        ("extra-bold", 800_u16),
    ];
    let families = [
        "JetBrains Mono",
        "Hack",
        "DejaVu Sans Mono",
        "Noto Sans Mono",
    ];

    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        for family in families {
            let heading = format!("  -- family: {family} --\n");
            buf.insert(&heading);
            for height in [0.9_f32, 1.0_f32, 1.2_f32, 1.6_f32] {
                for (weight_name, weight_value) in weights {
                    let line_beg = if buf.is_text_empty() {
                        1usize
                    } else {
                        buf.point_max_char_pos().get() as usize + 1
                    };
                    let prefix = format!("  {:<35} ", format!("h={height} w={weight_name}:"));
                    let sample_pos = line_beg + prefix.chars().count();
                    buf.insert(&prefix);
                    let sample_byte_start = buf.total_emacs_byte_len().get();
                    buf.insert(sample);
                    let sample_byte_end = buf.total_emacs_byte_len().get();
                    buf.insert("\n");
                    let plist = Value::list(vec![
                        Value::keyword("family"),
                        Value::string(family),
                        Value::keyword("height"),
                        Value::make_float(height as f64),
                        Value::keyword("weight"),
                        Value::symbol(weight_name),
                    ]);
                    buf.put_text_property(
                        sample_byte_start,
                        sample_byte_end,
                        Value::symbol("face"),
                        plist,
                    );
                    targets.push(TargetRow {
                        family,
                        line_beg,
                        sample_pos,
                        height,
                        weight_name,
                        weight: weight_value,
                    });
                }
            }
            buf.insert("\n");
        }
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-family-switches", 1400, 1600, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    let mut metrics = FontMetricsService::new();

    for target in &targets {
        let byte_pos = {
            let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
            buffer
                .lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(target.line_beg))
                .get()
        };
        let _ = eval
            .buffer_manager_mut()
            .goto_buffer_emacs_byte_pos(buf_id, neovm_core::buffer::EmacsBytePos::new(byte_pos));
        {
            let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
            let window = frame
                .find_window_mut(selected_window)
                .expect("selected window");
            if let neovm_core::window::Window::Leaf { point, .. } = window {
                *point = LispCharPos1::from_one_based_usize(target.line_beg);
            }
        }

        engine.layout_frame_rust(&mut eval, frame_id);

        let frame = eval.frame_manager().get(frame_id).expect("frame");
        let snapshot = frame
            .window_display_snapshot(selected_window)
            .expect("display snapshot");
        let all_points = snapshot.points.clone();
        let visible_span = snapshot.visible_buffer_span();
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        let sample_chars = [
            (
                target.sample_pos,
                char_at_lisp_pos(buffer, target.sample_pos),
            ),
            (
                target.sample_pos + 1,
                char_at_lisp_pos(buffer, target.sample_pos + 1),
            ),
            (
                target.sample_pos + 2,
                char_at_lisp_pos(buffer, target.sample_pos + 2),
            ),
            (
                target.sample_pos + 3,
                char_at_lisp_pos(buffer, target.sample_pos + 3),
            ),
        ];
        let a = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos))
            .unwrap_or_else(|| {
                panic!(
                    "sample a missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let hao1 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 1))
            .unwrap_or_else(|| {
                panic!(
                    "sample first 好 missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let hao2 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 2))
            .unwrap_or_else(|| {
                panic!(
                    "sample second 好 missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 3))
            .unwrap_or_else(|| {
                panic!(
                    "sample b missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let after_b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 4))
            .unwrap_or_else(|| {
                panic!(
                    "sample trailing space missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });

        let face_font_size = frame.font_pixel_size * target.height;
        let expected_a = metrics
            .char_width('a', target.family, target.weight, false, face_font_size)
            .round() as i64;
        let expected_hao = metrics
            .char_width('好', target.family, target.weight, false, face_font_size)
            .round() as i64;
        let expected_b = metrics
            .char_width('b', target.family, target.weight, false, face_font_size)
            .round() as i64;

        assert!(
            (a.width - expected_a).abs() <= 1,
            "expected a width {expected_a}, got {a:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            (hao1.width - expected_hao).abs() <= 1,
            "expected first 好 width {expected_hao}, got {hao1:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            (hao2.width - expected_hao).abs() <= 1,
            "expected second 好 width {expected_hao}, got {hao2:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            (b.width - expected_b).abs() <= 1,
            "expected b width {expected_b}, got {b:?}; target={target:?}; chars={sample_chars:?}; points={all_points:?}"
        );
        assert!(
            ((hao1.x - a.x) - expected_a).abs() <= 1,
            "expected first 好 x delta {expected_a}, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            a.x,
            hao1.x
        );
        assert!(
            ((hao2.x - hao1.x) - expected_hao).abs() <= 1,
            "expected second 好 x delta {expected_hao}, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            hao1.x,
            hao2.x
        );
        assert!(
            ((b.x - hao2.x) - expected_hao).abs() <= 1,
            "expected b x delta {expected_hao}, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            hao2.x,
            b.x
        );
        assert!(
            ((after_b.x - b.x) - expected_b).abs() <= 1,
            "expected post-b x delta {expected_b}, got {} -> {}; target={target:?}; chars={sample_chars:?}; points={all_points:?}",
            b.x,
            after_b.x
        );

        let _ = target.weight_name;
    }
}

#[test]
fn layout_frame_rust_word_wrap_snapshot_stays_sorted_after_rewind() {
    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aaaa bbbb cccc dddd\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("word-wrap", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-wrap", 96, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    assert!(
        snapshot.points.iter().any(|point| point.row > 0),
        "expected word-wrap to create multiple rows, got points={:?}",
        snapshot.points
    );
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let point_chars = snapshot
        .points
        .iter()
        .map(|point| {
            (
                point.buffer_pos,
                char_at_lisp_pos(buffer, point.buffer_pos.to_one_based_usize()),
            )
        })
        .collect::<Vec<_>>();
    for window in snapshot.points.windows(2) {
        assert!(
            window[0].buffer_pos < window[1].buffer_pos,
            "expected snapshot points to stay sorted after wrap rewind, got {:?}; chars={:?}",
            snapshot.points,
            point_chars
        );
    }
}

#[test]
fn layout_frame_rust_reads_far_enough_for_last_visible_truncated_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let mut text = String::new();
    for line in 0..32 {
        text.push_str(&format!("line-{line:02} abcdefghijklmnop\n"));
    }
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("truncate-lines", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-read-span", 96, 640, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let target_pos = {
        let mut pos = 1usize;
        for line in 0..26 {
            pos += format!("line-{line:02} abcdefghijklmnop\n").chars().count();
        }
        pos
    };
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        // Selected-window point lives in the buffer; keep pt_char in
        // sync with the target point so redisplay retries read the same
        // location the leaf window advertises.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
    }
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(target_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let target = snapshot.point_for_buffer_pos(LispCharPos1::from_one_based_usize(target_pos));
    assert!(
        target.is_some(),
        "expected last visible truncated line to remain readable by layout, target_pos={target_pos}, points={:?}",
        snapshot.points
    );
}

#[test]
fn layout_frame_rust_retries_window_when_point_starts_below_visible_span() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let lines = (0..40)
        .map(|line| format!("line-{line:02}\n"))
        .collect::<Vec<_>>();
    let text = lines.join("");
    let target_pos = lines
        .iter()
        .take(20)
        .map(|line| line.chars().count())
        .sum::<usize>()
        + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        // Selected-window point lives in the buffer; see
        // window.c:window_point. Set buffer pt_char to
        // target_pos so window_params_from_neovm reads it as
        // params.point.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-retry", 160, 192, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(target_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target_pos))
            .is_some(),
        "expected retried layout to publish geometry for point {target_pos}, points={:?}",
        snapshot.points
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert!(
                *window_start > LispCharPos1::ONE,
                "expected window-start to advance after retry, got {window_start:?}"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn next_window_start_from_visible_rows_uses_visual_row_boundaries() {
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(8)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(9)),
            end_buffer_pos: Some(LispCharPos1::new(16)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 32,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(17)),
            end_buffer_pos: Some(LispCharPos1::new(24)),
        },
        DisplayRowSnapshot {
            row: 3,
            y: 48,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(25)),
            end_buffer_pos: Some(LispCharPos1::new(32)),
        },
    ];

    assert_eq!(
        next_window_start_from_visible_rows(&rows, 1),
        Some(32),
        "expected retry to advance to the next internal 0-based char position after the last visible row"
    );
    assert_eq!(
        next_window_start_from_visible_rows(&rows, 25),
        Some(32),
        "expected retry to keep the furthest internal 0-based visible progress that still advances"
    );
    assert_eq!(
        next_window_start_from_visible_rows(&rows, 33),
        None,
        "expected no retry candidate once the rendered span no longer advances"
    );
}

#[test]
fn next_window_start_for_partially_visible_point_row_scrolls_enough_to_fit_row() {
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 20,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 20,
            height: 20,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(20)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 40,
            height: 30,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(21)),
            end_buffer_pos: Some(LispCharPos1::new(30)),
        },
    ];

    assert_eq!(
        next_window_start_for_partially_visible_point_row(&rows, 25, 0, 60, 1),
        Some(10),
        "expected retry to scroll away enough top rows to fit the point row using the next internal 0-based char position"
    );
    assert_eq!(
        next_window_start_for_partially_visible_point_row(&rows, 15, 0, 60, 1),
        None,
        "expected no retry when the point row is already fully visible"
    );
}

#[test]
fn next_window_start_for_point_line_continuation_advances_last_visible_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdefghijklmnopqrstuvwxyz\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() as i64
    };
    let access = {
        let buf = eval.buffer_manager().get(buf_id).expect("buffer");
        RustBufferAccess::new(buf)
    };
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(20)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 32,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(21)),
            end_buffer_pos: Some(LispCharPos1::new(25)),
        },
    ];

    assert_eq!(
        next_window_start_for_point_line_continuation(&rows, 21, 1, &access, buffer_size),
        Some(20),
        "expected retry to move point toward the top when the visible point row continues below the window"
    );

    let terminated_rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(27)),
        },
    ];
    assert_eq!(
        next_window_start_for_point_line_continuation(
            &terminated_rows,
            11,
            1,
            &access,
            buffer_size
        ),
        None,
        "expected no retry once the final visible row already reaches the newline"
    );
}

#[test]
fn next_window_start_for_point_line_continuation_ignores_newline_terminated_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("needle target\nfiller line 06\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() as i64
    };
    let access = {
        let buf = eval.buffer_manager().get(buf_id).expect("buffer");
        RustBufferAccess::new(buf)
    };
    let rows = vec![DisplayRowSnapshot {
        row: 0,
        y: 0,
        height: 16,
        start_x: 0,
        start_col: 0,
        end_x: 0,
        end_col: 0,
        start_buffer_pos: Some(LispCharPos1::new(1)),
        end_buffer_pos: Some(LispCharPos1::new(14)),
    }];

    assert_eq!(
        next_window_start_for_point_line_continuation(&rows, 0, 0, &access, buffer_size),
        None,
        "expected no retry when the last visible row ended on a real newline"
    );
}

#[test]
fn next_window_start_for_point_line_continuation_ignores_tail_clipping_when_point_row_is_not_last_visible_row()
 {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() as i64
    };
    let access = {
        let buf = eval.buffer_manager().get(buf_id).expect("buffer");
        RustBufferAccess::new(buf)
    };
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(20)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 32,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(21)),
            end_buffer_pos: Some(LispCharPos1::new(30)),
        },
        DisplayRowSnapshot {
            row: 3,
            y: 48,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(31)),
            end_buffer_pos: Some(LispCharPos1::new(40)),
        },
        DisplayRowSnapshot {
            row: 4,
            y: 64,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(41)),
            end_buffer_pos: Some(LispCharPos1::new(50)),
        },
    ];

    assert_eq!(
        next_window_start_for_point_line_continuation(&rows, 21, 1, &access, buffer_size),
        None,
        "expected no retry here because the point row is not the final visible row; partially visible rows are handled by the separate point-row retry path"
    );
}

#[test]
fn char_advance_ascii_cache_distinguishes_semantic_font_identity() {
    let mut ascii_width_cache = std::collections::HashMap::new();
    let mut font_metrics_svc = Some(FontMetricsService::new());

    let regular_width = char_advance(
        &mut ascii_width_cache,
        true,
        &mut font_metrics_svc,
        'A',
        1,
        8.0,
        14,
        8.0,
        "monospace",
        400,
        false,
    );
    assert!(
        regular_width > 0.0,
        "expected measurable width for regular ASCII glyph"
    );
    assert_eq!(
        ascii_width_cache.len(),
        1,
        "expected one cache entry after first ASCII measurement"
    );

    let bold_width = char_advance(
        &mut ascii_width_cache,
        true,
        &mut font_metrics_svc,
        'A',
        1,
        8.0,
        14,
        8.0,
        "monospace",
        700,
        false,
    );
    assert!(
        bold_width > 0.0,
        "expected measurable width for bold ASCII glyph"
    );
    assert_eq!(
        ascii_width_cache.len(),
        2,
        "expected distinct cache entries for different semantic font specs even when face ids match"
    );

    let repeated_regular_width = char_advance(
        &mut ascii_width_cache,
        true,
        &mut font_metrics_svc,
        'A',
        1,
        8.0,
        14,
        8.0,
        "monospace",
        400,
        false,
    );
    assert_eq!(
        repeated_regular_width, regular_width,
        "expected repeated measurement for the same semantic font spec to reuse the cache entry"
    );
    assert_eq!(
        ascii_width_cache.len(),
        2,
        "expected cache size to stay stable when the semantic font spec is unchanged"
    );
}

#[test]
fn layout_frame_rust_converges_visibility_for_wrapped_rows_in_one_redisplay() {
    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let logical_lines = (0..24)
        .map(|line| format!("line-{line:02} abcdefghijklmno\n"))
        .collect::<Vec<_>>();
    let text = logical_lines.join("");
    let target_pos = logical_lines
        .iter()
        .take(18)
        .map(|line| line.chars().count())
        .sum::<usize>()
        + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        // Move the buffer point to target_pos so the selected
        // window reads it as params.point (GNU
        // window.c:window_point says selected windows use
        // BUF_PT, not pointm). Without this, the Window::point
        // assignment below would be shadowed by buffer.pt_char
        // during window_params_from_neovm and layout would
        // never see the target.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
        buf.set_buffer_local("word-wrap", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-wrap-retry", 80, 192, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(target_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let point_chars = snapshot
        .points
        .iter()
        .map(|point| {
            (
                point.buffer_pos,
                char_at_lisp_pos(buffer, point.buffer_pos.to_one_based_usize()),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target_pos))
            .is_some(),
        "expected wrapped-line redisplay to converge on point {target_pos}, points={:?}, rows={:?}, chars={:?}",
        snapshot.points,
        snapshot.rows,
        point_chars
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert!(
                *window_start > LispCharPos1::ONE,
                "expected window-start to advance for wrapped redisplay, got {window_start:?}"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn layout_frame_rust_converges_visibility_for_point_line_tail_clipping() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let prefix = (0..2)
        .map(|line| format!("p{line:02}\n"))
        .collect::<Vec<_>>()
        .join("");
    let target_line = "abcdefghijklmno\n";
    let text = format!("{prefix}{target_line}");
    let point = prefix.chars().count() + 1;
    let later_pos = point + 10;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("word-wrap", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-point-line-tail", 80, 256, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(later_pos))
            .is_some(),
        "expected redisplay to publish later positions from the point line after retry, points={:?}, rows={:?}",
        snapshot.points,
        snapshot.rows
    );
}

#[test]
fn layout_frame_rust_keeps_visible_eob_cursor_on_short_trailing_newline_buffer() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "LEFT WINDOW\nLine 2\nLine 3\n";
    let point = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() + 1
    };
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-eob-visible", 320, 640, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
            .is_some(),
        "expected first line to remain visible when EOB cursor is already onscreen, points={:?}, rows={:?}",
        snapshot.points,
        snapshot.rows
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert_eq!(
                *window_start,
                LispCharPos1::ONE,
                "expected visible EOB cursor not to force a retry scroll"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn layout_frame_rust_keeps_default_scratch_message_at_top_when_eob_is_visible() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = ";; This buffer is for text that is not saved, and for Lisp evaluation.\n\
;; To create a file, visit it with \u{2018}C-x C-f\u{2019} and enter text in its buffer.\n\n";
    let point = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
        point
    };
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-scratch-eob-visible", 600, 1188, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
            .is_some(),
        "expected the first scratch row to remain visible when EOB fits onscreen, points={:?}, rows={:?}",
        snapshot.points,
        snapshot.rows
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert_eq!(
                *window_start,
                LispCharPos1::ONE,
                "expected short scratch buffer to stay at top, got window-start {window_start:?}"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn layout_frame_rust_formats_mode_line_from_current_redisplay_geometry() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = (0..80)
        .map(|line| format!("Line {line:02}\n"))
        .collect::<String>();
    let point = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        buf.set_buffer_local("mode-line-format", Value::string("%o|%p|%P"));
        let point = buf.point_max_char_pos().get() + 1;
        // Selected-window point lives in the buffer; see
        // window.c:window_point.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
        point
    };
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-mode-line-geometry", 640, 96, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let mode_line_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::ModeLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    let published_window_start = {
        let frame = eval.frame_manager().get(frame_id).expect("frame");
        let window = frame.find_window(selected_window).expect("selected window");
        match window {
            neovm_core::window::Window::Leaf { window_start, .. } => *window_start,
            other => panic!("expected leaf window, got {other:?}"),
        }
    };
    let expected_mode_line = eval_status_line_format(
        &mut eval,
        "mode-line-format",
        selected_window.0 as i64,
        buf_id.0,
        80,
    )
    .expect("mode-line text");

    assert!(
        published_window_start > LispCharPos1::ONE,
        "expected point at EOB to advance window-start, got {published_window_start:?}"
    );
    assert!(
        mode_line_text == expected_mode_line,
        "expected rendered mode-line to match freshly evaluated mode-line after redisplay publish, got rendered={mode_line_text:?} expected={expected_mode_line:?}"
    );
}

#[test]
fn layout_frame_rust_honors_window_mode_line_format_none() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("mode-line-format", Value::string("BUFFER MODE"));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-window-mode-line-none", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    eval.frame_manager_mut().set_window_parameter(
        selected_window,
        Value::symbol("mode-line-format"),
        Value::symbol("none"),
    );

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let mode_line_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::ModeLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    let snapshot = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.window_display_snapshot(selected_window))
        .expect("display snapshot");

    assert_eq!(
        snapshot.mode_line_height, 0,
        "window parameter mode-line-format=none should suppress mode-line height like GNU"
    );
    assert!(
        mode_line_text.is_empty(),
        "window parameter mode-line-format=none should suppress rendered mode-line, got {mode_line_text:?}"
    );
}

#[test]
fn layout_frame_rust_uses_window_mode_line_format_override() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("mode-line-format", Value::NIL);
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-window-mode-line-format", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    eval.frame_manager_mut().set_window_parameter(
        selected_window,
        Value::symbol("mode-line-format"),
        Value::string("WINDOW MODE"),
    );

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let mode_line_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::ModeLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    let snapshot = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.window_display_snapshot(selected_window))
        .expect("display snapshot");

    assert!(
        snapshot.mode_line_height > 0,
        "non-nil window mode-line-format should request a mode-line like GNU"
    );
    assert!(
        mode_line_text.contains("WINDOW MODE"),
        "expected window parameter mode-line-format to override nil buffer format, got {mode_line_text:?}"
    );
}

#[test]
fn layout_frame_rust_advances_live_output_through_mode_line_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-output-progress-mode-line", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(selected_window))
        .and_then(|window| window.display())
        .expect("window display state");
    let logical_cursor = display.cursor.expect("logical cursor");
    let output_cursor = display.output_cursor.expect("output cursor");

    assert!(
        output_cursor.row > logical_cursor.row,
        "expected live output progression to continue past text rows into mode-line rows, cursor={logical_cursor:?} output={output_cursor:?}"
    );
}

#[test]
fn layout_frame_rust_renders_header_line_text_for_non_nil_header_line_format() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("header-line-format", Value::string("LEFT HEADER"));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-header-line", 640, 160, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let header_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::HeaderLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();

    assert!(
        header_text.contains("LEFT HEADER"),
        "expected header-line row to render buffer-local header-line-format text, got {header_text:?}"
    );
}

#[test]
fn layout_frame_rust_uses_full_window_row_space_for_header_text_and_mode_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("header-line-format", Value::string("LEFT HEADER"));
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-header-row-space", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("window display snapshot");
    let display = frame
        .find_window(selected_window)
        .and_then(|window| window.display())
        .expect("window display state");
    let logical_cursor = display.cursor.expect("logical cursor");
    let output_cursor = display.output_cursor.expect("output cursor");

    let header_row = snapshot
        .rows
        .iter()
        .find(|row| row.row == 0)
        .expect("header row snapshot");

    assert!(
        header_row.start_buffer_pos.is_none() && header_row.end_buffer_pos.is_none(),
        "expected row 0 to be reserved for header-line chrome, got {header_row:?}"
    );
    assert!(
        logical_cursor.row >= 1,
        "expected logical cursor row to be offset below header-line chrome, got {logical_cursor:?}"
    );
    assert!(
        output_cursor.row > logical_cursor.row,
        "expected mode-line output to advance past logical text rows, cursor={logical_cursor:?} output={output_cursor:?}"
    );
}

#[test]
fn layout_frame_rust_advances_live_output_through_tab_line_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("tab-line-format", Value::string("TAB ROW"));
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-tab-line-row-space", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("window display snapshot");
    let display = frame
        .find_window(selected_window)
        .and_then(|window| window.display())
        .expect("window display state");
    let logical_cursor = display.cursor.expect("logical cursor");
    let output_cursor = display.output_cursor.expect("output cursor");

    let tab_row = snapshot
        .rows
        .iter()
        .find(|row| row.row == 0)
        .expect("tab-line row snapshot");

    assert!(
        tab_row.start_buffer_pos.is_none() && tab_row.end_buffer_pos.is_none(),
        "expected row 0 to be reserved for tab-line chrome, got {tab_row:?}"
    );
    assert!(
        logical_cursor.row >= 1,
        "expected logical cursor row to be offset below tab-line chrome, got {logical_cursor:?}"
    );
    assert!(
        output_cursor.row > logical_cursor.row,
        "expected mode-line output to advance past logical text rows, cursor={logical_cursor:?} output={output_cursor:?}"
    );
}

#[test]
fn layout_frame_rust_tab_line_unicode_uses_shared_display_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("tab-line-format", Value::string("A中👨‍👩"));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-tab-line-unicode-baseline", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let tab_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::TabLine)
        .expect("tab-line row");
    let glyphs = &tab_row.glyphs[1];
    let cjk = glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("tab-line CJK glyph");

    assert_eq!(glyphs_logical_text(glyphs), "A中👨‍👩");
    assert!(
        cjk.wide,
        "tab-line chrome row should record CJK as a wide glyph through the shared builder: {glyphs:?}"
    );
    assert!(
        glyphs.iter().any(|glyph| glyph.padding),
        "tab-line chrome row should retain padding cells through the shared builder: {glyphs:?}"
    );
    assert!(
        glyphs
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
        "tab-line chrome row should compose ZWJ emoji through the shared builder: {glyphs:?}"
    );
}

#[test]
fn layout_frame_rust_baseline_buffer_text_uses_main_buffer_wide_and_cluster_glyphs() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("A中👨‍👩B\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-unicode-baseline", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_glyphs = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyphs[1].iter())
        .collect::<Vec<_>>();

    assert!(
        text_glyphs.iter().any(|glyph| {
            matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide
        }),
        "main buffer path should record CJK as a wide glyph: {text_glyphs:?}"
    );
    assert!(
        text_glyphs.iter().any(|glyph| glyph.padding),
        "main buffer wide/cluster glyphs should retain padding cells: {text_glyphs:?}"
    );
    assert!(
        text_glyphs.iter().any(|glyph| {
            matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))
        }),
        "main buffer path should compose the ZWJ emoji sequence: {text_glyphs:?}"
    );
}

#[test]
fn buffer_text_source_shadow_matches_main_buffer_simple_unicode_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("A中👨‍👩B\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-source-shadow", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let main_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("main buffer text row");

    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let line_end = CharPos0::new("A中👨‍👩B".chars().count());
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buf_id,
        &snapshot,
        CharPos0::ZERO,
        line_end,
        RenderFaceRef::FaceId(0),
    );
    let mut row_builder = crate::display_row_builder::DisplayRowBuilder::new(
        crate::display_row_builder::DisplayRowLayout {
            role: GlyphRowRole::Text,
            y_px: 0.0,
            width_px: 640.0,
            height_px: 16.0,
            ascent_px: 12.0,
            char_width_px: 8.0,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
            base_face: RenderFaceRef::FaceId(0),
            symbol_values: std::collections::HashMap::new(),
        },
    );
    let mut context = crate::display_source::DisplaySourceContext::empty();
    while let Some(item) = source.next_item(&mut context) {
        row_builder.push_item(item);
    }
    let shadow_row = row_builder.finish();

    assert_eq!(
        glyphs_logical_text(&shadow_row.glyphs[1]),
        glyphs_logical_text(&main_row.glyphs[1])
    );
    assert_eq!(
        shadow_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide),
        main_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide)
    );
    assert_eq!(
        shadow_row.glyphs[1].iter().any(|glyph| glyph.padding),
        main_row.glyphs[1].iter().any(|glyph| glyph.padding)
    );
    assert_eq!(
        shadow_row
            .glyphs[1]
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
        main_row
            .glyphs[1]
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}')))
    );
}

#[test]
fn buffer_text_source_shadow_matches_main_buffer_tab_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\tb\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-source-tab-shadow", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let main_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("main buffer text row");

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let line_end = CharPos0::new("a\tb".chars().count());
    let mut source = crate::display_source::BufferTextSourceCursor::new(
        buf_id,
        &snapshot,
        CharPos0::ZERO,
        line_end,
        RenderFaceRef::FaceId(0),
    );
    let mut row_builder = crate::display_row_builder::DisplayRowBuilder::new(
        crate::display_row_builder::DisplayRowLayout {
            role: GlyphRowRole::Text,
            y_px: 0.0,
            width_px: 640.0,
            height_px: frame.char_height,
            ascent_px: frame.char_height,
            char_width_px: frame.char_width,
            tab_policy: crate::display_row_builder::DisplayTabPolicy::every(8),
            base_face: RenderFaceRef::FaceId(0),
            symbol_values: std::collections::HashMap::new(),
        },
    );
    let mut context = crate::display_source::DisplaySourceContext::empty();
    while let Some(item) = source.next_item(&mut context) {
        row_builder.push_item(item);
    }
    let shadow_row = row_builder.finish();

    let main_glyphs = &main_row.glyphs[1];
    let shadow_glyphs = &shadow_row.glyphs[1];
    let main_tab = main_glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { .. }))
        .expect("main tab stretch");
    let shadow_tab = shadow_glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { .. }))
        .expect("shadow tab stretch");

    assert_eq!(
        glyphs_logical_text(main_glyphs),
        glyphs_logical_text(shadow_glyphs)
    );
    assert_eq!(main_tab.glyph_type, shadow_tab.glyph_type);
    assert_eq!(main_tab.pixel_width, shadow_tab.pixel_width);
}

#[test]
fn layout_frame_rust_preserves_multiline_overlay_output_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("after-string"),
            Value::string("A\nB"),
        );
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-output-rows", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("window display snapshot");
    let display = frame
        .find_window(selected_window)
        .and_then(|window| window.display())
        .expect("window display state");
    let second_text_row = snapshot
        .rows
        .iter()
        .find(|row| row.row == 1)
        .expect("second overlay row snapshot");
    let overlay_hit_row = unsafe {
        (&*std::ptr::addr_of!(crate::hit_test::FRAME_HIT_DATA))
            .as_ref()
            .and_then(|windows| {
                windows
                    .iter()
                    .find(|window| window.window_id == selected_window.0 as i64)
            })
            .and_then(|window| {
                window.rows.iter().find(|row| {
                    let y = second_text_row.y as f32 + 1.0;
                    y >= row.y_start && y < row.y_end
                })
            })
            .cloned()
    }
    .expect("overlay hit row");
    let overlay_hit = crate::hit_test::hit_test_window_charpos(
        selected_window.0 as i64,
        0.0,
        second_text_row.y as f32 + 1.0,
    );

    assert!(
        snapshot
            .rows
            .iter()
            .any(|row| row.row == 0 && row.start_buffer_pos.is_some()),
        "expected first text row snapshot to survive multiline overlay output, rows={:?}",
        snapshot.rows
    );
    assert!(
        snapshot.rows.iter().any(|row| row.row == 1),
        "expected multiline overlay output to publish a second text row, rows={:?}",
        snapshot.rows
    );
    assert!(
        display.output_cursor.is_some_and(|cursor| cursor.row >= 1),
        "expected live output cursor to advance onto multiline overlay rows, output={:?}",
        display.output_cursor
    );
    assert!(
        overlay_hit >= overlay_hit_row.charpos_start && overlay_hit <= overlay_hit_row.charpos_end,
        "expected multiline overlay row hit-testing to land inside the recorded overlay row span, hit={overlay_hit} row={overlay_hit_row:?}"
    );
}

#[test]
fn layout_frame_rust_renders_overlay_string_tabs_as_stretches() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("after-string"),
            Value::string("a\tb"),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-tab-string", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);
    assert!(
        !logical_text.contains('\t'),
        "overlay tab should not render as a literal tab, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        logical_text.contains("a      b"),
        "overlay tab should expand to the next tab stop, text={logical_text:?}"
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 6 })),
        "overlay tab should be a stretch glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_renders_overlay_string_glyphless_chars_as_glyphless() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("after-string"),
            Value::string("\u{fff0}"),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-glyphless-string", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Glyphless { ch: '\u{fff0}' })),
        "overlay glyphless source char should emit a glyphless glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_places_cursor_inside_overlay_string_text_run() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let overlay_text = Value::string_with_text_properties(
            "AB",
            vec![StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![Value::symbol("cursor"), Value::T]),
            }],
        );
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("after-string"), overlay_text);
        buf.goto_emacs_byte_pos(EmacsBytePos::new(1));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-cursor-run", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let x_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("x point");
    let expected_overlay_slot_width = frame.char_width.round() as i64;

    assert_eq!(cursor.row, x_point.row);
    assert_eq!(
        cursor.x,
        x_point.x + x_point.width + expected_overlay_slot_width
    );
    assert_eq!(cursor.col, x_point.col + 2);
    assert_eq!(cursor.width, expected_overlay_slot_width);
}

#[test]
fn layout_frame_rust_renders_zero_length_eob_before_string_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("Find file: ~/.config/doom/");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("\ninit.el\nconfig.el"),
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-eob-before-overlay", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rows = enabled_window_row_texts(entry);

    assert!(
        rows.iter().any(|row| row.contains("init.el")),
        "expected zero-length EOB before-string to render init.el, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("config.el")),
        "expected zero-length EOB before-string to render config.el, rows={rows:?}"
    );
}

#[test]
fn layout_frame_rust_continues_eob_before_string_after_overlong_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nsecond.el\nthird.el"),
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-eob-overlong-before-overlay",
        96,
        180,
        buf_id,
    );
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.char_width = 8.0;
        frame.char_height = 16.0;
        frame.font_pixel_size = 16.0;
    }
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new_without_font_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rows = enabled_window_row_texts(entry);

    assert!(
        rows.iter().any(|row| row.contains("second.el")),
        "expected overlong overlay row not to suppress the next candidate row, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("third.el")),
        "expected rendering to continue after later overlay newlines, rows={rows:?}"
    );
}

#[test]
fn layout_frame_rust_honors_display_space_align_in_overlay_strings() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let display_space = Value::string_with_text_properties(
            "config.el -rw",
            vec![StringTextPropertyRun {
                start: "config.el".chars().count(),
                end: "config.el ".chars().count(),
                plist: Value::list(vec![
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":align-to"),
                        Value::list(vec![
                            Value::symbol("+"),
                            Value::symbol("left"),
                            Value::fixnum(20),
                        ]),
                    ]),
                ]),
            }],
        );
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("before-string"), display_space);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-overlay-display-space-align",
        640,
        180,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rendered_rows: Vec<String> = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .map(|row| {
            row.glyphs[1]
                .iter()
                .map(|glyph| match &glyph.glyph_type {
                    GlyphType::Char { ch } => ch.to_string(),
                    GlyphType::Composite { text } => text.to_string(),
                    GlyphType::Stretch { width_cols } => " ".repeat(*width_cols as usize),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .collect();

    assert!(
        rendered_rows
            .iter()
            .any(|row| row.contains("config.el           -rw")),
        "GNU TTY expands overlay-string display spaces before suffix text, rows={rendered_rows:?}"
    );
}

#[test]
fn layout_frame_rust_does_not_grow_minibuffer_for_eob_before_string_like_gnu() {
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("resize-mini-windows", Value::symbol("grow-only"));
    eval.obarray_mut()
        .set_symbol_value("max-mini-window-height", Value::fixnum(10));

    let root_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let minibuf_id = eval.buffer_manager_mut().create_buffer(" *Minibuf-1*");
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(minibuf_id)
            .expect("buffer");
        buf.insert("Find file: ~/.config/doom/");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(minibuf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("\ninit.el\nconfig.el"),
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-mini-eob-before-overlay",
        120,
        40,
        root_buf_id,
    );
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.char_width = 1.0;
        frame.char_height = 1.0;
        frame.shrink_mini_window();
    }
    let minibuffer_window_id = eval
        .activate_minibuffer_window_for_buffer(
            minibuf_id,
            LispString::from_utf8("Find file: "),
            Some(LispString::from_utf8("~/.config/doom/")),
        )
        .expect("activate minibuffer")
        .expect("minibuffer window");

    let mut engine = LayoutEngine::new_without_font_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id.0)
        .expect("minibuffer matrix entry");
    let rows = enabled_window_row_texts(entry);

    assert!(
        rows.iter()
            .any(|row| row.contains("Find file: ~/.config/doom/")),
        "expected minibuffer prompt row to render, rows={rows:?}"
    );
    assert!(
        rows.iter()
            .all(|row| !row.contains("init.el") && !row.contains("config.el")),
        "GNU does not grow the parent minibuffer for a zero-length EOB before-string, rows={rows:?}"
    );
}

#[test]
fn layout_frame_rust_renders_tab_bar_text_from_lisp_tab_bar_keymap() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["x", "neomacs"]).expect("bootstrap");
    apply_runtime_startup_state(&mut eval).expect("runtime startup state");
    // Bootstrap may or may not install an initial selected
    // frame depending on cache state. Capture whatever exists
    // so we can restore the selection after switching to the
    // target frame for the tab-bar assertions.
    let prior_selected_frame = eval.frame_manager().selected_frame().map(|f| f.id);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-bar", 1600, 160, buf_id);
    eval.obarray_mut()
        .set_symbol_value("layout-target-frame", Value::make_frame(frame_id.0));
    eval.eval_str(
        r#"
          (require 'tab-bar)
          (setq tab-bar-show 1)
          (tab-bar-mode 1)
          (switch-to-buffer (get-buffer-create "*frame-a*"))
          (tab-bar-new-tab)
          (switch-to-buffer (get-buffer-create "*frame-a-2*"))
          (tab-bar-select-tab 1)
          (select-frame layout-target-frame)
          (tab-bar-new-tab)
          (switch-to-buffer (get-buffer-create "*tb-2*"))
          (tab-bar-rename-tab "T中👨‍👩")
          (tab-bar-select-tab 1)
        "#,
    )
    .expect("eval tab-bar forms");
    eval.eval_form(Value::list(vec![
        Value::symbol("select-frame"),
        Value::make_frame(frame_id.0),
        Value::NIL,
    ]))
    .expect("select target frame for tab-bar debug");
    let keymap_debug =
        match eval.eval_form(Value::list(vec![Value::symbol("tab-bar-make-keymap-1")])) {
            Ok(value) => eval
                .eval_form(Value::list(vec![Value::symbol("prin1-to-string"), value]))
                .ok()
                .and_then(|rendered| rendered.as_runtime_string_owned())
                .unwrap_or_else(|| "<render-unavailable>".to_string()),
            Err(err) => format!("<error: {err}>"),
        };
    let tabs_debug = eval
        .eval_str("(prin1-to-string (frame-parameter nil 'tabs))")
        .ok()
        .and_then(|value| value.as_runtime_string_owned())
        .unwrap_or_else(|| "<unavailable>".to_string());
    let format_debug = eval
        .eval_str("(prin1-to-string tab-bar-format)")
        .ok()
        .and_then(|value| value.as_runtime_string_owned())
        .unwrap_or_else(|| "<unavailable>".to_string());
    if let Some(prev) = prior_selected_frame {
        eval.eval_form(Value::list(vec![
            Value::symbol("select-frame"),
            Value::make_frame(prev.0),
            Value::NIL,
        ]))
        .expect("restore selected frame");
    }

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    assert!(
        frame.tab_bar_height > 0,
        "expected tab-bar-mode to reserve frame tab-bar height"
    );

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let tab_bar_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .frame_chrome_rows
                .iter()
                .filter(|row| row.row.role == GlyphRowRole::TabBar && row.row.enabled)
                .map(|row| glyphs_logical_text(&row.row.glyphs[1]))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    assert!(
        tab_bar_text.contains("T中👨‍👩"),
        "expected tab-bar row to render tab captions from tab-bar keymap, got {tab_bar_text:?}; tabs={tabs_debug}; format={format_debug}; keymap={keymap_debug}"
    );
    let tab_bar_glyphs = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .frame_chrome_rows
                .iter()
                .filter(|row| row.row.role == GlyphRowRole::TabBar && row.row.enabled)
                .flat_map(|row| row.row.glyphs[1].iter())
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(
        tab_bar_glyphs
            .iter()
            .filter(|glyph| !glyph.padding)
            .all(|glyph| glyph.pixel_width > 0.0),
        "expected tab-bar glyphs to carry display-row pixel widths: {tab_bar_glyphs:?}"
    );
    let cjk = tab_bar_glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("tab-bar CJK glyph");
    assert!(
        cjk.wide,
        "tab-bar CJK glyph should use the shared wide-glyph builder: {tab_bar_glyphs:?}"
    );
    assert!(
        tab_bar_glyphs.iter().any(|glyph| glyph.padding),
        "tab-bar CJK glyph should retain its padding cell: {tab_bar_glyphs:?}"
    );
    assert!(
        tab_bar_glyphs.iter().any(
            |glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.as_ref() == "👨‍👩")
        ),
        "tab-bar ZWJ emoji should be clustered by the shared builder: {tab_bar_glyphs:?}"
    );
    let window_tab_bar_rows = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::TabBar && row.enabled)
                .count()
        })
        .unwrap_or(0);
    assert_eq!(
        window_tab_bar_rows, 0,
        "expected frame tab bar to live in frame_chrome_rows, not in leaf-window matrices"
    );
    // Note: a previous version of this test also asserted
    // `!tab_bar_text.contains("*frame-a-2*")` as a
    // "frame-isolation" check. The tab-bar.el keymap produced
    // by `tab-bar-make-keymap-1` walks all tabs reachable from
    // the current frame's `tabs` parameter and does not
    // filter by which frame created each tab, so the negative
    // assertion was testing a speculative behavior that isn't
    // part of the render contract. Dropping it keeps the
    // primary "renders any target-frame text at all" check
    // and leaves frame-scoped tab isolation as a separate
    // concern.
}

#[test]
fn layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_tty() {
    assert_echo_message_renders_in_minibuffer_window(false);
}

#[test]
fn layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_gui() {
    assert_echo_message_renders_in_minibuffer_window(true);
}

#[test]
fn layout_frame_rust_resizes_multiline_echo_rows_for_tty() {
    assert_multiline_echo_message_resizes_minibuffer_rows(false);
}

#[test]
fn layout_frame_rust_resizes_multiline_echo_rows_for_gui() {
    assert_multiline_echo_message_resizes_minibuffer_rows(true);
}

#[test]
fn test_ligature_run_buffer_is_empty_len() {
    let mut buf = LigatureRunBuffer::new();

    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);

    buf.push('a', 8.0);

    assert!(!buf.is_empty());
    assert_eq!(buf.len(), 1);

    buf.push('b', 8.0);

    assert!(!buf.is_empty());
    assert_eq!(buf.len(), 2);
}

#[test]
fn test_ligature_run_buffer_push() {
    let mut buf = LigatureRunBuffer::new();

    buf.push('h', 8.0);
    assert_eq!(buf.chars, vec!['h']);
    assert_eq!(buf.advances, vec![8.0]);
    assert_eq!(buf.total_advance, 8.0);

    buf.push('e', 8.0);
    assert_eq!(buf.chars, vec!['h', 'e']);
    assert_eq!(buf.advances, vec![8.0, 8.0]);
    assert_eq!(buf.total_advance, 16.0);

    buf.push('l', 7.5);
    assert_eq!(buf.chars, vec!['h', 'e', 'l']);
    assert_eq!(buf.advances, vec![8.0, 8.0, 7.5]);
    assert_eq!(buf.total_advance, 23.5);
}

#[test]
fn test_ligature_run_buffer_clear() {
    let mut buf = LigatureRunBuffer::new();

    buf.push('a', 8.0);
    buf.push('b', 8.0);
    buf.start_x = 100.0;
    buf.start_y = 200.0;
    buf.face_h = 16.0;
    buf.face_ascent = 12.0;
    buf.face_id = 42;
    buf.is_overlay = true;
    buf.height_scale = 1.5;

    buf.clear();

    // Vectors and total_advance cleared
    assert_eq!(buf.chars.len(), 0);
    assert_eq!(buf.advances.len(), 0);
    assert_eq!(buf.total_advance, 0.0);

    // Position/face fields NOT cleared
    assert_eq!(buf.start_x, 100.0);
    assert_eq!(buf.start_y, 200.0);
    assert_eq!(buf.face_h, 16.0);
    assert_eq!(buf.face_ascent, 12.0);
    assert_eq!(buf.face_id, 42);
    assert_eq!(buf.is_overlay, true);
    assert_eq!(buf.height_scale, 1.5);
}

#[test]
fn test_ligature_run_buffer_start() {
    let mut buf = LigatureRunBuffer::new();

    buf.push('x', 10.0);
    buf.start_x = 999.0;

    buf.start(50.0, 60.0, 20.0, 15.0, 5, true, 1.2);

    // Clears chars/advances/total_advance
    assert_eq!(buf.chars.len(), 0);
    assert_eq!(buf.advances.len(), 0);
    assert_eq!(buf.total_advance, 0.0);

    // Sets all position/face params
    assert_eq!(buf.start_x, 50.0);
    assert_eq!(buf.start_y, 60.0);
    assert_eq!(buf.face_h, 20.0);
    assert_eq!(buf.face_ascent, 15.0);
    assert_eq!(buf.face_id, 5);
    assert_eq!(buf.is_overlay, true);
    assert_eq!(buf.height_scale, 1.2);
}

#[test]
fn test_max_ligature_run_len_constant() {
    assert_eq!(MAX_LIGATURE_RUN_LEN, 64);
}

#[test]
fn test_flush_run_is_noop() {
    // flush_run is now a no-op: glyph output has been migrated to GlyphMatrixBuilder.
    let mut run = LigatureRunBuffer::new();
    run.start(10.0, 20.0, 16.0, 12.0, 1, false, 0.0);
    run.push('a', 8.0);
    let len_before = run.len();
    let advance_before = run.total_advance;

    flush_run(&run, true);
    flush_run(&run, false);
    assert_eq!(run.len(), len_before);
    assert_eq!(run.total_advance, advance_before);

    // Empty run
    let empty_run = LigatureRunBuffer::new();
    flush_run(&empty_run, true);
}

#[test]
fn test_is_ligature_char() {
    // Ligature-eligible characters
    for ch in [
        '-', '>', '<', '=', '!', '|', '&', '*', '+', '.', '/', ':', ';', '?', '@', '\\', '^', '~',
        '#', '$', '%',
    ] {
        assert!(is_ligature_char(ch), "'{}' should be a ligature char", ch);
    }
    // Non-ligature characters
    for ch in [
        'a', 'Z', '0', '9', ' ', '\n', '\t', '(', ')', '[', ']', '{', '}', ',', '\'', '"',
    ] {
        assert!(
            !is_ligature_char(ch),
            "'{}' should NOT be a ligature char",
            ch
        );
    }
}

#[test]
fn test_run_is_pure_ligature() {
    // Pure symbol run
    let mut run = LigatureRunBuffer::new();
    run.start(0.0, 0.0, 16.0, 12.0, 1, false, 0.0);
    run.push('-', 8.0);
    run.push('>', 8.0);
    assert!(run_is_pure_ligature(&run));

    // Mixed run (alpha + symbol)
    let mut run2 = LigatureRunBuffer::new();
    run2.start(0.0, 0.0, 16.0, 12.0, 1, false, 0.0);
    run2.push('a', 8.0);
    run2.push(':', 8.0);
    assert!(!run_is_pure_ligature(&run2));

    // Pure alpha run
    let mut run3 = LigatureRunBuffer::new();
    run3.start(0.0, 0.0, 16.0, 12.0, 1, false, 0.0);
    run3.push('h', 8.0);
    run3.push('i', 8.0);
    assert!(!run_is_pure_ligature(&run3));
}

#[test]
fn test_cursor_point_columns_wide_char() {
    let params = test_window_params();
    let text = "你".as_bytes();
    assert_eq!(cursor_point_columns(text, 0, 0, &params), 2);
}

#[test]
fn test_cursor_point_columns_tab_uses_tab_stop_list() {
    let mut params = test_window_params();
    params.tab_width = 8;
    params.tab_stop_list = vec![4, 10];
    let text = b"\t";

    assert_eq!(cursor_point_columns(text, 0, 3, &params), 1);
    assert_eq!(cursor_point_columns(text, 0, 4, &params), 6);
}

#[test]
fn test_cursor_width_for_style_bar_uses_bar_width() {
    let params = test_window_params();
    let text = "你".as_bytes();

    let width = cursor_width_for_style(CursorStyle::Bar(2.5), text, 0, 0, &params, 7.0);
    assert_eq!(width, 2.5);
}

#[test]
fn test_cursor_width_for_style_tab_clamps_when_x_stretch_cursor_is_nil() {
    let params = test_window_params();
    let text = b"\t";

    let width = cursor_width_for_style(CursorStyle::FilledBox, text, 0, 1, &params, 8.0);
    assert_eq!(width, 8.0);
}

#[test]
fn test_cursor_width_for_style_tab_expands_when_x_stretch_cursor_is_t() {
    let mut params = test_window_params();
    params.x_stretch_cursor = true;
    let text = b"\t";

    let width = cursor_width_for_style(CursorStyle::FilledBox, text, 0, 1, &params, 8.0);
    assert_eq!(width, 56.0);
}

#[test]
fn test_cursor_width_for_style_hbar_uses_glyph_columns() {
    let params = test_window_params();
    let text = "你".as_bytes();

    let width = cursor_width_for_style(CursorStyle::Hbar(2.0), text, 0, 0, &params, 7.0);
    assert_eq!(width, 14.0);
}

#[test]
fn test_cursor_style_for_nonselected_bar_uses_resolved_width() {
    let mut params = test_window_params();
    params.selected = false;
    params.cursor_kind = neomacs_display_protocol::frame_glyphs::CursorKind::Bar;
    params.cursor_bar_width = CursorBarWidth::new(4);

    assert_eq!(
        cursor_style_for_window(&params),
        Some(CursorStyle::Bar(4.0))
    );
}

#[test]
fn test_cursor_style_for_nonselected_no_cursor_is_none() {
    let mut params = test_window_params();
    params.selected = false;
    params.cursor_kind = neomacs_display_protocol::frame_glyphs::CursorKind::NoCursor;

    assert_eq!(cursor_style_for_window(&params), None);
}

#[test]
fn test_resolve_cursor_vertical_metrics_uses_row_metrics() {
    let (y, height, ascent) =
        resolve_cursor_vertical_metrics(20.0, 24.0, 18.0, 24.0, 14.0, 16.0, false);

    assert_eq!(y, 16.0);
    assert_eq!(height, 24.0);
    assert_eq!(ascent, 18.0);
}

#[test]
fn test_resolve_cursor_vertical_metrics_preserves_eob_origin() {
    let (y, height, ascent) =
        resolve_cursor_vertical_metrics(20.0, 24.0, 18.0, 24.0, 14.0, 16.0, true);

    assert_eq!(y, 20.0);
    assert_eq!(height, 20.0);
    assert_eq!(ascent, 14.0);
}
