//! Oracle parity for buffer movement: `goto-char`, `forward-char`,
//! `forward-line`, `beginning-of-line`, `end-of-line`, `point`,
//! `point-min`, `point-max`.
//!
//! GNU src/editfns.c, src/cmds.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_goto_char_sets_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv*")) (erase-buffer) (insert "0123456789") (goto-char 5) (point))"#,
    );
    assert_ok_eq("5", &o, &n);
}

#[test]
fn oracle_goto_char_out_of_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv2*")) (erase-buffer) (goto-char 999) (>= (point) 1))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_forward_char_moves_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv3*")) (erase-buffer) (insert "0123456789") (goto-char 1) (forward-char 3) (point))"#,
    );
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_forward_line_moves() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv4*")) (erase-buffer) (insert "line1\nline2\nline3") (goto-char 1) (forward-line 2) (point))"#,
    );
    assert_ok_eq("13", &o, &n);
}

#[test]
fn oracle_beginning_of_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv5*")) (erase-buffer) (insert "abc\ndef") (goto-char 7) (beginning-of-line) (point))"#,
    );
    assert_ok_eq("5", &o, &n);
}

#[test]
fn oracle_end_of_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv6*")) (erase-buffer) (insert "abc\ndef") (goto-char 1) (end-of-line) (point))"#,
    );
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_point_min_is_one() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv7*")) (erase-buffer) (= (point-min) 1))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_point_max_after_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*mv8*")) (erase-buffer) (insert "12345") (= (point-max) 6))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_forward_char_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(forward-char 'a)"#);
    assert_err_kind(&o, &n, "wrong-type-argument");
}
