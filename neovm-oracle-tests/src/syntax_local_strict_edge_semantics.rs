//! Oracle parity for char-syntax + buffer-local operations.
//! GNU src/syntax.c, src/data.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_char_syntax_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(char-syntax ?a)"#,
        expect_test::expect![[r#""OK 119""#]],
    );
    assert_ok_eq("119", &o, &n);
}

#[test]
fn oracle_char_syntax_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(char-syntax ?\s)"#,
        expect_test::expect![[r#""OK 32""#]],
    );
    assert_ok_eq("32", &o, &n);
}

#[test]
fn oracle_syntax_table_returns_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(syntax-table-p (syntax-table))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_make_local_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (set (make-local-variable 'neovm--test-mlv) 42) neovm--test-mlv)"#,
        expect_test::expect![[r#""OK 42""#]],
    );
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_kill_local_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (set (make-local-variable 'neovm--test-klv) 77) (kill-local-variable 'neovm--test-klv) (not (boundp 'neovm--test-klv)))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_set_syntax_table_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (let ((orig (syntax-table))) (unwind-protect (syntax-table-p (set-syntax-table orig)) (set-syntax-table orig))))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_syntax_class_to_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(characterp (syntax-class-to-char 2))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_modify_syntax_entry_alters_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (modify-syntax-entry ?z "w") (char-syntax ?z))"#,
        expect_test::expect![[r#""OK 119""#]],
    );
    assert_ok_eq("119", &o, &n);
}
