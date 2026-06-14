use super::*;

fn request(
    requested_window_start: i64,
    previous_window_end: Option<i64>,
    point_charpos: i64,
    accessible_end: i64,
    max_rows: usize,
    is_minibuffer: bool,
) -> BufferTextWindowSourceRequest {
    BufferTextWindowSourceRequest::new(
        requested_window_start,
        previous_window_end,
        point_charpos,
        0,
        accessible_end,
        max_rows,
        20.0,
        is_minibuffer,
    )
}

fn byte_at_charpos(text: &'static [u8]) -> impl Fn(i64) -> Option<u8> {
    move |charpos| text.get(charpos as usize).copied()
}

#[test]
fn source_request_scrolls_back_when_start_is_past_remaining_content() {
    let text = b"a\nb\nc\nd\ne\nf\n";
    let resolved = request(10, None, 10, text.len() as i64, 4, false)
        .resolve_window_start(byte_at_charpos(text));

    assert_eq!(resolved, 7);
}

#[test]
fn source_request_scrolls_back_when_point_is_above_window_start() {
    let text = b"a\nb\nc\nd\ne\nf\n";
    let resolved = request(8, None, 3, text.len() as i64, 4, false)
        .resolve_window_start(byte_at_charpos(text));

    assert_eq!(resolved, 1);
}

#[test]
fn source_request_scrolls_forward_when_point_passed_previous_end() {
    let text = b"a\nb\nc\nd\ne\nf\ng\nh\n";
    let resolved = request(0, Some(4), 12, text.len() as i64, 4, false)
        .resolve_window_start(byte_at_charpos(text));

    assert_eq!(resolved, 7);
}

#[test]
fn source_request_does_not_forward_scroll_minibuffer() {
    let text = b"a\nb\nc\nd\ne\nf\ng\nh\n";
    let resolved = request(0, Some(4), 12, text.len() as i64, 4, true)
        .resolve_window_start(byte_at_charpos(text));

    assert_eq!(resolved, 0);
}
