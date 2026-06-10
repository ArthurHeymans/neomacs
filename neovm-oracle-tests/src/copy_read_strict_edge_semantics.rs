//! Oracle parity tests for `copy-sequence` across types + `read-from-string`
//! return value handling (cons of VALUE and END-POSITION).
//!
//! GNU src/fns.c `Fcopy_sequence`: dispatches on type (cons, string,
//! vector, bool-vector, char-table, record). Returns equal but not eq copy.
//! GNU src/lread.c: `read-from-string` returns (VALUE . END-POSITION).

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// copy-sequence
// ---------------------------------------------------------------------------

#[test]
fn oracle_copy_sequence_list_equal_not_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (let ((orig '(a b c)))
    (let ((cpy (copy-sequence orig)))
      (list (equal orig cpy) (eq orig cpy)))))"#,
    );
    assert_ok_eq("(t nil)", &o, &n);
}

#[test]
fn oracle_copy_sequence_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(equal "hello" (copy-sequence "hello"))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_copy_sequence_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(equal [1 2 3] (copy-sequence [1 2 3]))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_copy_sequence_nil_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(copy-sequence nil)"#);
    assert_ok_eq("nil", &o, &n);
}

// ---------------------------------------------------------------------------
// read-from-string returns (VALUE . END-POSITION) cons
// ---------------------------------------------------------------------------

#[test]
fn oracle_read_from_string_string_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(car (read-from-string "\"hello\""))"#);
    assert_ok_eq("\"hello\"", &o, &n);
}

#[test]
fn oracle_read_from_string_integer_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(car (read-from-string "42"))"#);
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_read_from_string_list_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(car (read-from-string "(a b c)"))"#);
    assert_ok_eq("(a b c)", &o, &n);
}

#[test]
fn oracle_read_from_string_returns_cons_of_value_and_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(consp (read-from-string "42"))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_read_from_string_end_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(> (cdr (read-from-string "hello")) 0)"#);
    assert_ok_eq("t", &o, &n);
}
