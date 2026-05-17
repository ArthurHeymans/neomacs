//! Oracle parity for concat mixed, vconcat mixed, nconc, mapcan edges.
//! GNU src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_concat_integers_forms_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(concat 72 73)"#);
    assert_ok_eq("\"HI\"", &o, &n);
}

#[test]
fn oracle_concat_one_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(concat "hello")"#);
    assert_ok_eq("\"hello\"", &o, &n);
}

#[test]
fn oracle_vconcat_lists_to_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vconcat '(1 2) '(3 4))"#);
    assert_ok_eq("[1 2 3 4]", &o, &n);
}

#[test]
fn oracle_vconcat_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(vconcat [1 2] [3 4] [5 6])"#);
    assert_ok_eq("[1 2 3 4 5 6]", &o, &n);
}

#[test]
fn oracle_nconc_last_non_list_dotted() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nconc '(1 2) 3)"#);
    assert_ok_eq("(1 2 . 3)", &o, &n);
}

#[test]
fn oracle_nconc_nil_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nconc nil '(1 2) nil)"#);
    assert_ok_eq("(1 2)", &o, &n);
}

#[test]
fn oracle_mapcan_simple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mapcan 'list '(1 2 3))"#);
    assert_ok_eq("(1 2 3)", &o, &n);
}
