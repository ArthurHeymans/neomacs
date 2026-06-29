//! Oracle parity for type-system: type-of, max-char, bool-vector-p.
//! GNU src/data.c, src/character.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_type_of_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(type-of 42)"#,
        expect_test::expect![[r#""OK integer""#]],
    );
    assert_ok_eq("integer", &o, &n);
}

#[test]
fn oracle_type_of_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(type-of "hello")"#,
        expect_test::expect![[r#""OK string""#]],
    );
    assert_ok_eq("string", &o, &n);
}

#[test]
fn oracle_type_of_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(type-of 'sym)"#,
        expect_test::expect![[r#""OK symbol""#]],
    );
    assert_ok_eq("symbol", &o, &n);
}

#[test]
fn oracle_type_of_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(type-of '(a . b))"#,
        expect_test::expect![[r#""OK cons""#]],
    );
    assert_ok_eq("cons", &o, &n);
}

#[test]
fn oracle_type_of_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(type-of [1 2 3])"#,
        expect_test::expect![[r#""OK vector""#]],
    );
    assert_ok_eq("vector", &o, &n);
}

#[test]
fn oracle_type_of_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(type-of 3.14)"#,
        expect_test::expect![[r#""OK float""#]],
    );
    assert_ok_eq("float", &o, &n);
}

#[test]
fn oracle_max_char_positive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(> (max-char) 0)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_bool_vector_p_on_bool_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(bool-vector-p (bool-vector t nil))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}
