//! Oracle parity for buffer editing: `delete-char`, `delete-region`,
//! `erase-buffer`, `insert`, `insert-char`, `insert-byte`,
//! `delete-and-extract-region`.
//!
//! GNU src/cmds.c, src/editfns.c, src/insdel.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_insert_adds_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed*")) (erase-buffer) (insert "hello") (buffer-string))"#,
    );
    assert_ok_eq("\"hello\"", &o, &n);
}

#[test]
fn oracle_insert_multiple_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed2*")) (erase-buffer) (insert "a" "b" "c") (buffer-string))"#,
    );
    assert_ok_eq("\"abc\"", &o, &n);
}

#[test]
fn oracle_insert_char_inserts_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed3*")) (erase-buffer) (insert-char ?x 3) (buffer-string))"#,
    );
    assert_ok_eq("\"xxx\"", &o, &n);
}

#[test]
fn oracle_insert_byte_inserts_byte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed4*")) (erase-buffer) (insert-byte 65 1) (buffer-string))"#,
    );
    assert_ok_eq("\"A\"", &o, &n);
}

#[test]
fn oracle_erase_buffer_clears() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed5*")) (insert "junk") (erase-buffer) (buffer-string))"#,
    );
    assert_ok_eq("\"\"", &o, &n);
}

#[test]
fn oracle_delete_char_removes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed6*")) (erase-buffer) (insert "abcde") (goto-char 3) (delete-char 2) (buffer-string))"#,
    );
    assert_ok_eq("\"abe\"", &o, &n);
}

#[test]
fn oracle_delete_region_removes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed7*")) (erase-buffer) (insert "0123456789") (delete-region 3 6) (buffer-string))"#,
    );
    assert_ok_eq("\"0156789\"", &o, &n);
}

#[test]
fn oracle_delete_and_extract_returns_removed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed8*")) (erase-buffer) (insert "hello world") (delete-and-extract-region 1 6))"#,
    );
    assert_ok_eq("\"hello\"", &o, &n);
}

#[test]
fn oracle_delete_char_negative_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*ed9*")) (erase-buffer) (insert "abcde") (goto-char 4) (delete-char -2) (buffer-string))"#,
    );
    assert_ok_eq("\"ade\"", &o, &n);
}
