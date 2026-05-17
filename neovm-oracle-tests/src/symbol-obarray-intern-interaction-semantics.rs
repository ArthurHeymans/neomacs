//! Oracle parity for symbol, obarray, intern interaction edge cases.
//! GNU src/lread.c, src/data.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- intern / intern-soft ---

#[test]
fn oracle_intern_creates_and_returns_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(symbol-name (intern "nvm-test-sym-123"))"#);
    assert_ok_eq("\"nvm-test-sym-123\"", &o, &n);
}

#[test]
fn oracle_intern_soft_existing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(intern-soft "car")"#);
    assert_ok_eq("car", &o, &n);
}

#[test]
fn oracle_intern_soft_nonexistent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(intern-soft "nonexistent-sym-xyz-999")"#);
    assert_ok_eq("nil", &o, &n);
}

// --- makunbound ---

#[test]
fn oracle_makunbound_removes_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq mb-test 42) (makunbound 'mb-test) (boundp 'mb-test))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

// --- setplist / get / symbol-plist ---

#[test]
fn oracle_setplist_and_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq psym (intern "plt-test")) (setplist psym '(a 1 b 2)) (list (get psym 'a) (get psym 'b)))"#,
    );
    assert_ok_eq("(1 2)", &o, &n);
}

#[test]
fn oracle_symbol_plist_returns_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq psym2 (intern "plt-test2")) (setplist psym2 '(a 1 b 2)) (symbol-plist psym2))"#,
    );
    assert_ok_eq("(a 1 b 2)", &o, &n);
}

// --- set / symbol-value ---

#[test]
fn oracle_set_and_symbol_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq sv-sym (intern "sv-test")) (set sv-sym 99) (symbol-value sv-sym))"#,
    );
    assert_ok_eq("99", &o, &n);
}

// --- fmakunbound ---

#[test]
fn oracle_fmakunbound_stops_fboundp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Can't fmakunbound a built-in, but can test fset + fmakunbound
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq fsym (intern "fmak-test")) (fset fsym (symbol-function 'car)) (fmakunbound fsym) (fboundp fsym))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

// --- put / get property ---

#[test]
fn oracle_put_and_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq qsym (intern "prop-test")) (put qsym 'my-prop 42) (get qsym 'my-prop))"#,
    );
    assert_ok_eq("42", &o, &n);
}
