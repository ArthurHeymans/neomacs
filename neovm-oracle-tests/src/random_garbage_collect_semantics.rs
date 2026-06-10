//! Oracle parity tests for `random` and `garbage-collect`.
//!
//! GNU implements `random` in `src/fns.c` and `garbage-collect` in `src/alloc.c`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_random_returns_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(integerp (random 100))");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_random_with_limit_returns_value_in_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(< (random 10) 10)");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_random_with_t_uses_most_positive_fixnum() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(integerp (random t))");
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_garbage_collect_returns_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(listp (garbage-collect))");
    assert_ok_eq("t", &oracle, &neovm);
}
