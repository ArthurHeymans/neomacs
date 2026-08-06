use super::*;
use crate::buffer_source::text_source::BufferTextSourceCursor;
use crate::display_source::{DisplayItemSource, DisplaySourceContext};
use neovm_core::buffer::CharLen;
use neovm_core::emacs_core::Context;

fn buffer_with_text(eval: &mut Context, text: &str) -> BufferId {
    let buf_id = eval.buffer_manager_mut().create_buffer("*row-route*");
    eval.buffer_manager_mut()
        .get_mut(buf_id)
        .expect("buffer")
        .insert(text);
    buf_id
}

fn plain_policy() -> RowRouteWindowPolicy {
    RowRouteWindowPolicy {
        // Far outside any test row.
        point_charpos: 1_000,
        hscroll_active: false,
        selective_display: 0,
        word_wrap: false,
        show_trailing_whitespace: false,
    }
}

fn row_start(text: &[u8], byte_idx: usize, charpos: i64) -> RowRouteRowStart<'_> {
    RowRouteRowStart {
        text,
        byte_idx,
        charpos,
        text_start_byte: 0,
    }
}

fn wide_fit() -> RowRouteFit {
    RowRouteFit {
        start_x_px: 0.0,
        char_width_px: 8.0,
        right_edge_px: 640.0,
    }
}

fn classify_in_buffer(
    eval: &Context,
    buf_id: BufferId,
    row: RowRouteRowStart<'_>,
    fit: RowRouteFit,
    policy: RowRouteWindowPolicy,
) -> RowAcquisitionRoute {
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    classify_row_acquisition(buffer, row, fit, policy)
}

#[test]
fn classifier_routes_plain_ascii_row_to_item_renderer() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello world\n");
    let text = b"hello world\n";
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 0, 0),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::ItemRenderer
    );
}

#[test]
fn classifier_routes_trailing_whitespace_row_when_highlight_off() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "ab  \n");
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"ab  \n", 0, 0),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::ItemRenderer
    );
}

#[test]
fn classifier_rejects_content_the_item_route_does_not_cover() {
    let mut eval = Context::new();
    // Non-ASCII, tab, control char, missing final newline, empty line: all
    // stay on the buffer pipeline.
    for text in [
        "h\u{00e9}llo\n".as_bytes(),
        b"a\tb\n".as_slice(),
        b"a\x01b\n".as_slice(),
        b"hello".as_slice(),
        b"\nx\n".as_slice(),
    ] {
        let buf_id = buffer_with_text(&mut eval, std::str::from_utf8(text).unwrap());
        assert_eq!(
            classify_in_buffer(
                &eval,
                buf_id,
                row_start(text, 0, 0),
                wide_fit(),
                plain_policy()
            ),
            RowAcquisitionRoute::BufferPipeline,
            "content {:?} must stay on the buffer pipeline",
            String::from_utf8_lossy(text)
        );
    }
}

#[test]
fn classifier_rejects_mid_line_start() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abc\n");
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"abc\n", 1, 1),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline
    );
}

#[test]
fn classifier_rejects_line_exactly_filling_the_row() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abcd\n");
    // 4 chars * 8px == the full 32px row: exact fill is NOT eligible.
    let exact = RowRouteFit {
        start_x_px: 0.0,
        char_width_px: 8.0,
        right_edge_px: 32.0,
    };
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"abcd\n", 0, 0),
            exact,
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline
    );
    // One cell of slack routes.
    let slack = RowRouteFit {
        start_x_px: 0.0,
        char_width_px: 8.0,
        right_edge_px: 40.0,
    };
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"abcd\n", 0, 0),
            slack,
            plain_policy()
        ),
        RowAcquisitionRoute::ItemRenderer
    );
}

#[test]
fn classifier_rejects_rows_containing_point() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abc\nxyz\n");
    let text = b"abc\nxyz\n";
    for point in 0..=3 {
        let policy = RowRouteWindowPolicy {
            point_charpos: point,
            ..plain_policy()
        };
        assert_eq!(
            classify_in_buffer(&eval, buf_id, row_start(text, 0, 0), wide_fit(), policy),
            RowAcquisitionRoute::BufferPipeline,
            "point {point} lies on the row (incl. its newline)"
        );
    }
    // Point on the NEXT line does not disqualify this row.
    let policy = RowRouteWindowPolicy {
        point_charpos: 4,
        ..plain_policy()
    };
    assert_eq!(
        classify_in_buffer(&eval, buf_id, row_start(text, 0, 0), wide_fit(), policy),
        RowAcquisitionRoute::ItemRenderer
    );
}

#[test]
fn classifier_rejects_window_policy_features() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abc\n");
    let text = b"abc\n";
    let policies = [
        RowRouteWindowPolicy {
            hscroll_active: true,
            ..plain_policy()
        },
        RowRouteWindowPolicy {
            selective_display: 2,
            ..plain_policy()
        },
        RowRouteWindowPolicy {
            word_wrap: true,
            ..plain_policy()
        },
        RowRouteWindowPolicy {
            show_trailing_whitespace: true,
            ..plain_policy()
        },
    ];
    for policy in policies {
        assert_eq!(
            classify_in_buffer(&eval, buf_id, row_start(text, 0, 0), wide_fit(), policy),
            RowAcquisitionRoute::BufferPipeline,
            "policy {policy:?} must stay on the buffer pipeline"
        );
    }
}

#[test]
fn classifier_rejects_text_properties_and_overlays() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\nworld\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 1 3 'face 'bold)")
        .expect("put-text-property");
    let text = b"hello\nworld\n";
    // Faced first row rejected; the property also bounds the change scan so
    // probing exercises both arms.
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 0, 0),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline
    );
    // The second, unfaced row still routes.
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 6, 6),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::ItemRenderer
    );

    eval.eval_str("(overlay-put (make-overlay 8 10) 'face 'highlight)")
        .expect("make-overlay");
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 6, 6),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline,
        "any overlay in the buffer disqualifies the item route"
    );
}

#[test]
fn classifier_rejects_active_display_table() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abc\n");
    {
        let table = neovm_core::emacs_core::Value::make_char_table(
            Value::symbol("display-table"),
            Value::NIL,
            6,
        );
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.set_buffer_local("buffer-display-table", table);
    }
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"abc\n", 0, 0),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline
    );
}

#[test]
fn ascii_source_matches_buffer_text_source_cursor_items() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello world\n");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let start = CharPos0::ZERO;
    let line_end = CharPos0::new("hello world".chars().count());

    let mut cursor = BufferTextSourceCursor::new(
        buf_id,
        buffer,
        start,
        line_end.add_len(CharLen::new(1)),
        RenderFaceRef::Inherit,
    );
    let mut cursor_items = Vec::new();
    let mut context = DisplaySourceContext::empty();
    while let Some(item) = cursor.next_item(&mut context) {
        cursor_items.push(item);
    }

    let mut ascii = BufferAsciiItemSource::with_row_break(
        buf_id,
        buffer,
        start,
        line_end,
        RenderFaceRef::Inherit,
    );
    let mut ascii_items = Vec::new();
    while let Some(item) = ascii.next_item(&mut context) {
        ascii_items.push(item);
    }

    assert_eq!(ascii_items, cursor_items);
    assert_eq!(ascii_items.len(), 2, "one text run, then the row break");
}

#[test]
fn ascii_source_text_only_omits_the_row_break() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "ab\n");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let mut source = BufferAsciiItemSource::text_only(
        buf_id,
        buffer,
        CharPos0::ZERO,
        CharPos0::new(2),
        RenderFaceRef::Inherit,
    );
    let mut context = DisplaySourceContext::empty();
    let first = source.next_item(&mut context).expect("text run item");
    assert!(matches!(first.kind, DisplayItemKind::TextRun(_)));
    assert_eq!(source.next_item(&mut context), None);
}
