//! Oracle parity for final uncovered subrs: buffer-swap-text,
//! copy-keymap, lsh, key-description vector.
//! GNU src/buffer.c, src/keymap.c, src/data.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm, eval_oracle_and_neovm_with_bootstrap};

#[test]
fn oracle_copy_keymap_is_keymap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(keymapp (copy-keymap (make-sparse-keymap)))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_copy_keymap_returns_copy_not_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (let ((orig (make-sparse-keymap)) (cpy (copy-keymap orig))) (not (eq orig cpy))))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_lsh_left() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_with_bootstrap("(lsh 1 3)");
    assert_ok_eq("8", &o, &n);
}

#[test]
fn oracle_lsh_right() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_with_bootstrap("(lsh 16 -2)");
    assert_ok_eq("4", &o, &n);
}

#[test]
fn oracle_key_description_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(stringp (key-description [?\C-a]))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_lsh_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm_with_bootstrap("(lsh 42 0)");
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_nreverse_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nreverse [1 2 3])"#);
    assert_ok_eq("[3 2 1]", &o, &n);
}
