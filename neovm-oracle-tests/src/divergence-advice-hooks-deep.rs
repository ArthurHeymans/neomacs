//! Divergence tests: advice, hooks, before/after/around deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_advice_add_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defun test-advice-fn-xxx () 42)
  (advice-add 'test-advice-fn-xxx :around
    (lambda (fn &rest args) (1+ (apply fn args))))
  (list (test-advice-fn-xxx)
        (progn
          (advice-remove 'test-advice-fn-xxx
            (lambda (fn &rest args) (1+ (apply fn args))))
          (test-advice-fn-xxx)))) "#,
    );
}

#[test]
fn divergence_advice_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'advice-add)
  (fboundp 'advice-remove)
  (fboundp 'advice-mapc)
  (member :before '(before after around override))
  (member :after '(before after around override))) "#,
    );
}

#[test]
fn divergence_hooks_add_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar test-hook-var-xxx nil)
  (add-hook 'test-hook-var-xxx (lambda () 'hook-called))
  (list test-hook-var-xxx
        (progn
          (remove-hook 'test-hook-var-xxx (lambda () 'hook-called))
          test-hook-var-xxx))) "#,
    );
}

#[test]
fn divergence_hook_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar test-hook-depth-xxx nil)
  (add-hook 'test-hook-depth-xxx 'append-fn-xxx nil t)
  (list (listp test-hook-depth-xxx)
        (boundp 'test-hook-depth-xxx)
        (remove-hook 'test-hook-depth-xxx 'append-fn-xxx)
        test-hook-depth-xxx)) "#,
    );
}

#[test]
fn divergence_run_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'run-hooks)
  (fboundp 'run-hook-with-args)
  (fboundp 'run-hook-with-args-until-success)
  (fboundp 'run-hook-with-args-until-failure))"#,
    );
}

#[test]
fn divergence_add_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'add-function)
  (fboundp 'remove-function)
  (fboundp 'function-put)
  (fboundp 'function-get))"#,
    );
}

#[test]
fn divergence_narrowed_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'change-major-mode-hook)
  (boundp 'after-change-major-mode-hook)
  (listp change-major-mode-hook)
  (listp after-change-major-mode-hook))"#,
    );
}

#[test]
fn divergence_find_file_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'find-file-hook)
  (listp find-file-hook)
  (member 'find-file-hook (apropos-internal "hook"))) "#,
    );
}

#[test]
fn divergence_post_command_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'post-command-hook)
  (boundp 'pre-command-hook)
  (listp post-command-hook)
  (listp pre-command-hook))"#,
    );
}

#[test]
fn divergence_idle_timer_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'run-with-idle-timer)
  (fboundp 'run-at-time)
  (fboundp 'cancel-timer)
  (fboundp 'timerp)
  (fboundp 'current-idle-time))"#,
    );
}
