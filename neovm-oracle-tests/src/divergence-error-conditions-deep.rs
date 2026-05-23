//! Divergence tests: error conditions, condition-case, signal deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_condition_case_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (condition-case err
      (signal 'error "test")
    (error (list 'caught err)))
  (condition-case err
      (signal 'wrong-type-argument "test")
    (wrong-type-argument (list 'caught err))
    (error (list 'caught-error err)))) "#,
    );
}

#[test]
fn divergence_condition_case_hierarchy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (condition-case err
      (signal 'args-out-of-range '(0 10))
    (args-out-of-range 'args-caught)
    (wrong-type-argument 'wrong-type-caught)
    (error 'error-caught))) "#,
    );
}

#[test]
fn divergence_error_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'define-error)
  (fboundp 'signal)
  (fboundp 'error)
  (fboundp 'warn)
  (fboundp 'user-error)
  (fboundp 'message)) "#,
    );
}

#[test]
fn divergence_unwind_protect() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((result nil))
  (unwind-protect
      (progn (push 'body result)
             (signal 'error "test"))
    (push 'cleanup result))
  result) "#,
    );
}

#[test]
fn divergence_with_condition_unwind() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((result nil))
  (condition-case err
      (unwind-protect
          (progn
            (push 'body result)
            (signal 'error "test"))
        (push 'cleanup result))
    (error (push 'handler result)))
  result) "#,
    );
}

#[test]
fn divergence_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'error-message-string)
  (stringp (error-message-string '(error "test message")))
  (fboundp 'format-message)) "#,
    );
}

#[test]
fn divergence_debug_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'debug-on-error)
  (booleanp debug-on-error)
  (boundp 'debug-on-quit)
  (booleanp debug-on-quit)) "#,
    );
}

#[test]
fn divergence_signal_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'signal)
  (fboundp 'error)
  (fboundp 'user-error)
  (condition-case nil
      (user-error "test")
    (user-error 'user-error-caught)
    (error 'error-caught))) "#,
    );
}

#[test]
fn divergence_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(condition-case err
    (car "not-a-list")
  (wrong-type-argument
   (list 'caught (car err) (cdr err)))) "#,
    );
}

#[test]
fn divergence_void_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(condition-case err
    (nonexistent-function-xyz-123)
  (void-function
   (list 'caught (car err)))
  (error
   (list 'caught-error (car err)))) "#,
    );
}
