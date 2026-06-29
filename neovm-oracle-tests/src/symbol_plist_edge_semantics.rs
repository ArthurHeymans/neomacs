//! Oracle parity tests for symbol property list edge cases.
//!
//! GNU src/data.c: `get`, `put`, `symbol-plist`, `setplist` — property
//! lists are a fundamental Emacs mechanism. Edges around nil, overwrite
//! semantics, and symbol identity are subtle.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_put_then_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (put 'neovm--test-prop-sym 'color 'blue)
  (get 'neovm--test-prop-sym 'color))"#,
        expect_test::expect![[r#""OK blue""#]],
    );
    assert_ok_eq("blue", &oracle, &neovm);
}

#[test]
fn oracle_get_nonexistent_property_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(get 'neovm--test-no-prop 'nonexistent-key)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_put_overwrites_previous_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (put 'neovm--test-overwrite 'key 1)
  (put 'neovm--test-overwrite 'key 2)
  (get 'neovm--test-overwrite 'key))"#,
        expect_test::expect![[r#""OK 2""#]],
    );
    assert_ok_eq("2", &oracle, &neovm);
}

#[test]
fn oracle_symbol_plist_returns_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (setplist 'neovm--test-plist '(a 1 b 2))
  (list (get 'neovm--test-plist 'a)
        (get 'neovm--test-plist 'b)))"#,
        expect_test::expect![[r#""OK (1 2)""#]],
    );
    assert_ok_eq("(1 2)", &oracle, &neovm);
}

#[test]
fn oracle_setplist_replaces_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (put 'neovm--test-replace 'old-key 99)
  (setplist 'neovm--test-replace '(new-key 42))
  (list (get 'neovm--test-replace 'old-key)
        (get 'neovm--test-replace 'new-key)))"#,
        expect_test::expect![[r#""OK (nil 42)""#]],
    );
    assert_ok_eq("(nil 42)", &oracle, &neovm);
}

#[test]
fn oracle_put_nil_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (put 'neovm--test-nil-val 'x nil)
  (get 'neovm--test-nil-val 'x))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_get_with_wrong_type_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(get 42 'key)",
        expect_test::expect![[r#""ERR (wrong-type-argument symbolp 42)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}
