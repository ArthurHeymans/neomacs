use super::*;

#[test]
fn buffer_edit_without_tree_does_not_track_pending_edit() {
    crate::test_utils::init_test_tracing();
    let mut manager = TreeSitterManager::new();
    let buffer_id = BufferId(7);
    let mut buffer = Buffer::new(buffer_id, Value::string("test"));
    buffer.insert("alpha\nbeta\ngamma\n");
    let end = buffer.accessible_emacs_byte_range().end();

    manager.begin_buffer_edit(buffer_id, &buffer, EmacsByteRange::new(end, end));

    assert!(manager.pending_edits.is_empty());
}

#[test]
fn parser_edit_positions_are_relative_to_the_accessible_region() {
    crate::test_utils::init_test_tracing();
    let buffer_id = BufferId(7);
    let mut buffer = Buffer::new(buffer_id, Value::string("test"));
    buffer.insert("hidden\nalpha\nbeta\nhidden");
    buffer.narrow_to_emacs_byte_range(EmacsByteRange::from_usize(7, 17));

    let edit = PendingBufferEdit::for_buffer(
        &buffer,
        EmacsByteRange::from_usize(13, 17),
        ParserPointTracking::LineAndColumn,
    );

    assert_eq!(edit.start_byte, 6);
    assert_eq!(edit.old_end_byte, 10);
    assert_eq!(edit.start_position, Point::new(1, 0));
    assert_eq!(edit.old_end_position, Point::new(1, 4));
}

#[test]
fn byte_only_parser_edit_preparation_does_not_scan_for_line_columns() {
    crate::test_utils::init_test_tracing();
    let buffer_id = BufferId(7);
    let mut buffer = Buffer::new(buffer_id, Value::string("test"));
    buffer.insert("alpha\nbeta\ngamma\n");

    let edit = PendingBufferEdit::for_buffer(
        &buffer,
        EmacsByteRange::from_usize(11, 11),
        ParserPointTracking::BytesOnly,
    );

    assert_eq!(edit.start_byte, 11);
    assert_eq!(edit.start_position, Point::new(1, 0));
    assert_eq!(edit.old_end_position, Point::new(1, 0));
}
