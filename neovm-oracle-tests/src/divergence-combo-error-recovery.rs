//! Divergence tests: complex error recovery + resource cleanup scenarios.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_temp_buffer_cleanup_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let* ((buf-count-before (length (buffer-list)))
        (result (condition-case err
                    (with-temp-buffer
                      (insert \"temp data\")
                      (error \"forced error\"))
                  (error (list 'caught (car err))))))
  (list result
        (= (length (buffer-list)) buf-count-before))) ",
    );
}

#[test]
fn divergence_unwind_protect_buffer_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((temp-buf nil)
        (log nil))
  (unwind-protect
      (progn
        (setq temp-buf (generate-new-buffer \"*test-cleanup*\"))
        (with-current-buffer temp-buf (insert \"data\"))
        (push 'before-error log)
        (error \"test error\"))
    (push 'cleanup log)
    (when (and temp-buf (buffer-live-p temp-buf))
      (kill-buffer temp-buf)
      (push 'buffer-killed log)))
  (nreverse log)) ",
    );
}

#[test]
fn divergence_nested_unwind_with_multiple_cleanups() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((log nil))
  (ignore-errors
    (unwind-protect
        (unwind-protect
            (unwind-protect
                (error \"inner\")
              (push 'cleanup-1 log))
          (push 'cleanup-2 log))
      (push 'cleanup-3 log)))
  (nreverse log)) ",
    );
}

#[test]
fn divergence_condition_case_error_data_integrity() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (condition-case err
      (car 42)
    (wrong-type-argument (list (car err) (length (cdr err)) (cdr err))))
  (condition-case err
      (nth 10 '(a b c))
    (args-out-of-range (list (car err) (cdr err))))
  (condition-case err
      (/ 1 0)
    (arith-error (list (car err) (cdr err))))) ",
    );
}

#[test]
fn divergence_save_excursion_restore_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(progn
  (insert \"ABCDEFGHIJ\")
  (let ((orig-point (point)))
    (ignore-errors
      (save-excursion
        (goto-char 5)
        (error \"oops\")))
    (list (= (point) orig-point)
          (point)
          orig-point))) ",
    );
}

#[test]
fn divergence_save_restriction_restore_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(progn
  (insert \"ABCDEFGHIJ\")
  (narrow-to-region 3 7)
  (let ((orig-min (point-min)) (orig-max (point-max)))
    (ignore-errors
      (save-restriction
        (widen)
        (error \"oops\")))
    (list (point-min) (point-max) orig-min orig-max
          (= (point-min) orig-min)
          (= (point-max) orig-max)))) ",
    );
}

#[test]
fn divergence_marker_recovery_after_failed_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(progn
  (insert \"ABCDEFGHIJ\")
  (let ((m (set-marker (make-marker) 5)))
    (undo-boundary)
    (ignore-errors
      (goto-char 5)
      (insert \"123\")
      (error \"fail\"))
    (list (marker-position m)
          (buffer-string)))) ",
    );
}

#[test]
fn deficiency_error_message_formatting() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (condition-case err
      (error \"test %s %d\" \"hello\" 42)
    (error (error-message-string err)))
  (condition-case err
      (signal 'wrong-type-argument '(listp 42))
    (wrong-type-argument (error-message-string err)))
  (condition-case err
      (signal 'args-out-of-range '(10 5))
    (args-out-of-range (error-message-string err)))) ",
    );
}

#[test]
fn divergence_signal_vs_error_condition_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (condition-case err
      (signal 'custom-error-signal-xxx '(\"data\"))
    (custom-error-signal-xxx (list 'custom (cdr err)))
    (error (list 'generic (cdr err))))
  (condition-case err
      (signal 'error '(\"generic\"))
    (error (list 'error-handler (cdr err))))) ",
    );
}

#[test]
fn divergence_user_error_vs_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (condition-case err
      (user-error \"user mistake\")
    (user-error (list 'user (error-message-string err)))
    (error (list 'error (error-message-string err))))
  (condition-case err
      (error \"real error\")
    (user-error (list 'user (error-message-string err)))
    (error (list 'error (error-message-string err))))) ",
    );
}
