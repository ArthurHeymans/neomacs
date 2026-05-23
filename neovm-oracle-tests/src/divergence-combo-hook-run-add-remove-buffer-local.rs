//! Deep combo: hook execution + run-hooks + add-hook + remove-hook + buffer-local hooks.
//! Tests hook system with ordering, buffer-local, and depth semantics.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_add_hook_and_run_hooks_ordering() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar my-hook nil)\n\
         (setq my-hook nil)\n\
         (add-hook 'my-hook (lambda () (push 'first my-hook-log)) nil t)\n\
         (add-hook 'my-hook (lambda () (push 'second my-hook-log)) nil t)\n\
         (defvar my-hook-log nil)\n\
         (run-hooks 'my-hook)\n\
         (nreverse my-hook-log))",
    );
}

#[test]
fn deficiency_add_hook_append_vs_prepend() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar ord-hook nil)\n\
         (defvar ord-log nil)\n\
         (setq ord-hook nil ord-log nil)\n\
         (add-hook 'ord-hook (lambda () (push 'first ord-log)))\n\
         (add-hook 'ord-hook (lambda () (push 'second ord-log)) t)\n\
         (run-hooks 'ord-hook)\n\
         (nreverse ord-log))",
    );
}

#[test]
fn deficiency_remove_hook_mid_execution() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar rem-hook nil)\n\
         (defvar rem-log nil)\n\
         (setq rem-hook nil rem-log nil)\n\
         (defun rem-fn1 () (push 'fn1 rem-log))\n\
         (defun rem-fn2 () (push 'fn2 rem-log))\n\
         (add-hook 'rem-hook 'rem-fn1)\n\
         (add-hook 'rem-hook 'rem-fn2)\n\
         (run-hooks 'rem-hook)\n\
         (remove-hook 'rem-hook 'rem-fn2)\n\
         (run-hooks 'rem-hook)\n\
         (list (nreverse rem-log)))",
    );
}

#[test]
fn deficiency_hook_with_args_via_run_hook_with_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar arg-hook nil)\n\
         (defvar arg-log nil)\n\
         (setq arg-hook nil arg-log nil)\n\
         (add-hook 'arg-hook (lambda (x) (push x arg-log)))\n\
         (run-hook-with-args 'arg-hook 42)\n\
         (run-hook-with-args 'arg-hook 99)\n\
         (nreverse arg-log))",
    );
}

#[test]
fn deficiency_buffer_local_hook_isolation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar blh-hook nil)\n\
         (setq blh-hook nil)\n\
         (defvar blh-log nil)\n\
         (setq blh-log nil)\n\
         (add-hook 'blh-hook (lambda () (push 'global blh-log)))\n\
         (let ((buf (generate-new-buffer \"blh\")))\n\
         (with-current-buffer buf\n\
         (make-local-variable 'blh-hook)\n\
         (add-hook 'blh-hook (lambda () (push 'local blh-log))))\n\
         (run-hooks 'blh-hook)\n\
         (with-current-buffer buf\n\
         (run-hooks 'blh-hook))\n\
         (kill-buffer buf)\n\
         (nreverse blh-log)))",
    );
}

#[test]
fn deficiency_hook_depth_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar depth-hook nil)\n\
         (defvar depth-log nil)\n\
         (setq depth-hook nil depth-log nil)\n\
         (add-hook 'depth-hook (lambda () (push 'depth-90 depth-log)) 90)\n\
         (add-hook 'depth-hook (lambda () (push 'depth-10 depth-log)) 10)\n\
         (add-hook 'depth-hook (lambda () (push 'depth-50 depth-log)) 50)\n\
         (run-hooks 'depth-hook)\n\
         (nreverse depth-log))",
    );
}

#[test]
fn deficiency_hook_nil_does_nothing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar nil-hook nil)\n\
         (setq nil-hook nil)\n\
         (list (run-hooks 'nil-hook)\n\
         (null nil-hook)))",
    );
}

#[test]
fn deficiency_run_hook_wrapped_catches_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar wrap-hook nil)\n\
         (defvar wrap-log nil)\n\
         (setq wrap-hook nil wrap-log nil)\n\
         (add-hook 'wrap-hook (lambda () (push 'before wrap-log)))\n\
         (add-hook 'wrap-hook (lambda () (error \"boom\")))\n\
         (add-hook 'wrap-hook (lambda () (push 'after wrap-log)))\n\
         (condition-case err\n\
         (run-hooks 'wrap-hook)\n\
         (error (push (car err) wrap-log)))\n\
         (nreverse wrap-log))",
    );
}

#[test]
fn deficiency_run_hook_until_failure() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar fail-hook nil)\n\
         (defvar fail-log nil)\n\
         (setq fail-hook nil fail-log nil)\n\
         (add-hook 'fail-hook (lambda () (push 'first fail-log)))\n\
         (add-hook 'fail-hook (lambda () (push 'second fail-log) nil))\n\
         (add-hook 'fail-hook (lambda () (push 'third fail-log)))\n\
         (run-hook-with-args-until-failure 'fail-hook)\n\
         (nreverse fail-log))",
    );
}

#[test]
fn deficiency_hook_symbol_value_is_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar sv-hook nil)\n\
         (setq sv-hook nil)\n\
         (add-hook 'sv-hook 'sv-fn1)\n\
         (add-hook 'sv-hook 'sv-fn2)\n\
         (defun sv-fn1 () t)\n\
         (defun sv-fn2 () t)\n\
         (list (listp sv-hook)\n\
         (length sv-hook)\n\
         (memq 'sv-fn1 sv-hook)\n\
         (memq 'sv-fn2 sv-hook)))",
    );
}
