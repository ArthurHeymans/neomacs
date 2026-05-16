//! Oracle parity for keymap operations: `define-key`, `lookup-key`,
//! `keymapp`, `make-sparse-keymap`, `make-keymap`, `keymap-parent`.
//!
//! GNU src/keymap.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_keymapp_on_keymap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keymapp (make-sparse-keymap))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_keymapp_on_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keymapp nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_make_sparse_keymap_creates_keymap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keymapp (make-sparse-keymap))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_make_keymap_creates_keymap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keymapp (make-keymap))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_lookup_key_undefined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(lookup-key (make-sparse-keymap) "a")"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_define_key_and_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (let ((km (make-sparse-keymap))) (define-key km "a" 'forward-char) (commandp (lookup-key km "a"))))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_keymap_parent_default_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keymap-parent (make-sparse-keymap))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_set_keymap_parent_returns_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (let ((child (make-sparse-keymap)) (parent (make-sparse-keymap))) (set-keymap-parent child parent) (eq parent (keymap-parent child))))"#,
    );
    assert_ok_eq("t", &o, &n);
}
