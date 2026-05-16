//! Oracle parity for pos + read + byte-position ops.
//! GNU src/editfns.c, src/lread.c, src/marker.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_pos_bol_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*pb*")) (erase-buffer) (insert "abc\ndef") (goto-char 6) (pos-bol))"#,
    );
    assert_ok_eq("5", &o, &n);
}

#[test]
fn oracle_pos_eol_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*pe*")) (erase-buffer) (insert "abc\ndef") (goto-char 2) (pos-eol))"#,
    );
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_position_bytes_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*pb2*")) (erase-buffer) (insert "hello") (integerp (position-bytes 3)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_byte_to_position_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*bp*")) (erase-buffer) (insert "hello") (integerp (byte-to-position 3)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_read_from_string_multiple_forms() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(car (read-from-string "1 2 3"))"#);
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_read_from_string_with_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(car (read-from-string "a b c" 4))"#);
    assert_ok_eq("c", &o, &n);
}

#[test]
fn oracle_read_from_string_returns_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(consp (read-from-string "42"))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_read_from_string_end_pos_greater_than_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(> (cdr (read-from-string "hello")) 0)"#);
    assert_ok_eq("t", &o, &n);
}
