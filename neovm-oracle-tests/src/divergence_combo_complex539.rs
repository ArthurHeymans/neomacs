/// Batch 539: condition-case in all forms, handler-bind, signal error conditions.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx539_condition_case_simple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (+ 1 2) (error (car e)))
"##,
    );
}

#[test]
fn div_cx539_condition_case_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (error "fail") (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx539_condition_case_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (car 1 2)
  (wrong-number-of-arguments "wrong-num")
  (error "other"))
"##,
    );
}

#[test]
fn div_cx539_condition_case_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e1
    (condition-case e2
        (error "nested")
      (error (format "inner: %s" (cadr e2))))
  (error (format "outer: %s" (cadr e1))))
"##,
    );
}

#[test]
fn div_cx539_condition_case_unless() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case-unless-debug e
    (+ "a" 1)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx539_handler_bind() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (handler-bind ((error (lambda (e) (message "caught: %S" e))))
      (error "test-handler"))
  (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx539_signal_error_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (signal 'error '(test-data "message"))
  (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx539_signal_custom_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (signal 'my-custom-error '(123 "data"))
  (my-custom-error "custom-caught")
  (error "generic"))
"##,
    );
}

#[test]
fn div_cx539_ignore_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(ignore-errors (+ 1 2))
"##,
    );
}

#[test]
fn div_cx539_ignore_error_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(ignore-errors (car 1 2))
"##,
    );
}

#[test]
fn div_cx539_with_demoted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-demoted-errors "ERR: %S" (+ 1 2))
"##,
    );
}

#[test]
fn div_cx539_with_demoted_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-demoted-errors "ERR: %S" (car 1 2))
"##,
    );
}

#[test]
fn div_cx539_error_message_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (car 1 2)
  (error (error-message-string e)))
"##,
    );
}

#[test]
fn div_cx539_user_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (user-error "user `error' with %s" 'arg)
  (error (cadr e)))
"##,
    );
}

#[test]
fn div_cx539_warn() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (warn "warning test %s" 'arg)
  (error (cadr e)))
"##,
    );
}
