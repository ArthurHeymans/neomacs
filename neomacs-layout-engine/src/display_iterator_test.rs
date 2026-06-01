use super::*;

#[test]
fn iterator_method_codes_match_gnu_it_method() {
    let cases = [
        (ItMethod::FromBuffer, 0),
        (ItMethod::FromDisplayVector, 1),
        (ItMethod::FromString, 2),
        (ItMethod::FromCString, 3),
        (ItMethod::FromImage, 4),
        (ItMethod::FromStretch, 5),
        (ItMethod::FromXwidget, 6),
    ];

    for (method, code) in cases {
        assert_eq!(method.gnu_code(), code);
        assert_eq!(ItMethod::from_gnu_code(code), Some(method));
    }
    assert_eq!(ItMethod::from_gnu_code(7), None);
}

#[test]
fn iterator_element_codes_match_gnu_display_element_type() {
    let cases = [
        (ItWhat::Character, 0),
        (ItWhat::Composition, 1),
        (ItWhat::Glyphless, 2),
        (ItWhat::Image, 3),
        (ItWhat::Stretch, 4),
        (ItWhat::Eob, 5),
        (ItWhat::Truncation, 6),
        (ItWhat::Continuation, 7),
        (ItWhat::Xwidget, 8),
    ];

    for (what, code) in cases {
        assert_eq!(what.gnu_code(), Some(code));
        assert_eq!(ItWhat::from_gnu_code(code), Some(what));
    }
    assert_eq!(ItWhat::Empty.gnu_code(), None);
    assert_eq!(ItWhat::from_gnu_code(u8::MAX), None);
    assert_eq!(ItWhat::from_gnu_code(9), None);
}

#[test]
fn iterator_line_wrap_codes_match_gnu_line_wrap_method() {
    let cases = [
        (LineWrap::Truncate, 0),
        (LineWrap::WordWrap, 1),
        (LineWrap::WindowWrap, 2),
    ];

    for (line_wrap, code) in cases {
        assert_eq!(line_wrap.gnu_code(), code);
        assert_eq!(LineWrap::from_gnu_code(code), Some(line_wrap));
    }
    assert_eq!(LineWrap::from_gnu_code(3), None);
}

#[test]
fn iterator_bidi_dir_codes_match_gnu_bidi_dir_t() {
    let cases = [(BidiDir::Neutral, 0), (BidiDir::Ltr, 1), (BidiDir::Rtl, 2)];

    for (direction, code) in cases {
        assert_eq!(direction.gnu_code(), code);
        assert_eq!(BidiDir::from_gnu_code(code), Some(direction));
    }
    assert_eq!(BidiDir::from_gnu_code(3), None);
}

#[test]
fn mode_line_iterator_sets_mode_line_p() {
    let it = It::new_for_mode_line(0);
    assert!(it.mode_line_p);
    assert_eq!(it.method, ItMethod::FromString);
    assert_eq!(it.charpos, -1);
    assert_eq!(it.bytepos, -1);
}

#[test]
fn buffer_iterator_does_not_set_mode_line_p() {
    let it = It::new_for_buffer(1, 1, 0);
    assert!(!it.mode_line_p);
    assert_eq!(it.method, ItMethod::FromBuffer);
    assert_eq!(it.line_wrap, LineWrap::WindowWrap);
    assert_eq!(it.charpos, 1);
}

#[test]
fn reset_row_geometry_zeroes_per_row_fields() {
    let mut it = It::new_for_buffer(1, 1, 0);
    it.current_x = 100.0;
    it.ascent = 15.0;
    it.descent = 5.0;
    it.pixel_width = 10.0;
    it.reset_row_geometry();
    assert_eq!(it.current_x, 0.0);
    assert_eq!(it.ascent, 0.0);
    assert_eq!(it.descent, 0.0);
    assert_eq!(it.pixel_width, 0.0);
}

#[test]
fn bidi_fields_default_inactive() {
    // Per the Rev 3 correction: bidi fields are core to struct
    // it but neomacs day-1 ships with bidi_p=false. The fields
    // MUST exist (so the walker's type signature matches GNU's
    // calling convention) but day-1 uses unicode order.
    let it = It::new_for_mode_line(0);
    assert!(!it.bidi_p);
    assert_eq!(it.paragraph_embedding, BidiDir::Ltr);
}
