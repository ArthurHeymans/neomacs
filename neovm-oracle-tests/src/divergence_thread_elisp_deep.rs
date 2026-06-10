//! Divergence tests: thread macro, threading-first/last deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_thread_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (thread-first 5
    (+ 3)
    (* 2)
    (- 1))
  (= (thread-first 5 (+ 3) (* 2) (- 1)) 15)) "#,
    );
}

#[test]
fn divergence_thread_last() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (thread-last '(1 2 3)
    (mapcar #'1+)
    (-filter #'cl-evenp)
    (length))
  (= (thread-last '(1 2 3) (mapcar #'1+) (-filter #'cl-evenp) (length)) 1)) "#,
    );
}

#[test]
fn divergence_thread_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (thread-first "hello"
    (concat " " "world")
    (upcase))
  (equal (thread-first "hello"
           (concat " " "world")
           (upcase))
         "HELLO WORLD")) "#,
    );
}

#[test]
fn divergence_dash_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp '-map)
  (fboundp '-filter)
  (fboundp '-reduce)
  (fboundp '-flatten)
  (featurep 'dash)
  (featurep 'dash-functional)) "#,
    );
}

#[test]
fn divergence_dash_list_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp '-first)
  (fboundp '-last)
  (fboundp '-butlast)
  (fboundp '-drop)
  (fboundp '-take)
  (fboundp '-partition)) "#,
    );
}

#[test]
fn divergence_s_expression_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'sexp-at-point)
  (fboundp 'forward-sexp)
  (fboundp 'backward-sexp)
  (fboundp 'kill-sexp)
  (fboundp 'mark-sexp)
  (fboundp 'transpose-sexps)) "#,
    );
}

#[test]
fn divergence_lisp_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'lisp-mode)
  (fboundp 'emacs-lisp-mode)
  (fboundp 'lisp-interaction-mode)
  (featurep 'lisp-mode)) "#,
    );
}

#[test]
fn divergence_elisp_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'eval-buffer)
  (fboundp 'eval-region)
  (fboundp 'eval-last-sexp)
  (fboundp 'eval-expression)
  (fboundp 'eval-defun)
  (fboundp 'edebug-defun)) "#,
    );
}

#[test]
fn divergence_checkdoc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'checkdoc)
  (fboundp 'checkdoc-current-buffer)
  (fboundp 'checkdoc-file)
  (featurep 'checkdoc)) "#,
    );
}

#[test]
fn divergence_ert_testing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'ert-deftest)
  (fboundp 'ert-run-tests-interactively)
  (featurep 'ert)
  (fboundp 'should)
  (fboundp 'should-not)
  (fboundp 'should-error)) "#,
    );
}
