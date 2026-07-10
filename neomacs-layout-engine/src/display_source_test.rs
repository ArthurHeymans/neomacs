use super::*;
use crate::display_buffer_source_consumption::{
    BufferSourceConsumedItem, BufferSourceConsumptionState,
};
use crate::display_buffer_text_source::BufferTextSourceCursor;
use crate::display_item::{
    BufferDisplayReplacementSource, DisplayGlyphless, DisplayImageItem, DisplayItem,
    DisplayItemKind, DisplayLength, DisplayLengthExpr, DisplayLengthSymbol,
    DisplayMediaReplacement, DisplayRowBreakReason, DisplaySourceId, DisplaySourceMappedText,
    DisplaySourcePosition, DisplayStretch, DisplayStretchWidth, DisplayTextRun, GlyphlessMethod,
    RenderFaceRef, SourceSpan,
};
use crate::display_property::DisplayReplacementProperty;
use crate::display_source::DisplaySourceTextPosition;
use crate::neovm_bridge::{LayoutBufferSnapshot, LayoutBufferView};
use neomacs_display_protocol::types::FaceId;
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
            DisplayItemKind::SourceMappedText(text) => Some(text.text.to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn display_source_step_item_splits_text_run_at_buffer_charpos() {
    let buffer_id = BufferId(7);
    let item = DisplayItem::new(
        SourceSpan::new(
            DisplaySourcePosition::buffer(buffer_id, CharPos0::new(5), EmacsBytePos::new(110)),
            DisplaySourcePosition::buffer(buffer_id, CharPos0::new(8), EmacsBytePos::new(115)),
        ),
        RenderFaceRef::FaceId(FaceId::new(3)),
        DisplayItemKind::TextRun(DisplayTextRun::new("éβx")),
    );
    let source_item = DisplaySourceItem::new_for_test(item, 10, 5, Some('é'));
    let step_item = DisplaySourceStepItem::new(source_item, 100).expect("step item");

    let (prefix, suffix) = step_item
        .split_text_run_at_charpos(7, 100)
        .expect("split text run");
    let (_prefix_step, prefix_item) = prefix.into_test_render_parts().expect("prefix parts");
    let (_suffix_step, suffix_item) = suffix.into_test_render_parts().expect("suffix parts");

    let DisplayItemKind::TextRun(prefix_run) = &prefix_item.kind else {
        panic!("expected prefix text run");
    };
    let DisplayItemKind::TextRun(suffix_run) = &suffix_item.kind else {
        panic!("expected suffix text run");
    };
    assert_eq!(&*prefix_run.text, "éβ");
    assert_eq!(&*suffix_run.text, "x");
    assert_eq!(
        prefix_item.span.end,
        DisplaySourcePosition::buffer(buffer_id, CharPos0::new(7), EmacsBytePos::new(114))
    );
    assert_eq!(
        suffix_item.span.start,
        DisplaySourcePosition::buffer(buffer_id, CharPos0::new(7), EmacsBytePos::new(114))
    );
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

fn expected_source_coords(text: &str) -> Vec<(char, usize, i64)> {
    let mut byte_offset = 0usize;
    let mut charpos = 0i64;
    text.chars()
        .map(|ch| {
            let source = (ch, byte_offset, charpos);
            byte_offset += ch.len_utf8();
            charpos += 1;
            source
        })
        .collect()
}

#[test]
fn text_source_char_classification_matches_display_items() {
    assert_eq!(
        classify_text_source_char('\n'),
        TextSourceCharClassification::RowBreak
    );
    assert_eq!(
        classify_text_source_char('\u{7f}'),
        TextSourceCharClassification::ControlChar { ch: '\u{7f}' }
    );
    assert_eq!(
        classify_text_source_char('\u{feff}'),
        TextSourceCharClassification::Glyphless {
            ch: '\u{feff}',
            method: GlyphlessMethod::ZeroWidth,
        }
    );
    assert_eq!(
        classify_text_source_char('\t'),
        TextSourceCharClassification::Text
    );
    assert_eq!(
        classify_text_source_char('x'),
        TextSourceCharClassification::Text
    );
}

#[test]
fn buffer_display_replacement_source_builds_items_without_appending() {
    let source =
        BufferDisplayReplacementSource::new(BufferId(7), CharPos0::new(3), EmacsBytePos::new(12));

    let stretch_item = source.display_item(
        FaceId::new(42),
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(16.0)),
            height: Some(DisplayLength::Pixels(9.0)),
            ascent: Some(DisplayLength::Pixels(7.0)),
        }),
    );
    assert_eq!(stretch_item.face, RenderFaceRef::FaceId(FaceId::new(42)));
    assert!(matches!(
        stretch_item.kind,
        DisplayItemKind::Stretch(DisplayStretch {
            width: DisplayStretchWidth::Length(DisplayLength::Pixels(16.0)),
            height: Some(DisplayLength::Pixels(9.0)),
            ascent: Some(DisplayLength::Pixels(7.0)),
        })
    ));

    let text_item = source.display_item(
        FaceId::new(43),
        DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new("fallback")),
    );
    assert_eq!(text_item.face, RenderFaceRef::FaceId(FaceId::new(43)));
    assert!(matches!(
        text_item.kind,
        DisplayItemKind::SourceMappedText(text) if text.text.as_ref() == "fallback"
    ));
}

#[test]
fn buffer_display_replacement_source_can_span_covered_buffer_text() {
    let source = BufferDisplayReplacementSource::spanning(
        BufferId(7),
        CharPos0::new(3),
        EmacsBytePos::new(12),
        CharPos0::new(5),
        EmacsBytePos::new(18),
    );

    let text_item = source.display_item(
        FaceId::new(43),
        DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new("fallback")),
    );

    assert_eq!(
        text_item.span.start,
        DisplaySourcePosition::buffer(BufferId(7), CharPos0::new(3), EmacsBytePos::new(12))
    );
    assert_eq!(
        text_item.span.end,
        DisplaySourcePosition::buffer(BufferId(7), CharPos0::new(5), EmacsBytePos::new(18))
    );
}

#[test]
fn buffer_text_item_source_single_char_maps_one_buffer_character() {
    let source = BufferTextItemSource::single_char(
        BufferId(7),
        CharPos0::new(3),
        EmacsBytePos::new(12),
        EmacsBytePos::new(16),
    );

    let item = source.item(
        RenderFaceRef::FaceId(FaceId::new(42)),
        DisplayItemKind::TextRun(DisplayTextRun::new("x")),
    );

    assert_eq!(
        item.span.start,
        DisplaySourcePosition::buffer(BufferId(7), CharPos0::new(3), EmacsBytePos::new(12))
    );
    assert_eq!(
        item.span.end,
        DisplaySourcePosition::buffer(BufferId(7), CharPos0::new(4), EmacsBytePos::new(16))
    );
}

#[test]
fn buffer_display_replacement_string_source_maps_text_to_buffer_slot() {
    let _eval = Context::new();
    let replacement_source =
        BufferDisplayReplacementSource::new(BufferId(7), CharPos0::new(3), EmacsBytePos::new(12));
    let string_source = LispStringSourceCursor::new(
        1,
        Value::string("fallback"),
        RenderFaceRef::FaceId(FaceId::new(42)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");
    let mut source = BufferDisplayReplacementStringSource::new(replacement_source, string_source);
    let mut context = DisplaySourceContext::empty();

    let item = source.next_item(&mut context).expect("replacement item");

    assert_eq!(item.face, RenderFaceRef::FaceId(FaceId::new(42)));
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
fn buffer_display_replacement_string_source_ignores_display_properties_inside_replacement_string() {
    let _eval = Context::new();
    let replacement_source =
        BufferDisplayReplacementSource::new(BufferId(7), CharPos0::new(3), EmacsBytePos::new(12));
    let value = Value::string_with_text_properties(
        "Y",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![Value::symbol("display"), Value::string("Z")]),
        }],
    );
    let mut source = BufferDisplayReplacementStringRequest::new(1, value, replacement_source)
        .into_source(FaceId::new(42))
        .expect("replacement string source");

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["Y"]);
}

#[test]
fn lisp_string_source_cursor_emits_text_runs_with_source_spans() {
    let _eval = Context::new();
    let value = Value::string("abc");
    let mut source = LispStringSourceCursor::new(
        1,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["abc"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(FaceId::new(3)));
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
            Some("bold") => RenderFaceRef::FaceId(FaceId::new(7)),
            Some("font-lock-string-face") => RenderFaceRef::FaceId(FaceId::new(9)),
            _ => base,
        }
    }
}

struct ResolvedDisplayPropertyResolver {
    seen_face: Option<RenderFaceRef>,
}

impl DisplayItemFaceResolver for ResolvedDisplayPropertyResolver {
    fn resolve_face_ref(&mut self, base: RenderFaceRef, face_value: Value) -> RenderFaceRef {
        match face_value.as_symbol_name() {
            Some("bold") => RenderFaceRef::FaceId(FaceId::new(7)),
            _ => base,
        }
    }

    fn resolve_display_media_replacement(
        &mut self,
        display_prop: Value,
        face: RenderFaceRef,
    ) -> Option<DisplayMediaReplacement> {
        self.seen_face = Some(face);
        if display_prop.cons_car().is_symbol_named("image") {
            Some(DisplayMediaReplacement::image(DisplayImageItem {
                image_id: 42,
                width: 64.0,
                height: 32.0,
            }))
        } else {
            None
        }
    }
}

#[test]
fn display_property_source_action_classifies_strings_typed_items_and_resolver_fallback() {
    let _eval = Context::new();
    let base_face = RenderFaceRef::FaceId(FaceId::new(7));
    let mut resolver = ResolvedDisplayPropertyResolver { seen_face: None };

    {
        let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

        let display_string = DisplayPropertySourcePlan::new(Value::string("displayed"));
        match display_string.source_action(&mut context, base_face) {
            DisplayPropertySourceAction::PushReplacement { value, base_face } => {
                assert_eq!(
                    value.as_runtime_string_owned().as_deref(),
                    Some("displayed")
                );
                assert_eq!(base_face, RenderFaceRef::FaceId(FaceId::new(7)));
            }
            action => panic!("expected replacement string action, got {action:?}"),
        }

        let space_spec = Value::list(vec![
            Value::symbol("space"),
            Value::keyword(":width"),
            Value::fixnum(2),
        ]);
        let space_plan = DisplayPropertySourcePlan::new(space_spec);
        match space_plan.source_action(&mut context, base_face) {
            DisplayPropertySourceAction::Emit {
                kind:
                    DisplayItemKind::Stretch(DisplayStretch {
                        width: DisplayStretchWidth::Length(DisplayLength::Em(2.0)),
                        height: None,
                        ascent: None,
                    }),
                layout,
            } => assert_eq!(layout, DisplayItemLayout::default()),
            action => panic!("expected typed space action, got {action:?}"),
        }

        let image_plan = DisplayPropertySourcePlan::new(Value::list(vec![Value::symbol("image")]));
        match image_plan.source_action(&mut context, base_face) {
            DisplayPropertySourceAction::Emit {
                kind:
                    DisplayItemKind::MediaReplacement(DisplayMediaReplacement {
                        width: 64.0,
                        height: 32.0,
                        ..
                    }),
                layout,
            } => assert_eq!(layout, DisplayItemLayout::default()),
            action => panic!("expected resolved image action, got {action:?}"),
        }
    }

    assert_eq!(resolver.seen_face, Some(base_face));
}

#[test]
fn display_property_source_replacement_resolves_direct_media_item() {
    let _eval = Context::new();
    let media = DisplayMediaReplacement::xwidget(crate::display_item::DisplayXwidgetItem {
        xwidget_id: 21,
        width: 30.0,
        height: 12.0,
    });
    let mut context = DisplaySourceContext::empty();
    let replacement_property = crate::display_property::DisplayReplacementProperty::Media(
        crate::display_property::DisplayMediaReplacementProperty::Xwidget(media),
    );

    let replacement = DisplayPropertySourceReplacement::resolve(
        &mut context,
        Value::NIL,
        Some(&replacement_property),
        RenderFaceRef::FaceId(FaceId::new(7)),
    );

    let DisplayPropertySourceReplacement::Item(DisplayItemKind::MediaReplacement(resolved)) =
        replacement
    else {
        panic!("expected direct media replacement item");
    };
    assert_eq!(resolved, media);
}

#[test]
fn display_property_source_action_builds_cursor_actions() {
    let span = SourceSpan::synthetic(3, 0, 1);
    let face = RenderFaceRef::FaceId(FaceId::new(7));
    let layout = DisplayItemLayout {
        raise: Some(0.25),
        height: None,
    };

    let emit = DisplayPropertySourceAction::Emit {
        kind: DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new("x")),
        layout,
    }
    .into_cursor_action(span.clone(), face);

    let DisplayPropertySourceCursorAction::Emit(item) = emit else {
        panic!("expected emitted cursor item");
    };
    assert_eq!(item.span, span);
    assert_eq!(item.face, face);
    assert_eq!(item.layout, layout);

    let fallthrough = DisplayPropertySourceAction::Ignore { layout }.into_cursor_action(span, face);
    assert_eq!(
        fallthrough,
        DisplayPropertySourceCursorAction::FallThrough { layout }
    );
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
    let mut source = LispStringSourceCursor::new(
        2,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let mut items = Vec::new();
    while let Some(item) = source.next_item(&mut context) {
        items.push(item);
    }

    assert_eq!(item_texts(&items), ["a", "b", "c"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(FaceId::new(3)));
    assert_eq!(items[1].face, RenderFaceRef::FaceId(FaceId::new(7)));
    assert_eq!(items[2].face, RenderFaceRef::FaceId(FaceId::new(3)));
}

#[test]
fn lisp_string_source_cursor_resolves_display_property_through_context() {
    let _eval = Context::new();
    let display_spec = Value::list(vec![Value::symbol("image")]);
    let value = Value::string_with_text_properties(
        "x",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::symbol("bold"),
                Value::symbol("display"),
                display_spec,
            ]),
        }],
    );
    let mut source = LispStringSourceCursor::new(
        4,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");
    let mut resolver = ResolvedDisplayPropertyResolver { seen_face: None };

    let item = {
        let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);
        let item = source.next_item(&mut context).expect("display item");
        assert!(source.next_item(&mut context).is_none());
        item
    };

    assert_eq!(
        resolver.seen_face,
        Some(RenderFaceRef::FaceId(FaceId::new(7)))
    );
    assert!(matches!(
        item.kind,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement {
            width: 64.0,
            height: 32.0,
            ..
        })
    ));
    assert_eq!(item.face, RenderFaceRef::FaceId(FaceId::new(7)));
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
    let mut source = LispStringSourceCursor::new(
        3,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let first = source.next_item(&mut context).expect("first item");
    let second = source.next_item(&mut context).expect("second item");

    assert_eq!(
        first.kind,
        DisplayItemKind::TextRun(DisplayTextRun::new("x"))
    );
    assert_eq!(first.face, RenderFaceRef::FaceId(FaceId::new(9)));
    assert_eq!(
        second.kind,
        DisplayItemKind::TextRun(DisplayTextRun::new("y"))
    );
    assert_eq!(second.face, RenderFaceRef::FaceId(FaceId::new(3)));
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
    let mut source = LispStringSourceCursor::new(
        4,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");

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
    let mut source = LispStringSourceCursor::new(
        5,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");

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
    let mut source = LispStringSourceCursor::new(
        6,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");
    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "b"]);
    assert!(matches!(items[1].kind, DisplayItemKind::RowBreak(_)));
}

#[test]
fn lisp_string_source_cursor_emits_control_and_glyphless_items() {
    let _eval = Context::new();
    let value = Value::string("a\u{0001}\u{fff0}b");
    let mut source = LispStringSourceCursor::new(
        7,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");

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
    let mut source = LispStringSourceCursor::new(
        7,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");

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
fn lisp_string_source_cursor_ignores_display_properties_inside_display_string_replacement() {
    let _eval = Context::new();
    let replacement = Value::string_with_text_properties(
        "Y",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![Value::symbol("display"), Value::string("Z")]),
        }],
    );
    let value = Value::string_with_text_properties(
        "x",
        vec![StringTextPropertyRun {
            start: 0,
            end: 1,
            plist: Value::list(vec![Value::symbol("display"), replacement]),
        }],
    );
    let mut source = LispStringSourceCursor::new(
        7,
        value,
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["Y"]);
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
        RenderFaceRef::FaceId(FaceId::new(3)),
        LispStringSourceOrigin::Normal,
    )
    .expect("string source");
    let lisp_items = collect_items(&mut lisp_source);

    assert!(matches!(
        lisp_items[0].kind,
        DisplayItemKind::MediaReplacement(DisplayMediaReplacement {
            width: 96.0,
            height: 54.0,
            ..
        })
    ));

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
        RenderFaceRef::FaceId(FaceId::new(3)),
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
        RenderFaceRef::FaceId(FaceId::new(3)),
    );

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["ab中"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(FaceId::new(3)));
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
        RenderFaceRef::FaceId(FaceId::new(3)),
    );
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let mut items = Vec::new();
    while let Some(item) = source.next_item(&mut context) {
        items.push(item);
    }

    assert_eq!(item_texts(&items), ["a", "b", "c"]);
    assert_eq!(items[0].face, RenderFaceRef::FaceId(FaceId::new(3)));
    assert_eq!(items[1].face, RenderFaceRef::FaceId(FaceId::new(7)));
    assert_eq!(items[2].face, RenderFaceRef::FaceId(FaceId::new(3)));
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
        RenderFaceRef::FaceId(FaceId::new(3)),
    );

    let items = collect_items(&mut source);

    assert_eq!(item_texts(&items), ["a", "YZ", "b"]);
    assert!(matches!(
        items[1].kind,
        DisplayItemKind::SourceMappedText(_)
    ));
    assert_eq!(
        items[1].span.start,
        DisplaySourcePosition::buffer(buffer_id, CharPos0::new(1), EmacsBytePos::new(1))
    );
    assert_eq!(
        items[1].span.end,
        DisplaySourcePosition::buffer(buffer_id, CharPos0::new(2), EmacsBytePos::new(2))
    );
}

#[test]
fn buffer_text_source_cursor_emits_propertized_display_string_as_atomic_replacement() {
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
        RenderFaceRef::FaceId(FaceId::new(3)),
    );
    let mut context = DisplaySourceContext::empty();
    let mut source_consumption = BufferSourceConsumptionState::new(0);
    let mut position = DisplaySourceTextPosition::new(0, 0);

    let Some(first) =
        source_consumption.next_source_consumption_item(&mut source, &mut context, &mut position)
    else {
        panic!("expected leading text step");
    };
    let first = first.into_renderable().expect("leading renderable item");
    let end_charpos = first.end_charpos();
    let (_, first_item) = first.into_test_render_parts().expect("render parts");
    assert_eq!(item_texts(std::slice::from_ref(&first_item)), ["a"]);
    position = DisplaySourceTextPosition::new(position.byte_idx(), end_charpos);

    let Some(replacement_item) =
        source_consumption.next_source_consumption_item(&mut source, &mut context, &mut position)
    else {
        panic!("expected atomic replacement string item");
    };
    let BufferSourceConsumedItem::DisplayPropertyReplacement(replacement) = replacement_item else {
        panic!("expected replacement item kind");
    };

    assert_eq!(replacement.start_byte_idx(0), Some(1));
    assert_eq!(replacement.start_charpos(), 1);
    assert_eq!(replacement.descriptor().skip_to_charpos(), 2);
    assert_eq!(replacement.descriptor().value().as_utf8_str(), Some("YZ"));
}

#[test]
fn buffer_text_source_cursor_emits_display_space_as_atomic_replacement() {
    let mut eval = Context::new();
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let display_space = Value::list(vec![
        Value::symbol("space"),
        Value::keyword(":width"),
        Value::fixnum(2),
    ]);
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
            display_space,
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
        RenderFaceRef::FaceId(FaceId::new(3)),
    );
    let mut context = DisplaySourceContext::empty();
    let mut source_consumption = BufferSourceConsumptionState::new(0);
    let mut position = DisplaySourceTextPosition::new(0, 0);

    let Some(first) =
        source_consumption.next_source_consumption_item(&mut source, &mut context, &mut position)
    else {
        panic!("expected leading text step");
    };
    let first = first.into_renderable().expect("leading renderable item");
    let end_charpos = first.end_charpos();
    let (_, first_item) = first.into_test_render_parts().expect("render parts");
    assert_eq!(item_texts(std::slice::from_ref(&first_item)), ["a"]);
    position = DisplaySourceTextPosition::new(position.byte_idx(), end_charpos);

    let Some(replacement_item) =
        source_consumption.next_source_consumption_item(&mut source, &mut context, &mut position)
    else {
        panic!("expected atomic display space item");
    };
    let BufferSourceConsumedItem::DisplayPropertyReplacement(replacement) = replacement_item else {
        panic!("expected replacement item kind");
    };

    assert_eq!(replacement.start_byte_idx(0), Some(1));
    assert_eq!(replacement.start_charpos(), 1);
    assert_eq!(replacement.descriptor().skip_to_charpos(), 2);
    assert!(matches!(
        replacement.descriptor().classification().replacement(),
        Some(DisplayReplacementProperty::Stretch(_))
    ));
}

#[test]
fn buffer_text_source_consumption_keeps_plain_text_run_renderable() {
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
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let mut source = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        buffer.total_char_end_pos(),
        RenderFaceRef::FaceId(FaceId::new(3)),
    );
    let mut context = DisplaySourceContext::empty();
    let mut source_consumption = BufferSourceConsumptionState::new(0);
    let mut position = DisplaySourceTextPosition::new(0, 0);

    let consumed = source_consumption
        .next_source_consumption_item(&mut source, &mut context, &mut position)
        .expect("renderable text run");
    let renderable = consumed.into_renderable().expect("renderable item");
    let (_, item) = renderable.into_test_render_parts().expect("render parts");

    assert_eq!(item_texts(&[item]), ["abc"]);
    assert_eq!(position, DisplaySourceTextPosition::new(3, 0));
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
        RenderFaceRef::FaceId(FaceId::new(3)),
    );
    let mut resolver = SymbolFaceResolver;
    let mut context = DisplaySourceContext::with_face_resolver(&mut resolver);

    let item = source
        .next_item(&mut context)
        .expect("first replacement item");

    assert_eq!(
        item.kind,
        DisplayItemKind::SourceMappedText(DisplaySourceMappedText::new("Y"))
    );
    assert_eq!(
        item.span.start,
        DisplaySourcePosition::buffer(buffer_id, CharPos0::new(0), EmacsBytePos::new(0))
    );
    assert_eq!(
        item.span.end,
        DisplaySourcePosition::buffer(buffer_id, CharPos0::new(1), EmacsBytePos::new(1))
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
        RenderFaceRef::FaceId(FaceId::new(3)),
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
        RenderFaceRef::FaceId(FaceId::new(3)),
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

#[test]
fn typed_buffer_source_events_match_expected_plain_text_coordinates() {
    let text = "abc\ndef\tghi\n";
    let (buffer_id, snapshot, end) = snapshot_with_text(text);
    let mut cursor = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::Inherit,
    );
    let mut context = DisplaySourceContext::empty();

    let mut typed = Vec::new();
    let mut byte_offset = 0usize;
    let mut charpos = 0i64;
    while let Some(item) = cursor.next_item(&mut context) {
        match item.kind {
            DisplayItemKind::TextRun(run) => {
                for ch in run.text.chars() {
                    typed.push((ch, byte_offset, charpos));
                    byte_offset += ch.len_utf8();
                    charpos += 1;
                }
            }
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline =>
            {
                typed.push(('\n', byte_offset, charpos));
                byte_offset += 1;
                charpos += 1;
            }
            DisplayItemKind::ControlChar { ch }
            | DisplayItemKind::Glyphless(DisplayGlyphless { ch, .. }) => {
                typed.push((ch, byte_offset, charpos));
                byte_offset += ch.len_utf8();
                charpos += 1;
            }
            other => panic!("unexpected display item for plain text: {:?}", other),
        }
    }

    assert_eq!(typed, expected_source_coords(text));
}

#[test]
fn typed_buffer_source_events_match_expected_control_and_glyphless_coordinates() {
    let text = "abc\u{0001}def\u{200b}ghi\n";
    let (buffer_id, snapshot, end) = snapshot_with_text(text);
    let mut cursor = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::Inherit,
    );
    let mut context = DisplaySourceContext::empty();

    let mut typed = Vec::new();
    let mut byte_offset = 0usize;
    let mut charpos = 0i64;
    while let Some(item) = cursor.next_item(&mut context) {
        match item.kind {
            DisplayItemKind::TextRun(run) => {
                for ch in run.text.chars() {
                    typed.push((ch, byte_offset, charpos));
                    byte_offset += ch.len_utf8();
                    charpos += 1;
                }
            }
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline =>
            {
                typed.push(('\n', byte_offset, charpos));
                byte_offset += 1;
                charpos += 1;
            }
            DisplayItemKind::ControlChar { ch }
            | DisplayItemKind::Glyphless(DisplayGlyphless { ch, .. }) => {
                typed.push((ch, byte_offset, charpos));
                byte_offset += ch.len_utf8();
                charpos += 1;
            }
            other => panic!("unexpected display item: {:?}", other),
        }
    }

    assert_eq!(typed, expected_source_coords(text));
}

#[test]
fn typed_buffer_source_events_match_expected_face_property_coordinates() {
    let text = "abc\ndef\n";
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
        let start = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(1));
        let end = buffer.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(3));
        buffer.text_props_put_property_in_emacs_byte_range(
            EmacsByteRange::new(start, end),
            Value::symbol("face"),
            Value::symbol("bold"),
        );
    }
    let buffer = eval.buffer_manager().get(buffer_id).expect("buffer");
    let end = buffer.total_char_end_pos();
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);

    let mut cursor = BufferTextSourceCursor::new(
        buffer_id,
        &snapshot,
        CharPos0::ZERO,
        end,
        RenderFaceRef::Inherit,
    );
    let mut context = DisplaySourceContext::empty();

    let mut typed = Vec::new();
    let mut byte_offset = 0usize;
    let mut charpos = 0i64;
    while let Some(item) = cursor.next_item(&mut context) {
        match item.kind {
            DisplayItemKind::TextRun(run) => {
                for ch in run.text.chars() {
                    typed.push((ch, byte_offset, charpos));
                    byte_offset += ch.len_utf8();
                    charpos += 1;
                }
            }
            DisplayItemKind::RowBreak(row_break)
                if row_break.reason == DisplayRowBreakReason::ExplicitNewline =>
            {
                typed.push(('\n', byte_offset, charpos));
                byte_offset += 1;
                charpos += 1;
            }
            DisplayItemKind::ControlChar { ch }
            | DisplayItemKind::Glyphless(DisplayGlyphless { ch, .. }) => {
                typed.push((ch, byte_offset, charpos));
                byte_offset += ch.len_utf8();
                charpos += 1;
            }
            other => panic!("unexpected display item: {:?}", other),
        }
    }

    assert_eq!(typed, expected_source_coords(text));
}
