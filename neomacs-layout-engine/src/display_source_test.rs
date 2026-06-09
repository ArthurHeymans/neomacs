use super::*;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayLength, DisplayLengthExpr,
    DisplayLengthSymbol, DisplaySourceId, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, DisplayTextRun, DisplayXwidgetItem, GlyphlessMethod, RenderFaceRef,
};
use crate::neovm_bridge::{LayoutBufferSnapshot, LayoutBufferView};
use neovm_core::buffer::{BufferId, CharPos0, EmacsBytePos, EmacsByteRange};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::emacs_core::{Context, Value};

fn collect_items(source: &mut impl DisplayItemSource) -> Vec<DisplayItem> {
    let mut context = DisplaySourceContext::empty();
    let mut items = Vec::new();
    while let Some(item) = source.next_item(&mut context) {
        items.push(item);
    }
    items
}

fn item_texts(items: &[DisplayItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match &item.kind {
            DisplayItemKind::TextRun(run) => Some(run.text.to_string()),
            _ => None,
        })
        .collect()
}

fn snapshot_with_text(text: &str) -> (BufferId, LayoutBufferSnapshot, CharPos0) {
    let mut eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval
            .buffer_manager_mut()
            .get_mut(buffer_id)
            .expect("buffer");
        buffer.insert(text);
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    let end = buffer.total_char_end_pos();
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    (buffer_id, snapshot, end)
}

#[test]
fn buffer_display_replacement_source_builds_items_without_appending() {
    let source = BufferDisplayReplacementSource::new(BufferId(7), 3, 12);

    let stretch_item = source.stretch_item(42, DisplayReplacementBox::new(16.0, 9.0, 7.0));
    assert_eq!(stretch_item.face, RenderFaceRef::FaceId(42));
    assert!(matches!(
        stretch_item.kind,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(16.0)),
            height: Some(DisplayLength::Pixels(9.0)),
            ascent: Some(DisplayLength::Pixels(7.0)),
        })
    ));

    let text_item = source.source_mapped_text_item(43, "fallback");
    assert_eq!(text_item.face, RenderFaceRef::FaceId(43));
    assert!(matches!(
        text_item.kind,
        DisplayItemKind::SourceMappedText(text) if text.text.as_ref() == "fallback"
    ));
}

#[test]
fn buffer_display_replacement_string_source_maps_text_to_buffer_slot() {
    let _eval = Context::new();
    let replacement_source = BufferDisplayReplacementSource::new(BufferId(7), 3, 12);
    let string_source =
        LispStringSourceCursor::new(1, Value::string("fallback"), RenderFaceRef::FaceId(42))
            .expect("string source");
    let mut source = BufferDisplayReplacementStringSource::new(replacement_source, string_source);
    let mut context = DisplaySourceContext::empty();

    let item = source.next_item(&mut context).expect("replacement item");

    assert_eq!(item.face, RenderFaceRef::FaceId(42));
    assert_eq!(
        item.span.start,
        DisplaySourcePosition::buffer(BufferId(7), CharPos0::new(3), EmacsBytePos::new(12))
    );
    assert_eq!(
        item.span.end,
        DisplaySourcePosition::buffer(BufferId(7), CharPos0::new(4), EmacsBytePos::new(12))
    );
    assert!(matches!(
        item.kind,
        DisplayItemKind::SourceMappedText(text) if text.text.as_ref() == "fallback"
    ));
    assert!(source.next_item(&mut context).is_none());
}

#[test]
fn lisp_string_source_cursor_emits_text_runs_with_source_spans() {
    let _eval = Context::new();
    let value = Value::string("abc");
    let mut source =
        LispStringSourceCursor::new(1, value, RenderFaceRef::FaceId(3)).expect("string source");

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["abc"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(3));
    assert_eq!(
        items[0].span.start,
        DisplaySourcePosition::lisp_string(1, 0, 0)
    );
    assert_eq!(
        items[0].span.end,
        DisplaySourcePosition::lisp_string(1, 3, 3)
    );
}

struct SymbolFaceResolver;

impl DisplayItemFaceResolver for SymbolFaceResolver {
    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef {
        match face_value.as_symbol_name() {
            Some("bold") => RenderFaceRef::FaceId(7),
            Some("font-lock-string-face") => RenderFaceRef::FaceId(9),
            _ => base,
        }
    }
}

#[test]
fn lisp_string_source_cursor_resolves_face_property() {
    let _eval = Context::new();
    let value = Value::string_with_text_properties(
        "abc",
        vec![StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![Value::symbol("face"), Value::symbol("bold")]),
        }],
    );
    let mut source =
        LispStringSourceCursor::new(2, value, RenderFaceRef::FaceId(3)).expect("string source");
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let mut items = Vec::new();
    while let Some(item) = source.next_item(&mut context) {
        items.push(item);
    }

    assert_eq!(item_texts(&items), ["a", "b", "c"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(3));
    assert_eq!(items[1].face, RenderFaceRef::FaceId(7));
    assert_eq!(items[2].face, RenderFaceRef::FaceId(3));
}

#[test]
fn lisp_string_source_cursor_uses_font_lock_face_when_face_is_absent() {
    let _eval = Context::new();
    let value = Value::string_with_text_properties(
        "xy",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![
                Value::symbol("font-lock-face"),
                Value::symbol("font-lock-string-face"),
            ]),
        }],
    );
    let mut source =
        LispStringSourceCursor::new(3, value, RenderFaceRef::FaceId(3)).expect("string source");
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let first = source.next_item(&mut context).expect("first item");
    let second = source.next_item(&mut context).expect("second item");

    assert_eq!(
        first.kind,
        DisplayItemKind::TextRun(DisplayTextRun::new("x"))
    );
    assert_eq!(first.face, RenderFaceRef::FaceId(9));
    assert_eq!(
        second.kind,
        DisplayItemKind::TextRun(DisplayTextRun::new("y"))
    );
    assert_eq!(second.face, RenderFaceRef::FaceId(3));
}

#[test]
fn lisp_string_source_cursor_parses_display_space_width_as_stretch() {
    let _eval = Context::new();
    let value = Value::string_with_text_properties(
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
    let mut source =
        LispStringSourceCursor::new(4, value, RenderFaceRef::FaceId(3)).expect("string source");

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "b"]);
    assert_eq!(
        items[1].kind,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Em(3.0)),
            height: None,
            ascent: None,
        })
    );
}

#[test]
fn lisp_string_source_cursor_parses_display_space_align_to_as_typed_expression() {
    let _eval = Context::new();
    let value = Value::string_with_text_properties(
        " ",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![
                Value::symbol("display"),
                Value::list(vec![
                    Value::symbol("space"),
                    Value::keyword(":align-to"),
                    Value::list(vec![
                        Value::symbol("-"),
                        Value::symbol("right"),
                        Value::fixnum(2),
                    ]),
                ]),
            ]),
        }],
    );
    let mut source =
        LispStringSourceCursor::new(5, value, RenderFaceRef::FaceId(3)).expect("string source");

    let item = source
        .next_item(&mut DisplaySourceContext::empty())
        .expect("item");

    assert_eq!(
        item.kind,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::AlignTo(DisplayLengthExpr::Sub(vec![
                DisplayLengthExpr::Symbol(DisplayLengthSymbol::Right),
                DisplayLengthExpr::Em(2.0),
            ])),
            height: None,
            ascent: None,
        })
    );
}

#[test]
fn lisp_string_source_cursor_emits_explicit_newline_row_breaks() {
    let _eval = Context::new();
    let value = Value::string("a\nb");
    let mut source =
        LispStringSourceCursor::new(6, value, RenderFaceRef::FaceId(3)).expect("string source");
    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "b"]);
    assert!(matches!(items[1].kind, DisplayItemKind::RowBreak(_)));
}

#[test]
fn lisp_string_source_cursor_emits_control_and_glyphless_items() {
    let _eval = Context::new();
    let value = Value::string("a\u{0001}\u{fff0}b");
    let mut source =
        LispStringSourceCursor::new(7, value, RenderFaceRef::FaceId(3)).expect("string source");

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "b"]);
    assert_eq!(
        items[1].kind,
        DisplayItemKind::ControlChar { ch: '\u{0001}' }
    );
    assert_eq!(
        items[2].kind,
        DisplayItemKind::Glyphless(DisplayGlyphless {
            ch: '\u{fff0}',
            method: GlyphlessMethod::HexCode,
        })
    );
}

#[test]
fn lisp_string_source_cursor_pushes_display_string_replacement_source() {
    let _eval = Context::new();
    let replacement = Value::string("YZ");
    let value = Value::string_with_text_properties(
        "axb",
        vec![StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![Value::symbol("display"), replacement]),
        }],
    );
    let mut source =
        LispStringSourceCursor::new(7, value, RenderFaceRef::FaceId(3)).expect("string source");

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "YZ", "b"]);
    let DisplaySourcePosition::LispString { source_id, .. } = items[1].span.start else {
        panic!("replacement text should come from a Lisp string source");
    };
    assert_ne!(
        source_id,
        DisplaySourceId::new(7),
        "replacement string should be emitted from a nested source frame, not flattened into the parent span"
    );
}

#[test]
fn display_sources_parse_xwidget_display_specs_as_typed_items() {
    let mut eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let xwidget = Value::make_xwidget(
        Value::symbol("webkit"),
        Value::string("Title"),
        Value::make_buffer(buffer_id),
        96,
        54,
        1234,
    );
    let display_spec = Value::list(vec![
        Value::symbol("xwidget"),
        Value::keyword("xwidget"),
        xwidget,
    ]);

    let mut lisp_source = LispStringSourceCursor::new(
        8,
        Value::string_with_text_properties(
            "x",
            vec![StringTextPropertyRun {
                start: 0,
                end: 1,
                plist: Value::list(vec![Value::symbol("display"), display_spec]),
            }],
        ),
        RenderFaceRef::FaceId(3),
    )
    .expect("string source");
    let lisp_items = collect_items(&mut lisp_source);

    assert_eq!(
        lisp_items[0].kind,
        DisplayItemKind::Xwidget(DisplayXwidgetItem {
            xwidget_id: 1234,
            width: 96.0,
            height: 54.0,
        })
    );

    {
        let buffer = eval
            .buffer_manager_mut()
            .get_mut(buffer_id)
            .expect("buffer");
        buffer.insert("x");
        let start = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(0));
        let end = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(1));
        buffer.text_props_put_property_in_emacs_byte_range(
            EmacsByteRange::new(start, end),
            Value::symbol("display"),
            display_spec,
        );
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    let end = buffer.total_char_end_pos();
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let mut buffer_source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::FaceId(3),
    );
    let buffer_items = collect_items(&mut buffer_source);

    assert_eq!(buffer_items[0].kind, lisp_items[0].kind);
}

#[test]
fn buffer_text_source_cursor_emits_text_runs_with_buffer_spans() {
    let (buffer_id, snapshot, end) = snapshot_with_text("ab中");
    let mut source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::FaceId(3),
    );

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["ab中"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(3));
    assert_eq!(
        items[0].span.start,
        DisplaySourcePosition::buffer(
            buffer_id,
            CharPos0::new(0),
            neovm_core::buffer::EmacsBytePos::new(0)
        )
    );
    assert_eq!(
        items[0].span.end,
        DisplaySourcePosition::buffer(
            buffer_id,
            CharPos0::new(3),
            snapshot.layout_char_pos_to_emacs_byte_pos(CharPos0::new(3))
        )
    );
}

#[test]
fn buffer_text_source_cursor_resolves_face_property_runs() {
    let mut eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval
            .buffer_manager_mut()
            .get_mut(buffer_id)
            .expect("buffer");
        buffer.insert("abc");
        let start = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(1));
        let end = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(2));
        buffer.text_props_put_property_in_emacs_byte_range(
            EmacsByteRange::new(start, end),
            Value::symbol("face"),
            Value::symbol("bold"),
        );
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    let end = buffer.total_char_end_pos();
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let mut source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::FaceId(3),
    );
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let mut items = Vec::new();
    while let Some(item) = source.next_item(&mut context) {
        items.push(item);
    }

    assert_eq!(item_texts(&items), ["a", "b", "c"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(3));
    assert_eq!(items[1].face, RenderFaceRef::FaceId(7));
    assert_eq!(items[2].face, RenderFaceRef::FaceId(3));
}

#[test]
fn buffer_text_source_cursor_pushes_display_string_replacement_source() {
    let mut eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval
            .buffer_manager_mut()
            .get_mut(buffer_id)
            .expect("buffer");
        buffer.insert("axb");
        let start = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(1));
        let end = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(2));
        buffer.text_props_put_property_in_emacs_byte_range(
            EmacsByteRange::new(start, end),
            Value::symbol("display"),
            Value::string("YZ"),
        );
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    let end = buffer.total_char_end_pos();
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let mut source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::FaceId(3),
    );

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "YZ", "b"]);
    let DisplaySourcePosition::LispString { .. } = items[1].span.start else {
        panic!("replacement text should be emitted from a nested Lisp string source");
    };
}

#[test]
fn buffer_text_source_cursor_reports_nested_replacement_source_position() {
    let mut eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buffer = eval
            .buffer_manager_mut()
            .get_mut(buffer_id)
            .expect("buffer");
        buffer.insert("x");
        let start = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(0));
        let end = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(1));
        let replacement = Value::string_with_text_properties(
            "YZ",
            vec![StringTextPropertyRun {
                start: 0,
                end: 1,
                plist: Value::list(vec![Value::symbol("face"), Value::symbol("bold")]),
            }],
        );
        buffer.text_props_put_property_in_emacs_byte_range(
            EmacsByteRange::new(start, end),
            Value::symbol("display"),
            replacement,
        );
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    let end = buffer.total_char_end_pos();
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let mut source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::FaceId(3),
    );
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let item = source
        .next_item(&mut context)
        .expect("first replacement item");

    assert_eq!(
        item.kind,
        DisplayItemKind::TextRun(DisplayTextRun::new("Y"))
    );
    assert!(matches!(
        source.source_position(),
        DisplaySourcePosition::LispString { char_index: 1, .. }
    ));
}

#[test]
fn buffer_text_source_cursor_emits_explicit_newline_row_breaks() {
    let (buffer_id, snapshot, end) = snapshot_with_text("a\nb");
    let mut source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::FaceId(3),
    );

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "b"]);
    assert!(matches!(items[1].kind, DisplayItemKind::RowBreak(_)));
}

#[test]
fn buffer_text_source_cursor_emits_control_and_glyphless_items() {
    let (buffer_id, snapshot, end) = snapshot_with_text("a\u{0001}\u{200b}b");
    let mut source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::FaceId(3),
    );

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "b"]);
    assert_eq!(
        items[1].kind,
        DisplayItemKind::ControlChar { ch: '\u{0001}' }
    );
    assert_eq!(
        items[2].kind,
        DisplayItemKind::Glyphless(DisplayGlyphless {
            ch: '\u{200b}',
            method: GlyphlessMethod::ZeroWidth,
        })
    );
}
