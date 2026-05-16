//! Oracle parity tests for sequence operations.
//!
//! GNU src/fns.c: `length`, `elt`, `aref`, `aset` operate on sequences
//! (strings, vectors, lists, bool-vectors). Bounds checking and type
//! errors are key divergence areas.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_length_of_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(length '(a b c))");
    assert_ok_eq("3", &oracle, &neovm);
}

#[test]
fn oracle_length_of_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(length "hello")"#);
    assert_ok_eq("5", &oracle, &neovm);
}

#[test]
fn oracle_length_of_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(length [1 2 3 4])"#);
    assert_ok_eq("4", &oracle, &neovm);
}

#[test]
fn oracle_length_of_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(length nil)");
    assert_ok_eq("0", &oracle, &neovm);
}

#[test]
fn oracle_elt_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(elt '(a b c) 1)");
    assert_ok_eq("b", &oracle, &neovm);
}

#[test]
fn oracle_elt_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(elt [10 20 30] 2)");
    assert_ok_eq("30", &oracle, &neovm);
}

#[test]
fn oracle_elt_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(elt "abc" 0)"#);
    assert_ok_eq("97", &oracle, &neovm);
}

#[test]
fn oracle_elt_out_of_bounds_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: elt returns nil for out-of-bounds, not an error.
    let (oracle, neovm) = eval_oracle_and_neovm("(elt '(a b) 5)");
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_aref_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(aref [1 2 3] 1)");
    assert_ok_eq("2", &oracle, &neovm);
}

#[test]
fn oracle_aref_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(aref "abc" 2)"#);
    assert_ok_eq("99", &oracle, &neovm);
}

#[test]
fn oracle_aset_vector_mutation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (let ((v [1 2 3]))
    (aset v 0 99)
    v))"#,
    );
    assert_ok_eq("[99 2 3]", &oracle, &neovm);
}

#[test]
fn oracle_length_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm("(length 42)");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}
