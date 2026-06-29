//! Oracle parity for number-sequence + concat character-sequence edges.
//! GNU lisp/subr.el and src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_number_sequence_ascending() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(number-sequence 1 5)"#,
        expect_test::expect![[r#""OK (1 2 3 4 5)""#]],
    );
    assert_ok_eq("(1 2 3 4 5)", &o, &n);
}

#[test]
fn oracle_number_sequence_with_step() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(number-sequence 0 10 2)"#,
        expect_test::expect![[r#""OK (0 2 4 6 8 10)""#]],
    );
    assert_ok_eq("(0 2 4 6 8 10)", &o, &n);
}

#[test]
fn oracle_number_sequence_descending() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(number-sequence 5 1 -1)"#,
        expect_test::expect![[r#""OK (5 4 3 2 1)""#]],
    );
    assert_ok_eq("(5 4 3 2 1)", &o, &n);
}

#[test]
fn oracle_number_sequence_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(number-sequence 3 3)"#,
        expect_test::expect![[r#""OK (3)""#]],
    );
    assert_ok_eq("(3)", &o, &n);
}

#[test]
fn oracle_concat_integer_args_signal_sequencep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(concat 65 66 67)"#,
        expect_test::expect![[r#""ERR (wrong-type-argument sequencep 65)""#]],
    );
    assert_err_kind(&o, &n, "wrong-type-argument");
}

#[test]
fn oracle_concat_integer_list_is_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(concat '(65 66 67))"#,
        expect_test::expect![[r#""OK \"ABC\"""#]],
    );
    assert_ok_eq("\"ABC\"", &o, &n);
}

#[test]
fn oracle_concat_none_returns_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(concat)"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
    assert_ok_eq("\"\"", &o, &n);
}

#[test]
fn oracle_vconcat_none_returns_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(vconcat)"#,
        expect_test::expect![[r#""OK []""#]],
    );
    assert_ok_eq("[]", &o, &n);
}
