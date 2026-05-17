//! Oracle parity for vector, plist, and symbol deep edge cases.
//! GNU src/fns.c, src/lread.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- vconcat ---

#[test]
fn oracle_vconcat_two_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vconcat [1 2] [3 4])"#);
    assert_ok_eq("[1 2 3 4]", &o, &n);
}

#[test]
fn oracle_vconcat_vector_and_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vconcat [1] '(2 3))"#);
    assert_ok_eq("[1 2 3]", &o, &n);
}

#[test]
fn oracle_vconcat_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vconcat)"#);
    assert_ok_eq("[]", &o, &n);
}

// --- append with vector ---

#[test]
fn oracle_append_vector_to_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(append [1 2] '(3 4))"#);
    assert_ok_eq("(1 2 3 4)", &o, &n);
}

// --- aref / aset ---

#[test]
fn oracle_aref_returns_element() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(aref [10 20 30] 1)"#);
    assert_ok_eq("20", &o, &n);
}

#[test]
fn oracle_aset_returns_new_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(aset [10 20 30] 1 99)"#);
    assert_ok_eq("99", &o, &n);
}

// --- plist-get ---

#[test]
fn oracle_plist_get_found() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-get '(a 1 b 2) 'b)"#);
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_plist_get_missing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-get '(a 1 b 2) 'c)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- plist-put ---

#[test]
fn oracle_plist_put_destructive_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq p (list 'a 1 'b 2)) (plist-put p 'b 99) (plist-get p 'b))"#,
    );
    assert_ok_eq("99", &o, &n);
}

// --- make-symbol ---

#[test]
fn oracle_make_symbol_creates_uninterned() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(not (eq (make-symbol "foo") (make-symbol "foo")))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_make_symbol_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(symbol-name (make-symbol "my-sym"))"#);
    assert_ok_eq("\"my-sym\"", &o, &n);
}
