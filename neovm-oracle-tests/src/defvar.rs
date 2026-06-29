//! Oracle parity tests for `defvar`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

#[test]
fn oracle_prop_defvar_with_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // defvar with initial value
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(progn (defvar test--dv-x 42) test--dv-x)",
        expect_test::expect![[r#""OK 42""#]],
    );
    assert_ok_eq("42", &o, &n);

    // defvar does not overwrite existing value
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(progn (defvar test--dv-y 1) (defvar test--dv-y 2) test--dv-y)",
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);
}

#[test]
fn oracle_prop_defvar_without_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // defvar without init value should NOT bind the variable in batch mode
    crate::common::assert_oracle_parity_expect(
        "(progn (defvar test--dv-z) (boundp 'test--dv-z))",
        expect_test::expect![r#""OK nil""#],
    );
}

#[test]
fn oracle_prop_defvar_dynamic_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // dynamic scoping with let
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(progn (defvar test--dv-dyn 10) (let ((test--dv-dyn 77)) test--dv-dyn))",
        expect_test::expect![[r#""OK 77""#]],
    );
    assert_ok_eq("77", &o, &n);

    // dynamic binding restores after let
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(progn (defvar test--dv-restore 5) (let ((test--dv-restore 99)) nil) test--dv-restore)",
        expect_test::expect![[r#""OK 5""#]],
    );
    assert_ok_eq("5", &o, &n);
}
