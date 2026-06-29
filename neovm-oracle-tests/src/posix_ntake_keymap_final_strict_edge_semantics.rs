//! Oracle parity for posix regex + ntake + keymap accessors.
//! GNU src/search.c, src/fns.c, src/keyboard.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

#[test]
fn oracle_posix_looking_at_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (switch-to-buffer (get-buffer-create "*pl*")) (erase-buffer) (insert "hello123") (goto-char 1) (posix-looking-at "[a-z]+"))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_posix_string_match_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(posix-string-match "foo" "foobar")"#,
        expect_test::expect![[r#""OK 0""#]],
    );
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_posix_string_match_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(posix-string-match "xyz" "foobar")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_ntake_first_n() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(ntake 2 '(a b c d e))"#,
        expect_test::expect![[r#""OK (a b)""#]],
    );
    assert_ok_eq("(a b)", &o, &n);
}

#[test]
fn oracle_ntake_more_than_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(ntake 10 '(a b))"#,
        expect_test::expect![[r#""OK (a b)""#]],
    );
    assert_ok_eq("(a b)", &o, &n);
}

#[test]
fn oracle_current_global_map_is_keymap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(keymapp (current-global-map))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_current_local_map_default_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r#"(current-local-map)"#,
        expect_test::expect![[r#""OK nil""#]],
    );
}
