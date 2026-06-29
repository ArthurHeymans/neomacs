//! Oracle parity tests for `upcase`, `downcase`, `capitalize`, and related.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

#[test]
fn oracle_prop_upcase_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(upcase "hello")"#,
        expect_test::expect![[r#""OK \"HELLO\"""#]],
    );
    assert_ok_eq(r#""HELLO""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(upcase "Hello World")"#,
        expect_test::expect![[r#""OK \"HELLO WORLD\"""#]],
    );
    assert_ok_eq(r#""HELLO WORLD""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(upcase "ALREADY")"#,
        expect_test::expect![[r#""OK \"ALREADY\"""#]],
    );
    assert_ok_eq(r#""ALREADY""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(upcase "")"#,
        expect_test::expect![[r#""OK \"\"""#]],
    );
    assert_ok_eq(r#""""#, &o, &n);
}

#[test]
fn oracle_prop_downcase_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(downcase "HELLO")"#,
        expect_test::expect![[r#""OK \"hello\"""#]],
    );
    assert_ok_eq(r#""hello""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(downcase "Hello World")"#,
        expect_test::expect![[r#""OK \"hello world\"""#]],
    );
    assert_ok_eq(r#""hello world""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(downcase "already")"#,
        expect_test::expect![[r#""OK \"already\"""#]],
    );
    assert_ok_eq(r#""already""#, &o, &n);
}

#[test]
fn oracle_prop_upcase_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(upcase ?a)",
        expect_test::expect![[r#""OK 65""#]],
    );
    assert_ok_eq("65", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(upcase ?A)",
        expect_test::expect![[r#""OK 65""#]],
    );
    assert_ok_eq("65", &o, &n);
}

#[test]
fn oracle_prop_downcase_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(downcase ?A)",
        expect_test::expect![[r#""OK 97""#]],
    );
    assert_ok_eq("97", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(downcase ?a)",
        expect_test::expect![[r#""OK 97""#]],
    );
    assert_ok_eq("97", &o, &n);
}

#[test]
fn oracle_prop_capitalize_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(capitalize "hello world")"#,
        expect_test::expect![[r#""OK \"Hello World\"""#]],
    );
    assert_ok_eq(r#""Hello World""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(capitalize "HELLO WORLD")"#,
        expect_test::expect![[r#""OK \"Hello World\"""#]],
    );
    assert_ok_eq(r#""Hello World""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(capitalize "hello")"#,
        expect_test::expect![[r#""OK \"Hello\"""#]],
    );
    assert_ok_eq(r#""Hello""#, &o, &n);
}

#[test]
fn oracle_prop_upcase_downcase_with_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(upcase "abc123def")"#,
        expect_test::expect![[r#""OK \"ABC123DEF\"""#]],
    );
    assert_ok_eq(r#""ABC123DEF""#, &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(downcase "ABC123DEF")"#,
        expect_test::expect![[r#""OK \"abc123def\"""#]],
    );
    assert_ok_eq(r#""abc123def""#, &o, &n);
}

#[test]
fn oracle_prop_upcase_downcase_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r####"(string-equal (downcase (upcase "hello")) "hello")"####;
    let (o, n) =
        crate::common::eval_oracle_and_neovm_expect(form, expect_test::expect![[r#""OK t""#]]);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_upcase_initials() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(upcase-initials "hello world")"#,
        expect_test::expect![[r#""OK \"Hello World\"""#]],
    );
    assert_ok_eq(r#""Hello World""#, &o, &n);

    // upcase-initials only capitalizes first letter of each word, preserves rest
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(upcase-initials "hELLO wORLD")"#,
        expect_test::expect![[r#""OK \"HELLO WORLD\"""#]],
    );
    assert_ok_eq(r#""HELLO WORLD""#, &o, &n);
}

#[test]
fn oracle_prop_mapcar_upcase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r####"(mapcar 'upcase '("foo" "bar" "baz"))"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""OK (\"FOO\" \"BAR\" \"BAZ\")""#]],
    );
}
