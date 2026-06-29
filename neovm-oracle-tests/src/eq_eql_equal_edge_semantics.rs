//! Oracle parity tests for `eq`, `eql`, `equal` — deep edge cases.
//!
//! GNU Emacs src/fns.c: `eq` compares identity, `eql` adds numeric type
//! equivalence, `equal` adds structural equivalence.  Many edge cases
//! around floats, integers, strings (unibyte/multibyte), hash tables,
//! and circular structures can diverge.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_eq_identical_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(eq 'foo 'foo)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_eq_distinct_floats_are_not_eq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(eq 1.0 1.0)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_eql_same_integers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(eql 42 42)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_eql_float_and_integer_are_not_eql() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(eql 1 1.0)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_equal_same_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(equal "hello" "hello")"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_equal_propertized_strings_equal_by_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: `equal' compares string content only, ignoring text properties.
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(equal
     (propertize "foo" 'face 'bold)
     (propertize "foo" 'face 'italic))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_equal_same_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(equal [1 2 3] [1 2 3])"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_equal_nested_conses() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(equal '(a (b c)) '(a (b c)))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_eql_distinct_cons_not_eql() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(eql '(a) '(a))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_eq_nil_and_false_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(eq nil '())",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_equal_hash_tables_not_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: `equal' does NOT descend into hash-table contents.
    // Two different hash tables are never equal, even with same contents.
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (let ((h1 (make-hash-table :test 'equal))
        (h2 (make-hash-table :test 'equal)))
    (puthash "key" 42 h1)
    (puthash "key" 42 h2)
    (equal h1 h2)))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_equal_different_hash_table_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (let ((h1 (make-hash-table))
        (h2 (make-hash-table)))
    (puthash 'a 1 h1)
    (puthash 'a 2 h2)
    (equal h1 h2)))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}
