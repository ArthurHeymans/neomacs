//! Oracle parity tests for condition-case error handling — strict edges.
//!
//! Uses only built-in subrs (no bootstrap macros).  Tests error
//! re-signaling, handler matching, and body pass-through.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_condition_case_no_error_returns_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(condition-case nil 42 (error 'never))"#,
        expect_test::expect![[r#""OK 42""#]],
    );
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_condition_case_arith_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(condition-case err (/ 1 0) (arith-error 'caught-div))"#,
        expect_test::expect![[r#""OK caught-div""#]],
    );
    assert_ok_eq("caught-div", &o, &n);
}

#[test]
fn oracle_condition_case_handler_not_found_propagates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(condition-case nil
         (condition-case nil (/ 1 0) (void-variable 'wrong-handler))
       (arith-error 'outer-caught))"#,
        expect_test::expect![[r#""OK outer-caught""#]],
    );
    assert_ok_eq("outer-caught", &o, &n);
}

#[test]
fn oracle_condition_case_multiple_handlers_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(condition-case err (/ 1 0) (arith-error 'arithmetic) (error 'generic))"#,
        expect_test::expect![[r#""OK arithmetic""#]],
    );
    assert_ok_eq("arithmetic", &o, &n);
}

#[test]
fn oracle_condition_case_error_data_is_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(condition-case err (/ 1 0) (arith-error (consp err)))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_condition_case_nested_catches_at_right_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(condition-case nil
         (condition-case nil (/ 1 0) (arith-error 'inner))
       (arith-error 'outer))"#,
        expect_test::expect![[r#""OK inner""#]],
    );
    assert_ok_eq("inner", &o, &n);
}
