//! Oracle parity tests for `intern`, `intern-soft`, `concat`, `vconcat`,
//! and `reverse` — strict edge cases.
//!
//! GNU src/lread.c (intern), src/fns.c (concat, vconcat, reverse).

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// intern / intern-soft
// ---------------------------------------------------------------------------

#[test]
fn oracle_intern_identity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(eq (intern "test-abc") (intern "test-abc"))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_intern_soft_nil_for_unknown() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(intern-soft "xyznonexistent999")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_intern_soft_finds_existing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (intern "exists-here") (symbolp (intern-soft "exists-here")))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

// ---------------------------------------------------------------------------
// concat
// ---------------------------------------------------------------------------

#[test]
fn oracle_concat_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(concat "a" "b" "c")"#,
        expect_test::expect![[r#""OK \"abc\"""#]],
    );
    assert_ok_eq("\"abc\"", &o, &n);
}

#[test]
fn oracle_concat_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(concat)"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
    assert_ok_eq("\"\"", &o, &n);
}

#[test]
fn oracle_concat_single_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(concat "solo")"#,
        expect_test::expect![[r#""OK \"solo\"""#]],
    );
    assert_ok_eq("\"solo\"", &o, &n);
}

#[test]
fn oracle_concat_mixed_lists_and_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(concat '(97 98) "cd")"#,
        expect_test::expect![[r#""OK \"abcd\"""#]],
    );
    assert_ok_eq("\"abcd\"", &o, &n);
}

// ---------------------------------------------------------------------------
// vconcat
// ---------------------------------------------------------------------------

#[test]
fn oracle_vconcat_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(vconcat [1 2] [3 4])"#,
        expect_test::expect![[r#""OK [1 2 3 4]""#]],
    );
    assert_ok_eq("[1 2 3 4]", &o, &n);
}

#[test]
fn oracle_vconcat_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(vconcat)"#,
        expect_test::expect![[r#""OK []""#]],
    );
    assert_ok_eq("[]", &o, &n);
}

// ---------------------------------------------------------------------------
// reverse
// ---------------------------------------------------------------------------

#[test]
fn oracle_reverse_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(reverse '(a b c))"#,
        expect_test::expect![[r#""OK (c b a)""#]],
    );
    assert_ok_eq("(c b a)", &o, &n);
}

#[test]
fn oracle_reverse_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(reverse "abc")"#,
        expect_test::expect![[r#""OK \"cba\"""#]],
    );
    assert_ok_eq("\"cba\"", &o, &n);
}

#[test]
fn oracle_reverse_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(reverse nil)"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}
