use super::*;
use crate::buffer::{CharRange, LispCharPos1};

#[test]
fn undo_entry_head_domain_matches_gnu_apply_marker() {
    crate::test_utils::init_test_tracing();

    assert_eq!(UndoEntryHead::Apply.name(), "apply");
    assert_eq!(
        UndoEntryHead::from_lisp_value(&Value::symbol("apply")),
        Some(UndoEntryHead::Apply)
    );
    assert_eq!(
        UndoEntryHead::from_lisp_value(&Value::symbol("quote")),
        None
    );
    assert_eq!(UndoEntryHead::from_lisp_value(&Value::fixnum(0)), None);
}

#[test]
fn test_undo_boundary_no_args() {
    crate::test_utils::init_test_tracing();
    let mut eval = super::super::eval::Context::new();
    let result = builtin_undo_boundary(&mut eval, vec![]);
    assert!(result.is_ok());
    assert!(result.unwrap().is_nil());
}

#[test]
fn test_undo_boundary_eval_inserts_boundary_marker() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        buffer.insert("x");
    }
    let result = builtin_undo_boundary(&mut eval, vec![]);
    assert!(result.is_ok());
    let buffer = eval.buffers.current_buffer().expect("scratch buffer");
    let ul = buffer.get_undo_list();
    assert!(crate::buffer::undo_list_has_trailing_boundary(&ul));
}

#[test]
fn test_undo_boundary_wrong_args() {
    crate::test_utils::init_test_tracing();
    let mut eval = super::super::eval::Context::new();
    let result = builtin_undo_boundary(&mut eval, vec![Value::fixnum(1)]);
    assert!(result.is_err());
}

#[test]
fn test_primitive_undo_with_count_and_list() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let list = Value::list(vec![Value::NIL, Value::NIL, Value::NIL]);
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), list]);
    assert!(result.is_ok());
    // All-nil list: one group of nothing returns unconsumed tail.
}

#[test]
fn test_primitive_undo_zero_count() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let list = Value::list(vec![Value::NIL, Value::NIL]);
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(0), list]);
    assert!(result.is_ok());
    // Zero count returns list unchanged.
    assert_eq!(format!("{:?}", result.unwrap()), format!("{:?}", list));
}

#[test]
fn test_primitive_undo_negative_count() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let list = Value::list(vec![Value::NIL]);
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(-5), list]);
    assert!(result.is_ok());
    // Negative count returns list unchanged.
    assert_eq!(format!("{:?}", result.unwrap()), format!("{:?}", list));
}

#[test]
fn test_primitive_undo_invalid_count() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let list = Value::list(vec![]);
    let result = builtin_primitive_undo(&mut eval, vec![Value::make_float(1.5), list]);
    assert!(result.is_err());
}

#[test]
fn test_primitive_undo_non_list_signals_wrong_type() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), Value::fixnum(7)]);
    assert!(result.is_err());
}

#[test]
fn test_primitive_undo_wrong_arg_count() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(1)]);
    assert!(result.is_err());

    let result = builtin_primitive_undo(&mut eval, vec![]);
    assert!(result.is_err());

    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), Value::NIL, Value::NIL]);
    assert!(result.is_err());
}

#[test]
fn test_primitive_undo_reverts_insertion() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    // Insert text into the current buffer.
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        buffer.insert("hello");
    }
    // Build an undo list that describes the insertion: (1 . 6)
    // meaning bytes [1,6) were inserted (1-indexed).
    let entry = Value::cons(Value::fixnum(1), Value::fixnum(6));
    let list = Value::cons(entry, Value::NIL);
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), list]);
    assert!(result.is_ok());
    let contents = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .buffer_string();
    assert_eq!(contents, "");
}

#[test]
fn test_primitive_undo_reverts_deletion() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    // Buffer starts empty; the undo entry says "hello" was deleted at pos 1.
    let entry = Value::cons(Value::string("hello"), Value::fixnum(1));
    let list = Value::cons(entry, Value::NIL);
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), list]);
    assert!(result.is_ok());
    let contents = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .buffer_string();
    assert_eq!(contents, "hello");
}

#[test]
fn test_primitive_undo_restores_marker_adjustments_after_deletion() {
    crate::test_utils::init_test_tracing();
    use super::super::{eval::Context, marker};

    let mut eval = Context::new();
    let buf_id = eval.buffers.current_buffer_id().expect("scratch buffer");
    eval.buffers
        .current_buffer_mut()
        .expect("scratch buffer")
        .insert("ae");

    let marker = marker::make_registered_buffer_marker(
        &mut eval.buffers,
        buf_id,
        LispCharPos1::new(2),
        false,
    );
    let delete_record = Value::cons(Value::string("bcd"), Value::fixnum(2));
    let marker_record = Value::cons(marker, Value::fixnum(-2));
    let list = Value::list(vec![delete_record, marker_record]);

    builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), list]).unwrap();

    let contents = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .buffer_string();
    assert_eq!(contents, "abcde");
    let marker_position =
        marker::builtin_marker_position_in_buffers(&eval.buffers, vec![marker]).unwrap();
    assert_eq!(marker_position, Value::fixnum(4));
}

#[test]
fn test_delete_records_marker_adjustments_for_primitive_undo() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = eval
        .eval_str(
            r#"(progn
  (insert "ABCDE")
  (let ((m (make-marker)))
    (set-marker m 3)
    (undo-boundary)
    (goto-char 3)
    (insert "123")
    (undo-boundary)
    (delete-region 1 3)
    (let ((after-delete (marker-position m)))
      (primitive-undo 1 buffer-undo-list)
      (list after-delete (marker-position m) (buffer-string)))))"#,
        )
        .expect("undo marker eval");

    assert_eq!(
        result,
        Value::list(vec![
            Value::fixnum(1),
            Value::fixnum(3),
            Value::string("AB123CDE")
        ])
    );
}

#[test]
fn replace_match_undo_keeps_overlay_endpoint_like_gnu() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = eval
        .eval_str(
            r#"(progn
  (insert "hello WORLD hello WORLD hello")
  (let ((ov1 (make-overlay 1 12))
        (ov2 (make-overlay 13 25)))
    (undo-boundary)
    (goto-char 1)
    (while (re-search-forward "WORLD" nil t)
      (replace-match "UNIVERSE" t t))
    (let ((after-replace (list (overlay-start ov1) (overlay-end ov1)
                               (overlay-start ov2) (overlay-end ov2))))
      (primitive-undo 1 buffer-undo-list)
      (list after-replace
            (buffer-string)
            (overlay-start ov1) (overlay-end ov1)
            (overlay-start ov2) (overlay-end ov2)))))"#,
        )
        .expect("replace-match undo eval");

    assert_eq!(
        result,
        Value::list(vec![
            Value::list(vec![
                Value::fixnum(1),
                Value::fixnum(15),
                Value::fixnum(16),
                Value::fixnum(31),
            ]),
            Value::string("hello WORLD hello WORLD hello"),
            Value::fixnum(1),
            Value::fixnum(12),
            Value::fixnum(13),
            Value::fixnum(25),
        ])
    );
}

#[test]
fn transpose_regions_undo_records_equal_regions_like_gnu() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = eval
        .eval_str(
            r#"(progn
  (insert "AAA-BBB-CCC")
  (let ((m (copy-marker 5 t)))
    (undo-boundary)
    (transpose-regions 1 3 5 7)
    (let ((after (list (buffer-string) (marker-position m))))
      (primitive-undo 1 buffer-undo-list)
      (list after (buffer-string) (marker-position m)))))"#,
        )
        .expect("transpose undo eval");

    assert_eq!(
        result,
        Value::list(vec![
            Value::list(vec![Value::string("BBA-AAB-CCC"), Value::fixnum(1)]),
            Value::string("AAA-BBB-CCC"),
            Value::fixnum(3),
        ])
    );
}

#[test]
fn test_primitive_undo_reverts_raw_unibyte_deletion() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    eval.buffers
        .current_buffer_mut()
        .expect("scratch buffer")
        .set_multibyte_value(false);
    let raw = Value::heap_string(crate::heap_types::LispString::from_unibyte(vec![0xFF]));
    let entry = Value::cons(raw, Value::fixnum(1));
    let list = Value::cons(entry, Value::NIL);
    let result = builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), list]);
    assert!(result.is_ok());
    let contents = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .buffer_substring_lisp_string_range(crate::buffer::EmacsByteRange::from_usize(0, 1));
    assert!(!contents.is_multibyte());
    assert_eq!(contents.as_bytes(), &[0xFF]);
}

#[test]
fn test_primitive_undo_restores_nil_text_property_value() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let id = eval.buffers.current_buffer_id().expect("scratch buffer");
    let face = Value::symbol("face");
    let bold = Value::symbol("bold");

    eval.buffers
        .current_buffer_mut()
        .expect("scratch buffer")
        .insert("abcd");
    eval.buffers
        .put_buffer_text_property_in_emacs_byte_range(
            id,
            EmacsByteRange::from_usize(1, 3),
            face,
            bold,
        )
        .expect("scratch buffer");

    let range = Value::cons(Value::fixnum(2), Value::fixnum(4));
    let record = Value::cons(
        Value::NIL,
        Value::cons(face, Value::cons(Value::NIL, range)),
    );
    let list = Value::cons(record, Value::NIL);
    builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), list]).unwrap();

    let props = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .text_props_get_properties_ordered_at_emacs_byte_pos(crate::buffer::EmacsBytePos::new(1));
    assert_eq!(props, vec![(face, Value::NIL)]);
}

#[test]
fn test_primitive_undo_reinserts_string_text_properties() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let face = Value::symbol("face");
    let bold = Value::symbol("bold");
    let text = Value::string("abc");

    let mut table = crate::buffer::text_props::TextPropertyTable::new();
    table.put_property_in_char_range(CharRange::from_usize(0, 3), face, bold);
    crate::emacs_core::value::set_string_text_properties_table_for_value(text, table);

    let record = Value::cons(text, Value::fixnum(1));
    let list = Value::cons(record, Value::NIL);
    builtin_primitive_undo(&mut eval, vec![Value::fixnum(1), list]).unwrap();

    let buf = eval.buffers.current_buffer().expect("scratch buffer");
    assert_eq!(buf.buffer_string(), "abc");
    assert_eq!(
        buf.text_props_get_property_at_emacs_byte_pos(crate::buffer::EmacsBytePos::new(0), face),
        Some(bold)
    );
    assert_eq!(
        buf.text_props_get_property_at_emacs_byte_pos(crate::buffer::EmacsBytePos::new(2), face),
        Some(bold)
    );
}

#[test]
fn test_undo_no_args() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = builtin_undo(&mut eval, vec![]);
    assert!(result.is_err());
}

#[test]
fn test_undo_with_arg() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = builtin_undo(&mut eval, vec![Value::fixnum(5)]);
    assert!(result.is_err());
}

#[test]
fn test_undo_with_invalid_arg() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = builtin_undo(&mut eval, vec![Value::make_float(1.5)]);
    assert!(result.is_err());
}

#[test]
fn test_undo_with_multiple_args() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = builtin_undo(&mut eval, vec![Value::fixnum(2), Value::fixnum(3)]);
    assert!(result.is_err());
}

#[test]
fn test_undo_reverts_inserted_text() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        buffer.insert("abc");
        let mut ul = buffer.get_undo_list();
        crate::buffer::undo::undo_list_boundary(&mut ul);
        buffer.set_undo_list(ul);
    }
    let result = builtin_undo(&mut eval, vec![Value::fixnum(1)]);
    assert!(result.is_ok());
    let contents = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .buffer_string();
    assert_eq!(contents, "");
}

#[test]
fn test_undo_restores_property_when_range_start_was_unpropertied() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let id = eval.buffers.current_buffer_id().expect("scratch buffer");
    let face = Value::symbol("face");
    let bold = Value::symbol("bold");
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        buffer.insert("abcdef");
    }
    eval.buffers
        .put_buffer_text_property_in_emacs_byte_range(
            id,
            EmacsByteRange::from_usize(2, 4),
            face,
            bold,
        )
        .expect("scratch buffer");
    eval.buffers
        .configure_buffer_undo_list(id, Value::NIL)
        .expect("scratch buffer");
    eval.buffers
        .remove_buffer_text_property_in_emacs_byte_range(id, EmacsByteRange::from_usize(0, 4), face)
        .expect("scratch buffer");
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        let mut ul = buffer.get_undo_list();
        crate::buffer::undo::undo_list_boundary(&mut ul);
        buffer.set_undo_list(ul);
    }

    builtin_undo(&mut eval, vec![]).unwrap();

    let buffer = eval.buffers.current_buffer().expect("scratch buffer");
    assert!(
        buffer
            .text_props_get_properties_ordered_at_emacs_byte_pos(crate::buffer::EmacsBytePos::new(
                1
            ))
            .is_empty()
    );
    assert_eq!(
        buffer.text_props_get_properties_ordered_at_emacs_byte_pos(
            crate::buffer::EmacsBytePos::new(2)
        ),
        vec![(face, bold)]
    );
    assert_eq!(
        buffer.text_props_get_properties_ordered_at_emacs_byte_pos(
            crate::buffer::EmacsBytePos::new(3)
        ),
        vec![(face, bold)]
    );
    assert!(
        buffer
            .text_props_get_properties_ordered_at_emacs_byte_pos(crate::buffer::EmacsBytePos::new(
                4
            ))
            .is_empty()
    );
}

#[test]
fn test_set_text_properties_partial_interval_undo_matches_gnu() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let id = eval.buffers.current_buffer_id().expect("scratch buffer");
    let face = Value::symbol("face");
    let bold = Value::symbol("bold");
    let italic = Value::symbol("italic");
    let underline = Value::symbol("underline");
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        buffer.insert("abcdef");
    }
    eval.buffers
        .put_buffer_text_property_in_emacs_byte_range(
            id,
            EmacsByteRange::from_usize(0, 2),
            face,
            bold,
        )
        .expect("scratch buffer");
    eval.buffers
        .put_buffer_text_property_in_emacs_byte_range(
            id,
            EmacsByteRange::from_usize(2, 4),
            face,
            italic,
        )
        .expect("scratch buffer");
    eval.buffers
        .configure_buffer_undo_list(id, Value::NIL)
        .expect("scratch buffer");
    eval.buffers
        .set_buffer_text_properties_in_emacs_byte_range(
            id,
            EmacsByteRange::from_usize(1, 3),
            vec![(face, underline)],
        )
        .expect("scratch buffer");
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        let mut ul = buffer.get_undo_list();
        crate::buffer::undo::undo_list_boundary(&mut ul);
        buffer.set_undo_list(ul);
    }

    builtin_undo(&mut eval, vec![]).unwrap();

    let buffer = eval.buffers.current_buffer().expect("scratch buffer");
    assert_eq!(
        buffer.text_props_get_properties_ordered_at_emacs_byte_pos(
            crate::buffer::EmacsBytePos::new(0)
        ),
        vec![(face, bold)]
    );
    assert_eq!(
        buffer.text_props_get_properties_ordered_at_emacs_byte_pos(
            crate::buffer::EmacsBytePos::new(1)
        ),
        vec![(face, Value::NIL)]
    );
    assert_eq!(
        buffer.text_props_get_properties_ordered_at_emacs_byte_pos(
            crate::buffer::EmacsBytePos::new(2)
        ),
        vec![(face, Value::NIL)]
    );
    assert_eq!(
        buffer.text_props_get_properties_ordered_at_emacs_byte_pos(
            crate::buffer::EmacsBytePos::new(3)
        ),
        vec![(face, italic)]
    );
    assert!(
        buffer
            .text_props_get_properties_ordered_at_emacs_byte_pos(crate::buffer::EmacsBytePos::new(
                4
            ))
            .is_empty()
    );
}

#[test]
fn test_undo_without_boundary_signals_user_error_after_apply() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        buffer.insert("x");
    }
    let result = builtin_undo(&mut eval, vec![Value::fixnum(1)]);
    assert!(result.is_err());
    let contents = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .buffer_string();
    assert_eq!(contents, "");
}

#[test]
fn test_undo_with_non_positive_arg_and_boundary_returns_undo() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    {
        let buffer = eval.buffers.current_buffer_mut().expect("scratch buffer");
        buffer.insert("x");
        let mut ul = buffer.get_undo_list();
        crate::buffer::undo::undo_list_boundary(&mut ul);
        buffer.set_undo_list(ul);
    }
    let result = builtin_undo(&mut eval, vec![Value::fixnum(0)]).unwrap();
    assert_eq!(result, Value::string("Undo"));
    let contents = eval
        .buffers
        .current_buffer()
        .expect("scratch buffer")
        .buffer_string();
    assert_eq!(contents, "x");
}

/// Return the printed form of the current buffer's `buffer-undo-list` in a bare
/// `Context` (whose default `buffer-undo-list` is nil = enabled).
fn undo_list_after(eval: &mut super::super::eval::Context) -> String {
    let value = eval.eval_str("buffer-undo-list").expect("buffer-undo-list");
    super::super::print::print_value(&value)
}

/// GNU `record_first_change` (undo.c:64,210) pushes `(t . MODTIME)` whenever
/// the buffer transitions clean->modified, i.e. on every edit while
/// `MODIFF <= SAVE_MODIFF`.  `(set-buffer-modified-p nil)` resets
/// `SAVE_MODIFF = MODIFF`, so the buffer becomes clean again and the *next*
/// modification must re-emit the first-change marker.  neomacs previously
/// emitted it only once (a sticky `recorded_first_change` flag) and never
/// re-armed, producing `((8 . 9) (3 . 4) (1 . 11) (t . 0))` instead of GNU's
/// `((8 . 9) (3 . 4) (t . 0) (1 . 11) (t . 0))`.
///
/// Driven through the bare `Context` buffer (whose default `buffer-undo-list`
/// is nil = enabled) because the heavier bootstrap harness is unavailable in
/// this build; the recorder path exercised is identical.
#[test]
fn first_change_marker_rearmed_after_set_modified_nil() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    eval.eval_str("(insert \"0123456789\")").unwrap();
    eval.eval_str("(set-buffer-modified-p nil)").unwrap();
    eval.eval_str("(goto-char 3)").unwrap();
    eval.eval_str("(insert \"A\")").unwrap();
    eval.eval_str("(goto-char 8)").unwrap();
    eval.eval_str("(insert \"B\")").unwrap();
    assert_eq!(
        undo_list_after(&mut eval),
        "((8 . 9) (3 . 4) (t . 0) (1 . 11) (t . 0))"
    );
}

/// Without re-cleaning between edits, GNU records the first-change marker only
/// once: after the first edit increments `MODIFF`, later edits in the same
/// modified run see `MODIFF > SAVE_MODIFF` and skip the marker.  Guards against
/// over-eager re-arming (a regression in the other direction).  The original
/// `(1 . 11)` insert and its first-change marker remain on the list because
/// `set-buffer-modified-p nil` does not clear `buffer-undo-list`.
#[test]
fn first_change_marker_recorded_once_per_modified_run() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    eval.eval_str("(insert \"0123456789\")").unwrap();
    eval.eval_str("(set-buffer-modified-p nil)").unwrap();
    eval.eval_str("(insert \"X\")").unwrap();
    eval.eval_str("(insert \"Y\")").unwrap();
    // "X"/"Y" coalesce into one (11 . 13) entry, preceded by a single re-armed
    // first-change marker for the clean->modified transition.
    assert_eq!(
        undo_list_after(&mut eval),
        "((11 . 13) (t . 0) (1 . 11) (t . 0))"
    );
}

/// Re-cleaning between every edit re-arms the first-change marker each time,
/// matching GNU's per-transition behavior.
#[test]
fn first_change_marker_rearmed_on_each_clean_transition() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    eval.eval_str("(insert \"0123456789\")").unwrap();
    eval.eval_str("(set-buffer-modified-p nil)").unwrap();
    eval.eval_str("(insert \"X\")").unwrap();
    eval.eval_str("(set-buffer-modified-p nil)").unwrap();
    eval.eval_str("(insert \"Y\")").unwrap();
    assert_eq!(
        undo_list_after(&mut eval),
        "((12 . 13) (t . 0) (11 . 12) (t . 0) (1 . 11) (t . 0))"
    );
}

/// Back-to-front edits, the shape every LSP-style client applies (`tide` walks
/// TypeScript's `textChanges` in reverse so earlier positions stay valid), must
/// undo to the original text.
///
/// GNU records the two adjacent one-character insertions as `(19 . 20)` and
/// `(20 . 21)` -- `record_insert` (src/undo.c:98-112) coalesces only when the
/// newest record ENDS where the new insertion BEGINS.  neomacs also coalesced
/// in the opposite direction, producing a single `(19 . 21)` whose undo
/// deleted the untouched `=` between the two inserted spaces and turned
/// `total=add` into `total add`.
#[test]
fn undo_of_descending_adjacent_inserts_restores_the_untouched_text() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;

    let mut eval = Context::new();
    let result = eval
        .eval_str(
            r#"(progn
                 (insert "export const total=add(1,2)")
                 (undo-boundary)
                 (goto-char 20)
                 (insert " ")
                 (goto-char 19)
                 (insert " ")
                 (let ((records buffer-undo-list))
                   (primitive-undo 1 buffer-undo-list)
                   (list (buffer-string) (car records) (car (cdr records)))))"#,
        )
        .expect("descending insert undo eval");

    // Transcribed from GNU Emacs -Q --batch (tmp/p97-probe8.el).
    assert_eq!(
        super::super::print::print_value(&result),
        "(\"export const total=add(1,2)\" (19 . 20) (20 . 21))"
    );
}

/// GNU `recursive_edit_1` (keyboard.c:708-748) specbinds
/// `undo-auto--undoably-changed-buffers` to nil before it runs the command
/// loop, "so that changes in the recursive edit will not result in undo
/// boundaries in buffers changed before we entered there recursive edit"
/// (keyboard.c:741-747, Bug #23632).  Both `recursive-edit` and `read_minibuf`
/// enter through it, so reading an argument from the minibuffer must not drop a
/// boundary into a buffer an earlier, already-finished command edited.
///
/// Measured on GNU Emacs -Q --batch (tmp/p97-probe11.el): the undo list is
/// `((1 . 6) (t . 0))` both before and after the read.
#[test]
fn a_minibuffer_read_adds_no_undo_boundary_to_previously_changed_buffers() {
    crate::test_utils::init_test_tracing();

    let mut eval = crate::test_utils::runtime_startup_context();
    let result = eval
        .eval_str(
            r#"(let ((buf (get-buffer-create "neo-undo-boundary")))
                 (set-buffer buf)
                 (buffer-enable-undo)
                 (setq buffer-undo-list nil)
                 (insert "hello")
                 (let ((setup
                        (lambda ()
                          (setq unread-command-events
                                (append (listify-key-sequence (kbd "z RET"))
                                        unread-command-events)))))
                   (add-hook 'minibuffer-setup-hook setup)
                   (unwind-protect
                       (let ((executing-kbd-macro t))
                         (read-from-minibuffer "P: "))
                     (remove-hook 'minibuffer-setup-hook setup)))
                 (with-current-buffer buf
                   (let ((boundaries 0))
                     (dolist (entry buffer-undo-list)
                       (unless entry (setq boundaries (1+ boundaries))))
                     (list boundaries (length buffer-undo-list)))))"#,
        )
        .expect("minibuffer read should finish");

    assert_eq!(format!("{result}"), "(0 2)");
}

/// GNU's first-change entry is `(t . VISITED-FILE-MODTIME)`, not a constant:
/// `record_first_change` stores `buffer_visited_file_modtime (base_buffer)`
/// (`src/undo.c:209-223`), and `primitive-undo`'s `(t . TIME)` arm
/// (`lisp/simple.el:3669-3688`) clears the modified flag only when
/// `(time-equal-p time (visited-file-modtime))` -- so that undoing back to a
/// save the file has since outlived does NOT claim the buffer is unmodified.
///
/// Neomacs recorded the fixnum 0 for every buffer, so a file-visiting buffer's
/// comparison could never match and `undo` back to the saved text left
/// `buffer-modified-p` t where GNU reports nil.  The org-ref completion
/// workflow reads exactly that flag after `C-_`.
///
/// This pins the recorder, which is the part Neomacs owns -- `primitive-undo`
/// itself is GNU's `simple.el` and is not loaded in a bare `Context`.  The
/// entry's datum must be exactly what `visited-file-modtime` reports for the
/// same buffer, and for a file-visiting buffer that is a timestamp, never the
/// no-modtime 0.  The full flow reads
/// `(:before-undo t :text "" :after-undo nil :modtime-recorded t)` under GNU
/// Emacs 31.0.90.
#[test]
fn first_change_marker_records_the_visited_file_modtime_like_gnu() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("neovm-undo-first-change-{unique}"));
    std::fs::create_dir_all(&dir).expect("create temp fixture dir");
    let file = dir.join("probe.txt");
    std::fs::write(&file, "").expect("write empty fixture");

    let form = format!(
        r#"(progn
             (setq buffer-file-name {:?})
             (set-visited-file-modtime)
             (setq buffer-undo-list nil)
             (set-buffer-modified-p nil)
             (insert "hello")
             (list :recorded (cdr (assq t buffer-undo-list))
                   :matches (equal (cdr (assq t buffer-undo-list))
                                   (visited-file-modtime))))"#,
        file.to_string_lossy()
    );
    let rendered = super::super::print::print_value(&eval.eval_str(&form).expect("eval"));
    assert!(
        rendered.ends_with(":matches t)"),
        "the first-change datum must be the buffer's visited-file modtime: {rendered}"
    );
    assert!(
        !rendered.starts_with("(:recorded 0 "),
        "a file-visiting buffer records a timestamp, not GNU's no-modtime 0: {rendered}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// GNU `record_point` (`src/undo.c:47-78`) reads `at_boundary` from the undo
/// list BEFORE `record_first_change` may cons `(t . TIME)` onto it, and only
/// then pushes `point_before_last_command_or_undo`.  GNU's own comment on that
/// read names the hazard: "This check is currently dependent on being called
/// before record_first_change".
///
/// Neomacs ran the first-change step first and let each recorder re-derive
/// `at_boundary` afterwards, so on the very case the check exists for -- the
/// first change to a CLEAN buffer -- the list was no longer at a boundary and
/// the point entry was dropped.  `primitive-undo` then had nothing to restore
/// point from and `undo` left point at the change instead of where the command
/// started (Tide's format/undo-only workflow read point 206 for GNU's 1).
///
/// The expected list is what GNU Emacs 31.0.90 prints for this exact form
/// under `emacs -Q --batch`:
///
/// ```text
/// undo-list=((7 . 8) 1 (t 27263 42178 675008 795000))
/// ```
#[test]
fn record_point_runs_before_the_first_change_sentinel_like_gnu() {
    crate::test_utils::init_test_tracing();
    use super::super::eval::Context;
    let mut eval = Context::new();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("neovm-undo-record-point-{unique}"));
    std::fs::create_dir_all(&dir).expect("create temp fixture dir");
    let file = dir.join("probe.txt");
    std::fs::write(&file, "").expect("write empty fixture");

    let form = format!(
        r#"(progn
             (setq buffer-file-name {:?})
             (set-visited-file-modtime)
             (insert "abcdef")
             (setq buffer-undo-list nil)
             (set-buffer-modified-p nil)
             (goto-char (point-min))
             (undo-boundary)
             (save-excursion (goto-char (point-max)) (insert "X"))
             (list :newest (car buffer-undo-list)
                   :point-entry (nth 1 buffer-undo-list)
                   :first-change (car-safe (nth 2 buffer-undo-list))
                   :length (length buffer-undo-list)))"#,
        file.to_string_lossy()
    );
    let rendered = super::super::print::print_value(&eval.eval_str(&form).expect("eval"));
    assert_eq!(
        rendered, "(:newest (7 . 8) :point-entry 1 :first-change t :length 3)",
        "the point entry belongs between the change and the first-change sentinel"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
