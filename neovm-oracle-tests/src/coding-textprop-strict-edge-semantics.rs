//! Oracle parity tests for coding-system + text-property operations.
//!
//! GNU src/coding.c, src/textprop.c: `check-coding-system`,
//! `coding-system-p`, `get-byte`, `text-properties-at`, `propertize`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_coding_system_p_on_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(coding-system-p 'utf-8)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_coding_system_p_on_unknown() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(coding-system-p 'no-such-coding-system-xyz)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_check_coding_system_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(check-coding-system 'utf-8)"#);
    assert_ok_eq("utf-8", &o, &n);
}

#[test]
fn oracle_get_byte_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*gb*")) (erase-buffer) (insert "hello") (integerp (get-byte 3)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_text_properties_at_none() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*tpa*")) (erase-buffer) (insert "hello") (text-properties-at 2))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_text_properties_at_with_prop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (switch-to-buffer (get-buffer-create "*tpap*")) (erase-buffer) (insert "hello") (put-text-property 1 3 'face 'bold) (eq 'bold (get-text-property 2 'face)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_propertize_creates_string_with_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (let ((s (propertize "hello" 'face 'bold))) (eq 'bold (get-text-property 0 'face s))))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_check_coding_system_wrong_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(check-coding-system 42)"#);
    assert_err_kind(&o, &n, "wrong-type-argument");
}
