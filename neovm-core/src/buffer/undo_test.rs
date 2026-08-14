use super::*;

fn c(pos: usize) -> CharPos0 {
    CharPos0::new(pos)
}

fn clen(len: usize) -> CharLen {
    CharLen::new(len)
}

#[test]
fn basic_insert_undo() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::NIL;
    undo_list_record_insert(&mut list, c(0), clen(5), None);
    undo_list_record_insert(&mut list, c(5), clen(3), None);
    undo_list_boundary(&mut list);

    // Should have: nil, (1 . 9) [merged], at minimum
    // Actually the second insert merges with the first: (1 . 9)
    assert!(undo_list_has_trailing_boundary(&list));

    let group = undo_list_pop_group(&mut list);
    assert_eq!(group.len(), 1); // merged into one entry
    let entry = group[0];
    assert!(entry.is_cons());
    assert_eq!(entry.cons_car(), Value::fixnum(1));
    assert_eq!(entry.cons_cdr(), Value::fixnum(9));
}

#[test]
fn delete_records_text() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::NIL;
    undo_list_record_delete(
        &mut list,
        c(3),
        crate::heap_types::LispString::from_unibyte(b"hello".to_vec()),
        c(3),
        None,
    );
    undo_list_boundary(&mut list);

    let group = undo_list_pop_group(&mut list);
    assert_eq!(group.len(), 1);
    let entry = group[0];
    assert!(entry.is_cons());
    let car = entry.cons_car();
    assert!(car.is_string());
    // POS should be positive (4) because pt==beg
    assert_eq!(entry.cons_cdr(), Value::fixnum(4));
}

#[test]
fn boundary_separates_groups() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::NIL;
    undo_list_record_insert(&mut list, c(0), clen(1), None);
    undo_list_boundary(&mut list);
    undo_list_record_insert(&mut list, c(1), clen(1), None);
    undo_list_boundary(&mut list);

    let g2 = undo_list_pop_group(&mut list);
    assert_eq!(g2.len(), 1);
    let entry = g2[0];
    assert!(entry.is_cons());
    assert_eq!(entry.cons_car(), Value::fixnum(2)); // 1+1
    assert_eq!(entry.cons_cdr(), Value::fixnum(3)); // 1+1+1

    let g1 = undo_list_pop_group(&mut list);
    assert_eq!(g1.len(), 1);
    let entry = g1[0];
    assert!(entry.is_cons());
    assert_eq!(entry.cons_car(), Value::fixnum(1)); // 0+1
    assert_eq!(entry.cons_cdr(), Value::fixnum(2)); // 0+1+1
}

#[test]
fn disabled_records_nothing() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::T;
    undo_list_record_insert(&mut list, c(0), clen(5), None);
    assert!(undo_list_is_disabled(&list));
}

#[test]
fn cursor_move_dedup() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::NIL;
    undo_list_record_point(&mut list, c(5));
    undo_list_record_point(&mut list, c(5));
    undo_list_record_point(&mut list, c(5));
    // Should only have one entry
    assert!(list.is_cons());
    assert_eq!(list.cons_car(), Value::fixnum(6));
    assert!(list.cons_cdr().is_nil());

    undo_list_record_point(&mut list, c(10));
    // Now should have two entries
    assert!(list.is_cons());
    assert_eq!(list.cons_car(), Value::fixnum(11));
}

#[test]
fn no_double_boundary() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::NIL;
    undo_list_record_insert(&mut list, c(0), clen(1), None);
    undo_list_boundary(&mut list);
    undo_list_boundary(&mut list);
    undo_list_boundary(&mut list);
    // Only one boundary after the insert
    assert!(undo_list_has_trailing_boundary(&list));
    // Pop it: boundary + insert = 1 record in group
    let group = undo_list_pop_group(&mut list);
    assert_eq!(group.len(), 1);
}

/// GNU `record_insert` (src/undo.c:98-112) coalesces a new insertion into the
/// newest record in exactly one direction: when that record is a `(BEG . END)`
/// insertion whose END equals the new insertion's BEG.  There is no reverse
/// rule -- an insertion that ENDS where the newest record BEGINS stays its own
/// record, because `primitive-undo` replays the records in order and each
/// later record's positions are read against the buffer the earlier deletions
/// already reshaped.
///
/// Descending edits are the ordinary case: `tide-apply-edits` walks a
/// TypeScript `textChanges` list back-to-front so earlier positions stay valid,
/// which produces exactly this shape.  Verified on GNU Emacs -Q --batch: two
/// one-character insertions at 20 then 19 leave `((19 . 20) (20 . 21))`.
#[test]
fn descending_adjacent_inserts_stay_separate_records_like_gnu() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::NIL;
    // Insert one character at 1-indexed 20, then one at 1-indexed 19.
    undo_list_record_insert(&mut list, c(19), clen(1), None);
    undo_list_record_insert(&mut list, c(18), clen(1), None);

    let newest = list.cons_car();
    assert_eq!(newest.cons_car(), Value::fixnum(19));
    assert_eq!(newest.cons_cdr(), Value::fixnum(20));

    let older = list.cons_cdr().cons_car();
    assert_eq!(older.cons_car(), Value::fixnum(20));
    assert_eq!(older.cons_cdr(), Value::fixnum(21));

    assert!(list.cons_cdr().cons_cdr().is_nil());
}

#[test]
fn to_value_produces_list() {
    crate::test_utils::init_test_tracing();
    let mut list = Value::NIL;
    undo_list_record_insert(&mut list, c(0), clen(5), None);
    undo_list_boundary(&mut list);
    assert!(list.is_list());
}

#[test]
fn undoing_flag_not_needed() {
    crate::test_utils::init_test_tracing();
    // The undoing flag is now tracked on Buffer, not in the undo list itself.
    // This test just verifies that disabled lists don't record.
    let mut list = Value::T; // disabled
    undo_list_record_insert(&mut list, c(0), clen(5), None);
    assert!(undo_list_is_disabled(&list));

    let mut list2 = Value::NIL; // enabled
    undo_list_record_insert(&mut list2, c(0), clen(5), None);
    assert!(!undo_list_is_empty(&list2));
}
