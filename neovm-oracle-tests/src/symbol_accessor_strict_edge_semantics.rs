//! Oracle parity tests for symbol accessors — strict edge cases.
//!
//! GNU src/data.c: `symbol-name`, `symbol-function`, `symbol-value`,
//! `symbol-plist`, `boundp`, `fboundp`, `makunbound`, `fmakunbound`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_symbol_name_of_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(symbol-name nil)"#,
        expect_test::expect![[r#""OK \"nil\"""#]],
    );
    assert_ok_eq("\"nil\"", &o, &n);
}

#[test]
fn oracle_symbol_name_of_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(symbol-name t)"#,
        expect_test::expect![[r#""OK \"t\"""#]],
    );
    assert_ok_eq("\"t\"", &o, &n);
}

#[test]
fn oracle_symbol_name_returns_live_symbol_name_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
      (let* ((sym (make-symbol "abc"))
             (name1 (symbol-name sym))
             (name2 (symbol-name sym)))
        (aset name1 0 ?X)
        (list (eq name1 name2)
              name1
              name2
              (symbol-name sym)
              (eq name1 (symbol-name sym))))"#;
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        form,
        expect_test::expect![[r#""OK (t \"Xbc\" \"Xbc\" \"Xbc\" t)""#]],
    );
    assert_ok_eq(r#"(t "Xbc" "Xbc" "Xbc" t)"#, &o, &n);
}

#[test]
fn oracle_symbol_function_of_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(subrp (symbol-function 'car))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_symbol_function_of_void() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(symbol-function 'nonexistent-symbol-xyz999)"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_symbol_value_of_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(symbol-value t)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_boundp_on_bound() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (set 'neovm--test-bp 42) (boundp 'neovm--test-bp))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_boundp_on_void() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (makunbound 'neovm--test-bp2) (boundp 'neovm--test-bp2))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_signal_void_variable_for_unbound() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(condition-case err (symbol-value 'neovm--test-void-xy) (void-variable 'void))"#,
        expect_test::expect![[r#""OK void""#]],
    );
    assert_ok_eq("void", &o, &n);
}
