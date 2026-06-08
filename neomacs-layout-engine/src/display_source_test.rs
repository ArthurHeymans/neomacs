use super::*;
use crate::display_item::{
    DisplayGlyphless, DisplayItem, DisplayItemKind, DisplayLength, DisplayLengthExpr,
    DisplayLengthSymbol, DisplaySourceId, DisplaySourcePosition, DisplayStretch,
    DisplayStretchWidth, DisplayTextRun, GlyphlessMethod, RenderFaceRef,
};
use crate::neovm_bridge::{LayoutBufferSnapshot, LayoutBufferView};
use neovm_core::buffer::{BufferId, CharPos0, EmacsByteRange};
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
