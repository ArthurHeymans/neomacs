//! Oracle parity for Lisp-level constructs via binary execution.
//! Uses eval_oracle_and_neovm to test defun, push/pop,
//! dotimes/dolist, with-temp-buffer, setq-default, add-hook/run-hooks.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- defun ---

#[test]
fn oracle_defun_and_funcall_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defun neovm--test-binary-fn (x) (+ x 1))
  (neovm--test-binary-fn 41))"#,
    );
    assert_ok_eq("42", &o, &n);
}

// --- push / pop ---

#[test]
fn oracle_push_pop_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (setq neovm--test-stack nil)
  (push 'a neovm--test-stack)
  (push 'b neovm--test-stack)
  (list (pop neovm--test-stack) neovm--test-stack))"#,
    );
    // pop returns the top (b), stack becomes (a)
    assert_ok_eq("(b (a))", &o, &n);
}

// --- dotimes ---

#[test]
fn oracle_dotimes_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (setq neovm--test-sum 0)
  (dotimes (i 5 neovm--test-sum)
    (setq neovm--test-sum (+ neovm--test-sum i))))"#,
    );
    assert_ok_eq("10", &o, &n);
}

// --- dolist ---

#[test]
fn oracle_dolist_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (setq neovm--test-collect nil)
  (dolist (elt '(a b c) (nreverse neovm--test-collect))
    (push elt neovm--test-collect)))"#,
    );
    assert_ok_eq("(a b c)", &o, &n);
}

// --- with-temp-buffer ---

#[test]
fn oracle_with_temp_buffer_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (with-temp-buffer
    (insert "hello")
    (buffer-string)))"#,
    );
    assert_ok_eq("\"hello\"", &o, &n);
}

// --- setq-default ---

#[test]
fn oracle_setq_default_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (setq-default neovm--test-default-var 999)
  neovm--test-default-var)"#,
    );
    assert_ok_eq("999", &o, &n);
}

// --- add-hook / run-hooks ---

#[test]
fn oracle_add_hook_and_run_hooks_via_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (setq neovm--test-hook-result nil)
  (add-hook 'neovm--test-binary-hook (lambda () (push 'ran neovm--test-hook-result)))
  (run-hooks 'neovm--test-binary-hook)
  neovm--test-hook-result)"#,
    );
    assert_ok_eq("(ran)", &o, &n);
}
