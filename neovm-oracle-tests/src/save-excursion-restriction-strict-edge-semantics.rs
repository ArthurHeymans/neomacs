//! Oracle parity for save-excursion, save-restriction,
//! save-current-buffer interactions.
//! GNU src/editfns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_save_excursion_restores_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*se*")) (erase-buffer) (insert "0123456789") (goto-char 3) (save-excursion (goto-char 7) (point)) (point))"#,
    );
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_save_excursion_restores_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (let ((orig (current-buffer)) (other (get-buffer-create "*se2*"))) (save-excursion (set-buffer other) 'switched) (eq orig (current-buffer))))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_save_restriction_restores() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*sr*")) (erase-buffer) (insert "0123456789") (save-restriction (narrow-to-region 3 7) (point-min)) (point-min))"#,
    );
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_save_current_buffer_restores() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (save-current-buffer (set-buffer (get-buffer-create "*scb*"))) (run-hooks 'ignore))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_save_excursion_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*sen*")) (erase-buffer) (insert "0123456789") (goto-char 2) (save-excursion (goto-char 5) (save-excursion (goto-char 8) (point))) (point))"#,
    );
    assert_ok_eq("2", &o, &n);
}
