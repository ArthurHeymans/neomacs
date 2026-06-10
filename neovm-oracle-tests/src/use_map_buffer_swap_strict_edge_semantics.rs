//! Oracle parity for use-global-map, use-local-map, buffer-swap-text.
//! GNU src/keymap.c, src/buffer.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- use-global-map ---

#[test]
fn oracle_use_global_map_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(use-global-map (current-global-map))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_use_global_map_keeps_global_map() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (use-global-map (current-global-map)) (keymapp (current-global-map)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_use_global_map_nil_signals_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(condition-case err (progn (use-global-map nil) nil) (error (symbol-name (car err))))"#,
    );
    assert_ok_eq("\"wrong-type-argument\"", &o, &n);
}

// --- use-local-map ---

#[test]
fn oracle_use_local_map_nil_allowed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(use-local-map nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_use_local_map_sets_and_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(progn (use-local-map (make-sparse-keymap)) nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_use_local_map_sets_local_map() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (use-local-map (make-sparse-keymap)) (keymapp (current-local-map)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_use_local_map_non_keymap_signals_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(condition-case err (progn (use-local-map 42) nil) (error (symbol-name (car err))))"#,
    );
    assert_ok_eq("\"wrong-type-argument\"", &o, &n);
}

// --- buffer-swap-text ---

#[test]
fn oracle_buffer_swap_text_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Create two buffers, swap text between them
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq b1 (get-buffer-create "*swap-src*")) (set-buffer b1) (erase-buffer) (insert "hello") (setq b2 (get-buffer-create "*swap-dst*")) (set-buffer b2) (erase-buffer) (insert "world") (buffer-swap-text b1))"#,
    );
    // Return value is always nil
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_buffer_swap_text_swaps_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // After swap, the other buffer's text is now in b2
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (setq b1 (get-buffer-create "*swap-src2*")) (set-buffer b1) (erase-buffer) (insert "hello") (setq b2 (get-buffer-create "*swap-dst2*")) (set-buffer b2) (erase-buffer) (insert "world") (buffer-swap-text b1) (buffer-string))"#,
    );
    // b2 now has b1's old content "hello"
    assert_ok_eq("\"hello\"", &o, &n);
}
