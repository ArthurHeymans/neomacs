/// Batch 539: condition-case in all forms, handler-bind, signal error conditions.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx539_condition_case_simple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e (+ 1 2) (error (car e)))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx539_condition_case_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e (error "fail") (error (cadr e)))
"##,
        expect_test::expect![[r#""OK \"fail\"""#]],
    );
}

#[test]
fn div_cx539_condition_case_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (car 1 2)
  (wrong-number-of-arguments "wrong-num")
  (error "other"))
"##,
        expect_test::expect![[r#""OK \"wrong-num\"""#]],
    );
}

#[test]
fn div_cx539_condition_case_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e1
    (condition-case e2
        (error "nested")
      (error (format "inner: %s" (cadr e2))))
  (error (format "outer: %s" (cadr e1))))
"##,
        expect_test::expect![[r#""OK \"inner: nested\"""#]],
    );
}

#[test]
fn div_cx539_condition_case_unless() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case-unless-debug e
    (+ "a" 1)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK wrong-type-argument""#]],
    );
}

#[test]
fn div_cx539_handler_bind() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (handler-bind ((error (lambda (e) (message "caught: %S" e))))
      (error "test-handler"))
  (error (cadr e)))
"##,
        expect_test::expect![[r#""OK \"test-handler\"""#]],
    );
}

#[test]
fn div_cx539_signal_error_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (signal 'error '(test-data "message"))
  (error (cadr e)))
"##,
        expect_test::expect![[r#""OK test-data""#]],
    );
}

#[test]
fn div_cx539_signal_custom_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (signal 'my-custom-error '(123 "data"))
  (my-custom-error "custom-caught")
  (error "generic"))
"##,
        expect_test::expect![[r#""OK \"generic\"""#]],
    );
}

#[test]
fn div_cx539_ignore_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(ignore-errors (+ 1 2))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx539_ignore_error_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(ignore-errors (car 1 2))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx539_with_demoted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-demoted-errors "ERR: %S" (+ 1 2))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx539_with_demoted_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-demoted-errors "ERR: %S" (car 1 2))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx539_error_message_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (car 1 2)
  (error (error-message-string e)))
"##,
        expect_test::expect![[r#""OK \"Wrong number of arguments: car, 2\"""#]],
    );
}

#[test]
fn div_cx539_user_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (user-error "user `error' with %s" 'arg)
  (error (cadr e)))
"##,
        expect_test::expect![[r#""OK \"user ‘error’ with arg\"""#]],
    );
}

#[test]
fn div_cx539_warn() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (warn "warning test %s" 'arg)
  (error (cadr e)))
"##,
        expect_test::expect![[r#""OK \"Warning (emacs): warning test arg\"""#]],
    );
}
