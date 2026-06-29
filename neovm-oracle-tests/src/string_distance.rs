//! Oracle parity tests for `string-distance`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_prop_string_distance_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // identical
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "kitten" "kitten")"#,
        expect_test::expect![[r#""OK 0""#]],
    );
    assert_ok_eq("0", &o, &n);

    // single substitution
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "cat" "bat")"#,
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);

    // classic levenshtein example
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "kitten" "sitting")"#,
        expect_test::expect![[r#""OK 3""#]],
    );
    assert_ok_eq("3", &o, &n);

    // insertion
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "abc" "abcd")"#,
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);

    // deletion
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "abcd" "abc")"#,
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);

    // empty vs non-empty
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "" "test")"#,
        expect_test::expect![[r#""OK 4""#]],
    );
    assert_ok_eq("4", &o, &n);

    // both empty
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "" "")"#,
        expect_test::expect![[r#""OK 0""#]],
    );
    assert_ok_eq("0", &o, &n);

    // completely different
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "abc" "xyz")"#,
        expect_test::expect![[r#""OK 3""#]],
    );
    assert_ok_eq("3", &o, &n);

    // byte length mode
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(string-distance "abc" "axc" t)"#,
        expect_test::expect![[r#""OK 1""#]],
    );
    assert_ok_eq("1", &o, &n);
}
