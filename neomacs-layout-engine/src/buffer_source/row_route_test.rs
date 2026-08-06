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

static TAB_EVERY_8: std::sync::LazyLock<DisplayTabPolicy> =
    std::sync::LazyLock::new(|| DisplayTabPolicy::every(8));

fn wide_fit() -> RowRouteFit<'static> {
    fit_to(640.0)
}

fn fit_to(right_edge_px: f32) -> RowRouteFit<'static> {
    RowRouteFit {
        start_x_px: 0.0,
        start_col: 0,
        char_width_px: 8.0,
        right_edge_px,
        tab_policy: &TAB_EVERY_8,
    }
}

fn classify_in_buffer(
    eval: &Context,
    buf_id: BufferId,
    row: RowRouteRowStart<'_>,
    fit: RowRouteFit<'_>,
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
fn classifier_routes_tab_and_wide_char_rows() {
    let mut eval = Context::new();
    // Tabs, narrow non-ASCII (e-acute), and wide CJK chars all route since
    // the phase 2b extension.
    for text in [
        "a\tb\n",
        "\t\tindent\n",
        "h\u{00e9}llo\n",
        "ab\u{4E2D}\u{6587}cd\n",
        "a\t\u{4E2D}b\n",
    ] {
        let buf_id = buffer_with_text(&mut eval, text);
        assert_eq!(
            classify_in_buffer(
                &eval,
                buf_id,
                row_start(text.as_bytes(), 0, 0),
                wide_fit(),
                plain_policy()
            ),
            RowAcquisitionRoute::ItemRenderer,
            "content {text:?} must route to the item renderer"
        );
    }
}

#[test]
fn plan_reports_tab_wide_flags_and_char_byte_lengths() {
    let mut eval = Context::new();
    let text = "a\t\u{4E2D}b\n";
    let buf_id = buffer_with_text(&mut eval, text);
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(text.as_bytes(), 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("tab+wide row routes");
    assert_eq!(plan.line_char_len(), 4);
    assert_eq!(plan.line_byte_len(), 6, "one 3-byte CJK char");
    assert!(plan.has_tab());
    assert!(plan.has_wide());

    // A plain ASCII row classifies without either flag.
    let buf_id = buffer_with_text(&mut eval, "ab\n");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let ascii = plan_ascii_row(buffer, row_start(b"ab\n", 0, 0), wide_fit(), plain_policy())
        .expect("ascii row routes");
    assert!(!ascii.has_tab());
    assert!(!ascii.has_wide());
}

#[test]
fn plan_face_boundaries_are_char_offsets_on_multibyte_rows() {
    let mut eval = Context::new();
    // "a e-acute CJK b": 4 chars, 7 bytes. A face span over chars 2..4
    // (1-based [2, 4) = e-acute + CJK) must split at CHAR offsets 1 and 3.
    let text = "a\u{00E9}\u{4E2D}b\n";
    let buf_id = buffer_with_text(&mut eval, text);
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 2 4 'face 'bold)")
        .expect("put-text-property");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(text.as_bytes(), 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("multibyte faced row routes");
    assert_eq!(plan.line_char_len(), 4);
    assert_eq!(plan.line_byte_len(), 7);
    assert_eq!(plan.face_boundaries(), &[1, 3]);
    assert_eq!(
        plan.segment_ranges(CharPos0::ZERO),
        vec![
            (CharPos0::ZERO, CharPos0::new(1)),
            (CharPos0::new(1), CharPos0::new(3)),
            (CharPos0::new(3), CharPos0::new(4)),
        ]
    );
}

#[test]
fn classifier_rejects_tab_line_exactly_filling_the_row() {
    let mut eval = Context::new();
    // "ab\t": the tab expands from col 2 to the col-8 stop, 64px at 8px
    // cells. A 64px row is exact fill — refused; one cell of slack routes.
    let buf_id = buffer_with_text(&mut eval, "ab\t\n");
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"ab\t\n", 0, 0),
            fit_to(64.0),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline,
        "tab expansion landing exactly on the right edge must refuse"
    );
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"ab\t\n", 0, 0),
            fit_to(72.0),
            plain_policy()
        ),
        RowAcquisitionRoute::ItemRenderer
    );
}

#[test]
fn classifier_fit_advances_a_full_stop_for_tab_exactly_on_a_stop() {
    let mut eval = Context::new();
    // GNU next_tab_x (xdisp.c gui_produce_glyphs): the +1 in
    // ((1 + x + tab_width - 1) / tab_width) * tab_width forces a tab landing
    // EXACTLY on a stop to advance a FULL stop. "abcdefgh\t": the tab starts
    // exactly on the col-8 stop, so the line ends at col 16 (128px at 8px
    // cells) — a 16-cell row is exact fill (refused), 17 cells route.
    let buf_id = buffer_with_text(&mut eval, "abcdefgh\t\n");
    let text = b"abcdefgh\t\n";
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 0, 0),
            fit_to(128.0),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline,
        "a tab exactly on a stop must expand a full stop (to col 16), exact fill refuses"
    );
    // If the +1 rule were broken (tab advancing zero or one cell), the line
    // would end well inside 12 cells and wrongly route.
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 0, 0),
            fit_to(96.0),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline,
        "the full-stop expansion crosses a 12-cell row: refuse"
    );
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 0, 0),
            fit_to(136.0),
            plain_policy()
        ),
        RowAcquisitionRoute::ItemRenderer
    );
}

#[test]
fn classifier_rejects_wide_char_exact_fill_and_straddle() {
    let mut eval = Context::new();
    // "abc" + CJK = 5 cols = 40px at 8px cells.
    let text = "abc\u{4E2D}\n";
    let buf_id = buffer_with_text(&mut eval, text);
    for (edge, expected) in [
        (40.0, RowAcquisitionRoute::BufferPipeline), // exact fill
        (36.0, RowAcquisitionRoute::BufferPipeline), // wide char straddles the edge
        (48.0, RowAcquisitionRoute::ItemRenderer),   // one cell of slack
    ] {
        assert_eq!(
            classify_in_buffer(
                &eval,
                buf_id,
                row_start(text.as_bytes(), 0, 0),
                fit_to(edge),
                plain_policy()
            ),
            expected,
            "edge {edge}px"
        );
    }
}

#[test]
fn classifier_rejects_content_the_item_route_does_not_cover() {
    let mut eval = Context::new();
    // Control chars, missing final newline, empty line, combining marks,
    // zero-width chars, complex scripts, regional-indicator pairs, and
    // nobreak space/hyphen (nobreak-char-display consults a setting): all
    // stay on the buffer pipeline.
    for text in [
        b"a\x01b\n".as_slice(),
        b"a\rb\n".as_slice(),
        b"hello".as_slice(),
        b"\nx\n".as_slice(),
        "e\u{0301}llo\n".as_bytes(), // combining acute
        "a\u{200B}b\n".as_bytes(),   // zero-width space
        "a\u{200D}b\n".as_bytes(),   // ZWJ
        "\u{0633}\u{0644}\u{0627}\u{0645}\n".as_bytes(), // Arabic (shaped run)
        "\u{1F1E6}\u{1F1E9}\n".as_bytes(), // regional-indicator flag pair
        "a\u{00A0}b\n".as_bytes(),   // no-break space
        "a\u{00AD}b\n".as_bytes(),   // soft hyphen
        "a\u{0080}b\n".as_bytes(),   // C1 control (octal escape)
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
    let exact = fit_to(32.0);
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
    let slack = fit_to(40.0);
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
fn classifier_accepts_face_property_span_and_plans_boundaries() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\nworld\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 3 5 'face 'bold)")
        .expect("put-text-property");
    let text = b"hello\nworld\n";
    // A face span mid-line routes and segments the row at each property
    // change ("he" / "ll" bold / "o").
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(buffer, row_start(text, 0, 0), wide_fit(), plain_policy())
        .expect("face-propped row routes");
    assert_eq!(plan.line_char_len(), 5);
    assert_eq!(plan.line_byte_len(), 5);
    assert_eq!(plan.face_boundaries(), &[2, 4]);
    assert!(plan.is_segmented());
    assert_eq!(
        plan.segment_ranges(CharPos0::ZERO),
        vec![
            (CharPos0::ZERO, CharPos0::new(2)),
            (CharPos0::new(2), CharPos0::new(4)),
            (CharPos0::new(4), CharPos0::new(5)),
        ]
    );
    // The second, unfaced row routes unsegmented.
    let plan = plan_ascii_row(buffer, row_start(text, 6, 6), wide_fit(), plain_policy())
        .expect("unfaced row routes");
    assert!(!plan.is_segmented());
}

#[test]
fn classifier_accepts_font_lock_face_and_whole_line_span() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "keyword\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 1 8 'font-lock-face 'bold)")
        .expect("put-text-property");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(b"keyword\n", 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("font-lock-faced row routes");
    // The property covers the whole line but ends before the newline: the
    // change on the newline byte is not a text-segment boundary.
    assert_eq!(plan.face_boundaries(), &[] as &[usize]);
}

#[test]
fn classifier_accepts_fontified_boundary_as_segment_split() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abcd\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // A non-face property change (fontified) still splits the run, exactly
    // like GNU compute_stop_pos stops at EVERY property change.
    eval.eval_str("(put-text-property 1 3 'fontified t)")
        .expect("put-text-property");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(b"abcd\n", 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("fontified-bounded row routes");
    assert_eq!(plan.face_boundaries(), &[2]);
}

#[test]
fn classifier_rejects_hazard_properties_anywhere_on_the_line() {
    for (prop, value) in [
        ("display", "\"X\""),
        ("composition", "'((0 . 1))"),
        ("invisible", "t"),
        ("mouse-face", "'highlight"),
        ("line-height", "2.0"),
    ] {
        let mut eval = Context::new();
        let buf_id = buffer_with_text(&mut eval, "hello\n");
        eval.buffer_manager_mut().set_current(buf_id);
        eval.eval_str(&format!("(put-text-property 3 5 '{prop} {value})"))
            .expect("put-text-property");
        assert_eq!(
            classify_in_buffer(
                &eval,
                buf_id,
                row_start(b"hello\n", 0, 0),
                wide_fit(),
                plain_policy()
            ),
            RowAcquisitionRoute::BufferPipeline,
            "mid-line {prop} must stay on the buffer pipeline"
        );
    }
}

#[test]
fn classifier_rejects_hazard_property_on_the_newline() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\nx\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // A display property covering ONLY the newline would replace the line
    // end; the hazard probe must reach it.
    eval.eval_str("(put-text-property 6 7 'display \"|\")")
        .expect("put-text-property");
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(b"hello\nx\n", 0, 0),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline
    );
}

#[test]
fn classifier_accepts_face_only_overlay_and_plans_boundaries() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // Overlay over "el" (elisp 2..4) carrying only face-affecting props.
    eval.eval_str(
        "(let ((ov (make-overlay 2 4))) \
           (overlay-put ov 'face 'bold) \
           (overlay-put ov 'priority 5) \
           (overlay-put ov 'evaporate t))",
    )
    .expect("face-only overlay");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(b"hello\n", 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("face-only overlay row routes");
    // The overlay's start and end are face-segment boundaries, the neomacs
    // mirror of GNU compute_stop_pos folding next_overlay_change into
    // stop_charpos.
    assert_eq!(plan.face_boundaries(), &[1, 3]);
    assert!(plan.is_segmented());
}

#[test]
fn classifier_merges_overlay_and_text_prop_boundaries() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // Text face over "he" plus overlapping overlays: boundaries merge,
    // sort, and dedupe into one ascending char-offset list.
    eval.eval_str(
        "(progn (put-text-property 1 3 'face 'bold) \
                (overlay-put (make-overlay 3 5) 'face 'highlight) \
                (overlay-put (make-overlay 2 5) 'face 'underline))",
    )
    .expect("overlapping overlays over a text span");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(b"hello\n", 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("face-only overlays route");
    assert_eq!(plan.face_boundaries(), &[1, 2, 4]);
}

#[test]
fn classifier_accepts_zero_length_face_only_overlay() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // GNU next_overlay_change: an empty overlay contributes exactly one
    // stop, at its position; face merging paints nothing for it in either
    // path (the shadow suite proves glyph identity).
    eval.eval_str("(overlay-put (make-overlay 3 3) 'face 'bold)")
        .expect("zero-length overlay");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(b"hello\n", 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("zero-length face-only overlay routes");
    assert_eq!(plan.face_boundaries(), &[2]);
}

#[test]
fn classifier_rejects_overlay_hazard_properties() {
    // Any intersecting overlay carrying a property beyond the face-affecting
    // allow-list keeps the buffer pipeline: strings and display/invisible
    // rewrite content, window restricts applicability, category indirects to
    // arbitrary props, and unknown props are conservatively refused.
    for (prop, value) in [
        ("before-string", "\"B\""),
        ("after-string", "\"A\""),
        ("display", "\"X\""),
        ("invisible", "t"),
        ("mouse-face", "'highlight"),
        ("window", "t"),
        ("category", "'some-category"),
        ("line-prefix", "\"> \""),
        ("help-echo", "\"tip\""),
    ] {
        let mut eval = Context::new();
        let buf_id = buffer_with_text(&mut eval, "hello\n");
        eval.buffer_manager_mut().set_current(buf_id);
        eval.eval_str(&format!(
            "(let ((ov (make-overlay 2 4))) \
               (overlay-put ov 'face 'bold) \
               (overlay-put ov '{prop} {value}))"
        ))
        .expect("hazard overlay");
        assert_eq!(
            classify_in_buffer(
                &eval,
                buf_id,
                row_start(b"hello\n", 0, 0),
                wide_fit(),
                plain_policy()
            ),
            RowAcquisitionRoute::BufferPipeline,
            "an intersecting overlay with {prop} must stay on the buffer pipeline"
        );
    }
}

#[test]
fn classifier_rejects_string_overlay_touching_row_endpoints() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "ab\ncd\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // Overlay ends exactly at the second row's start: its after-string fires
    // there (GNU load_overlay_strings collects at end == charpos), so the
    // row must refuse.
    eval.eval_str("(overlay-put (make-overlay 1 4) 'after-string \"A\")")
        .expect("after-string overlay");
    let text = b"ab\ncd\n";
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 3, 3),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline,
        "an overlay ending at the row start with an after-string must refuse"
    );
    // Overlay starting exactly at the row's newline: its before-string fires
    // at the newline position; conservatively refused too.
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "ab\ncd\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(overlay-put (make-overlay 3 5) 'before-string \"B\")")
        .expect("before-string overlay");
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 0, 0),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline,
        "an overlay starting at the row's newline with a before-string must refuse"
    );
}

#[test]
fn classifier_ignores_overlays_on_other_rows() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\nworld\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // A string-carrying overlay entirely on the SECOND row: the first row
    // does not intersect it and still routes; the second refuses.
    eval.eval_str("(overlay-put (make-overlay 8 10) 'before-string \"B\")")
        .expect("second-row overlay");
    let text = b"hello\nworld\n";
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 0, 0),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::ItemRenderer,
        "an overlay on another row must not disqualify this row"
    );
    assert_eq!(
        classify_in_buffer(
            &eval,
            buf_id,
            row_start(text, 6, 6),
            wide_fit(),
            plain_policy()
        ),
        RowAcquisitionRoute::BufferPipeline
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
fn routed_source_matches_buffer_text_source_cursor_items_for_tab_and_wide() {
    let mut eval = Context::new();
    // Tab and a wide CJK char inside the run: the cursor keeps both in ONE
    // plain TextRun (tab and CJK classify as Text), and the routed source
    // must produce the identical item — same UTF-8 text, same char/byte
    // spans — followed by the identical row break.
    let text = "a\t\u{4E2D} b\n";
    let buf_id = buffer_with_text(&mut eval, text);
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let start = CharPos0::ZERO;
    let line_end = CharPos0::new(text.chars().count() - 1);

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

    let mut routed = BufferAsciiItemSource::with_row_break(
        buf_id,
        buffer,
        start,
        line_end,
        RenderFaceRef::Inherit,
    );
    let mut routed_items = Vec::new();
    while let Some(item) = routed.next_item(&mut context) {
        routed_items.push(item);
    }

    assert_eq!(routed_items, cursor_items);
    assert_eq!(routed_items.len(), 2, "one text run, then the row break");
    let DisplayItemKind::TextRun(run) = &routed_items[0].kind else {
        panic!("expected text run, got {:?}", routed_items[0].kind);
    };
    assert_eq!(run.text.as_ref(), "a\t\u{4E2D} b");
}

fn face_resolver_for(eval: &Context) -> FaceResolver {
    FaceResolver::new(eval.face_table(), 0x00FF_FFFF, 0x0000_0000, 14.0, None)
}

#[test]
fn plan_row_face_segments_resolves_per_segment_stable_ids() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 3 5 'face 'bold)")
        .expect("put-text-property");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let plan = plan_ascii_row(
        buffer,
        row_start(b"hello\n", 0, 0),
        wide_fit(),
        plain_policy(),
    )
    .expect("plan");
    let resolver = face_resolver_for(&eval);
    let mut face_ids = FrameFaceAttempt::for_test_with_next_id(0);
    let segments = plan_row_face_segments(buffer, &resolver, &mut face_ids, CharPos0::ZERO, &plan);
    assert_eq!(segments.len(), 3);
    assert_eq!(
        segments
            .iter()
            .map(|segment| (segment.start.get(), segment.end.get()))
            .collect::<Vec<_>>(),
        vec![(0, 2), (2, 4), (4, 5)]
    );
    // The outer (unfaced) segments content-address to the SAME stable id;
    // the bold span gets its own.
    assert_eq!(segments[0].face_id, segments[2].face_id);
    assert_ne!(segments[0].face_id, segments[1].face_id);
    assert_ne!(
        segments[0].resolved.font_weight,
        segments[1].resolved.font_weight
    );
}

#[test]
fn resolve_routed_position_face_covers_the_newline_span() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abcd\nnext\n");
    eval.buffer_manager_mut().set_current(buf_id);
    // Face span covering the newline (1-based [3, 6) covers chars "cd\n").
    eval.eval_str("(put-text-property 3 6 'face 'bold)")
        .expect("put-text-property");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let resolver = face_resolver_for(&eval);
    let mut face_ids = FrameFaceAttempt::for_test_with_next_id(0);
    let (span_id, _) =
        resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::new(2));
    let (newline_id, _) =
        resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::new(4));
    let (base_id, _) =
        resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::new(5));
    assert_eq!(
        span_id, newline_id,
        "a span covering the newline keeps its face at the newline position"
    );
    assert_ne!(newline_id, base_id);

    // A span ending exactly at the newline leaves the newline on the base
    // face.
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "abcd\nnext\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 3 5 'face 'bold)")
        .expect("put-text-property");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let resolver = face_resolver_for(&eval);
    let mut face_ids = FrameFaceAttempt::for_test_with_next_id(0);
    let (span_id, _) =
        resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::new(2));
    let (newline_id, _) =
        resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::new(4));
    let (base_id, _) =
        resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::new(5));
    assert_ne!(span_id, newline_id);
    assert_eq!(newline_id, base_id);
}

#[test]
fn routed_segment_item_face_agrees_for_plain_face_spans() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 3 5 'face 'bold)")
        .expect("put-text-property");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let resolver = face_resolver_for(&eval);
    let mut face_ids = FrameFaceAttempt::for_test_with_next_id(0);
    let default_resolved = resolver.default_face().clone();
    let default_face_id = crate::display_row::face_state::stable_face_id_for_resolved(
        &mut face_ids,
        &default_resolved,
    );
    for pos in [0usize, 2, 4] {
        let (expected_id, _) =
            resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::new(pos));
        assert!(
            !routed_segment_item_face_diverges(
                buffer,
                &resolver,
                &mut face_ids,
                &default_resolved,
                default_face_id,
                CharPos0::new(pos),
                expected_id,
            ),
            "checkpoint and per-run face chains must agree at {pos}"
        );
    }
}

#[test]
fn routed_segment_item_face_diverges_under_default_remapping() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\n");
    eval.buffer_manager_mut().set_current(buf_id);
    eval.eval_str("(put-text-property 1 6 'face 'italic)")
        .expect("put-text-property");
    eval.eval_str(
        "(progn (make-local-variable 'face-remapping-alist) \
                (setq face-remapping-alist '((default . bold))))",
    )
    .expect("face remapping");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let resolver = face_resolver_for(&eval);
    let mut face_ids = FrameFaceAttempt::for_test_with_next_id(0);
    let default_resolved = resolver.default_face().clone();
    let default_face_id = crate::display_row::face_state::stable_face_id_for_resolved(
        &mut face_ids,
        &default_resolved,
    );
    let (checkpoint_id, checkpoint_resolved) =
        resolve_routed_position_face(buffer, &resolver, &mut face_ids, CharPos0::ZERO);
    if checkpoint_resolved.font_weight == resolver.default_face().font_weight {
        // The engine's face remapping is not applied through this seam in
        // this build; the guard then has nothing to diverge on.
        return;
    }
    assert!(
        routed_segment_item_face_diverges(
            buffer,
            &resolver,
            &mut face_ids,
            &default_resolved,
            default_face_id,
            CharPos0::ZERO,
            checkpoint_id,
        ),
        "remapped default must force the row off the item route"
    );
}

#[test]
fn ascii_source_segments_produce_per_face_text_runs_and_break_face() {
    let mut eval = Context::new();
    let buf_id = buffer_with_text(&mut eval, "hello\n");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let bold = RenderFaceRef::FaceId(neomacs_display_protocol::types::FaceId::new(40));
    let base = RenderFaceRef::FaceId(neomacs_display_protocol::types::FaceId::new(33));
    let mut source = BufferAsciiItemSource::with_row_break_segments(
        buf_id,
        buffer,
        &[
            AsciiRowItemSegment {
                start: CharPos0::ZERO,
                end: CharPos0::new(2),
                face: base,
            },
            AsciiRowItemSegment {
                start: CharPos0::new(2),
                end: CharPos0::new(4),
                face: bold,
            },
            AsciiRowItemSegment {
                start: CharPos0::new(4),
                end: CharPos0::new(5),
                face: base,
            },
        ],
        bold,
    );
    let mut context = DisplaySourceContext::empty();
    let mut items = Vec::new();
    while let Some(item) = source.next_item(&mut context) {
        items.push(item);
    }
    assert_eq!(items.len(), 4, "three text runs then the row break");
    let texts: Vec<_> = items[..3]
        .iter()
        .map(|item| match &item.kind {
            DisplayItemKind::TextRun(run) => run.text.to_string(),
            other => panic!("expected text run, got {other:?}"),
        })
        .collect();
    assert_eq!(texts, vec!["he", "ll", "o"]);
    assert_eq!(
        items.iter().map(|item| item.face).collect::<Vec<_>>(),
        vec![base, bold, base, bold]
    );
    assert!(matches!(items[3].kind, DisplayItemKind::RowBreak(_)));
    // Spans stay contiguous over the row.
    assert_eq!(items[0].span.end, items[1].span.start);
    assert_eq!(items[1].span.end, items[2].span.start);
    assert_eq!(items[2].span.end, items[3].span.start);
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
