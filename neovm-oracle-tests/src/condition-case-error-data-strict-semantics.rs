//! Oracle parity tests for `condition-case` error data structure.
//!
//! DIVERGENCE: condition-case `(car err)` returns `void-function` in
//! Neovm instead of the signaled error symbol.  See test below.
//!
//! GNU src/eval.c: error data is a cons `(ERROR-SYMBOL . DATA)`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_condition_case_error_symbol_is_car_of_err() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: (condition-case err (error "test") (error (car err))) → error
    // Neovm: returns void-function instead of error (DIVERGENCE)
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(condition-case err
         (error "test")
       (error (car err)))"#,
    );
    assert_ok_eq("error", &oracle, &neovm);
}

#[test]
fn oracle_condition_case_error_message_is_cadr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(condition-case err
         (error "my message")
       (error (cadr err)))"#,
    );
    assert_ok_eq("\"my message\"", &oracle, &neovm);
}

#[test]
fn oracle_condition_case_error_data_is_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(condition-case err
         (error "msg")
       (error (listp err)))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}
