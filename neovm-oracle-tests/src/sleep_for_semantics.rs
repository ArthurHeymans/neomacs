//! Oracle parity tests for `sleep-for`.
//!
//! GNU implements `sleep-for` in `src/dispnew.c` — pauses for a given
//! number of seconds (and optional milliseconds).

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_sleep_for_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(sleep-for 0)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_sleep_for_with_milliseconds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(sleep-for 0 50)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_sleep_for_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(sleep-for 'a)",
        expect_test::expect![[r#""ERR (wrong-type-argument numberp a)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_sleep_for_wrong_number_of_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(sleep-for)",
        expect_test::expect![[r#""ERR (wrong-number-of-arguments sleep-for 0)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-number-of-arguments");
}
