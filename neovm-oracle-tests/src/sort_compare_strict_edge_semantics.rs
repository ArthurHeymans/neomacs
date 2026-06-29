//! Oracle parity for sort + compare-strings strict edges.
//! GNU src/fns.c, src/sort.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_sort_nil_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(sort nil '<)"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_sort_vector_returns_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(vectorp (sort [3 1 2] '<))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_sort_length_preserved() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(= (length (sort '(3 1 4 2) '<)) 4)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_compare_strings_equal_returns_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "abc" nil nil "abc" nil nil)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_compare_strings_less_returns_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(< (compare-strings "abc" nil nil "abd" nil nil) 0)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_compare_strings_case_insensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(eq (compare-strings "ABC" nil nil "abc" nil nil) t)"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_sort_called_with_custom_pred() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(sort '(3 1 4 2) (lambda (a b) (< a b)))"#,
        expect_test::expect![[r#""OK (1 2 3 4)""#]],
    );
    assert_ok_eq("(1 2 3 4)", &o, &n);
}
