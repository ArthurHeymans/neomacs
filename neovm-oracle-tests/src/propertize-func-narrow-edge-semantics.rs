//! Oracle parity for propertize, functionp, narrow/widen, indirect-function edge cases.
//! GNU src/fns.c, src/data.c, src/editfns.c, src/eval.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- propertize ---

#[test]
fn oracle_propertize_returns_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(stringp (propertize "hello" 'face 'bold))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_propertize_preserves_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(equal (propertize "hello" 'a 1 'b 2) "hello")"#);
    // equal by default ignores text properties
    assert_ok_eq("t", &o, &n);
}

// --- functionp ---

#[test]
fn oracle_functionp_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(functionp 'car)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_functionp_non_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(functionp 42)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- narrow-to-region / widen ---

#[test]
fn oracle_narrow_to_region_changes_point_min_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (set-buffer (get-buffer-create "*nrw*")) (erase-buffer) (insert "abcdef") (narrow-to-region 2 5) (list (point-min) (point-max)))"#,
    );
    assert_ok_eq("(2 5)", &o, &n);
}

#[test]
fn oracle_widen_restores_full_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (set-buffer (get-buffer-create "*nrw2*")) (erase-buffer) (insert "abcdef") (narrow-to-region 2 5) (widen) (point-max))"#,
    );
    assert_ok_eq("7", &o, &n);
}

// --- indirect-function ---

#[test]
fn oracle_indirect_function_resolves_alias() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (defalias 'my-indirect-fn (symbol-function '1+)) (functionp (indirect-function 'my-indirect-fn)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

// --- macrop ---

#[test]
fn oracle_macrop_subr_is_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(macrop 'car)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- define-key ---

#[test]
fn oracle_define_key_returns_command() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq km (make-sparse-keymap)) (define-key km "a" 'forward-char) (lookup-key km "a"))"#,
    );
    assert_ok_eq("forward-char", &o, &n);
}

// --- lookup-key ---

#[test]
fn oracle_lookup_key_missing_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (setq km (make-sparse-keymap)) (lookup-key km "x"))"#);
    assert_ok_eq("nil", &o, &n);
}
