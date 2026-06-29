//! Oracle parity tests for `string-version-lessp`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_string_version_lessp_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // identical
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "v1.0" "v1.0")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    // numeric ordering within strings
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "file2" "file10")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "file10" "file2")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    // pure numeric
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "9" "10")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    // version-style
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "1.9.3" "1.10.1")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "1.10.1" "1.9.3")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    // prefix relation
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "pkg" "pkg1")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "pkg1" "pkg")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    // empty strings
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "" "")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "" "a")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    // leading zeros
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "007" "7")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);

    // mixed alpha-numeric with dots
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-version-lessp "v2.0" "v10.0")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_prop_string_version_lessp_symbol_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU Emacs accepts symbols — neovm should too
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        "(string-version-lessp 'v2 'v10)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}
