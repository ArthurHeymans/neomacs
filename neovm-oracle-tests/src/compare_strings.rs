//! Oracle parity tests for `compare-strings`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_compare_strings_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // identical strings
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "foobar" nil nil "foobar" nil nil)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    // first < second
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "abc" nil nil "xyz" nil nil)"#,
        expect_test::expect![[r#""OK -1""#]],
    );
    assert_ok_eq("-1", &o, &n);

    // first > second
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "xyz" nil nil "abc" nil nil)"#,
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);

    // case-insensitive
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "HELLO" nil nil "hello" nil nil t)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    // subrange comparison
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "xxabcyy" 2 5 "zzabcww" 2 5)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);

    // prefix shorter
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "ab" nil nil "abcd" nil nil)"#,
        expect_test::expect![[r#""OK -3""#]],
    );
    assert_ok_eq("-3", &o, &n);

    // empty strings
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(compare-strings "" nil nil "" nil nil)"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}
