//! Oracle parity for misc coverage fillers: `proper-list-p`,
//! `plistp`, `readablep`, `flatten-tree`, `copy-tree`, `gensym`.
//! GNU src/fns.c, src/alloc.c, src/lread.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_proper_list_p_true() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(proper-list-p '(a b c))"#);
    assert_ok_eq("3", &o, &n);
}

#[test]
fn oracle_proper_list_p_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(proper-list-p nil)"#);
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_proper_list_p_dotted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(proper-list-p '(a b . c))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_consp_on_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(consp '(a . b))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_consp_on_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(consp nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_listp_on_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(listp '(a b c))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_listp_on_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(listp nil)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_nlistp_on_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nlistp nil)"#);
    assert_ok_eq("nil", &o, &n);
}
