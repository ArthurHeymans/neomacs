//! Oracle parity for list/sequence deep interaction edge cases.
//! GNU src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- nconc ---

#[test]
fn oracle_nconc_nil_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nconc nil '(a b))"#);
    assert_ok_eq("(a b)", &o, &n);
}

#[test]
fn oracle_nconc_nil_second() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nconc '(a) nil)"#);
    assert_ok_eq("(a)", &o, &n);
}

#[test]
fn oracle_nconc_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nconc)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_nconc_destructive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq a '(1 2)) (setq b '(3 4)) (setq c (nconc a b)) (list a b (eq a c)))"#,
    );
    // a is modified to (1 2 3 4); c is eq to a
    assert_ok_eq("((1 2 3 4) (3 4) t)", &o, &n);
}

// --- ntake ---

#[test]
fn oracle_ntake_less_than_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ntake 3 '(a b c d e))"#);
    assert_ok_eq("(a b c)", &o, &n);
}

#[test]
fn oracle_ntake_more_than_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ntake 10 '(a b))"#);
    assert_ok_eq("(a b)", &o, &n);
}

#[test]
fn oracle_ntake_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(ntake 0 '(a b))"#);
    assert_ok_eq("nil", &o, &n);
}

// --- reverse ---

#[test]
fn oracle_reverse_preserves_original() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq orig '(1 2 3)) (setq rev (reverse orig)) (list rev orig))"#,
    );
    assert_ok_eq("((3 2 1) (1 2 3))", &o, &n);
}

// --- nreverse ---

#[test]
fn oracle_nreverse_destructive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (setq lst (list 1 2 3)) (setq r (nreverse lst)) r)"#);
    assert_ok_eq("(3 2 1)", &o, &n);
}

// --- delq / delete interaction ---

#[test]
fn oracle_delq_removes_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn (setq lst (list 1 2 3 2 4)) (delq 2 lst) lst)"#);
    assert_ok_eq("(1 3 4)", &o, &n);
}
