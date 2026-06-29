//! Oracle parity tests for data construction: `make-string`,
//! `make-list`, `make-vector`, `make-symbol`, `string`, `vector`.
//!
//! GNU src/alloc.c, src/fns.c, src/lread.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_make_string_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(make-string 3 ?a)"#,
        expect_test::expect![[r#""OK \"aaa\"""#]],
    );
    assert_ok_eq("\"aaa\"", &o, &n);
}

#[test]
fn oracle_make_string_zero_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(make-string 0 ?x)"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
    assert_ok_eq("\"\"", &o, &n);
}

#[test]
fn oracle_make_list_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(make-list 3 'x)"#,
        expect_test::expect![[r#""OK (x x x)""#]],
    );
    assert_ok_eq("(x x x)", &o, &n);
}

#[test]
fn oracle_make_list_zero_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(make-list 0 'x)"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_make_vector_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(make-vector 3 'x)"#,
        expect_test::expect![[r#""OK [x x x]""#]],
    );
    assert_ok_eq("[x x x]", &o, &n);
}

#[test]
fn oracle_make_vector_zero_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(make-vector 0 'x)"#,
        expect_test::expect![[r#""OK []""#]],
    );
    assert_ok_eq("[]", &o, &n);
}

#[test]
fn oracle_make_symbol_creates_uninterned() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(symbolp (make-symbol "test-ms"))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_string_constructor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string 97 98 99)"#,
        expect_test::expect![[r#""OK \"abc\"""#]],
    );
    assert_ok_eq("\"abc\"", &o, &n);
}

#[test]
fn oracle_vector_constructor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(vector 1 2 3)"#,
        expect_test::expect![[r#""OK [1 2 3]""#]],
    );
    assert_ok_eq("[1 2 3]", &o, &n);
}

#[test]
fn oracle_make_string_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(make-string 'a ?x)"#,
        expect_test::expect![[r#""ERR (wrong-type-argument wholenump a)""#]],
    );
    assert_err_kind(&o, &n, "wrong-type-argument");
}
