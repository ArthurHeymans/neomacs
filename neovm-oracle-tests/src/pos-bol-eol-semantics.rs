//! Oracle parity tests for `pos-bol` and `pos-eol`.
//!
//! GNU implements both in `src/editfns.c` — return position of beginning/end
//! of line at a given position.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_pos_bol_at_buffer_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-pos-bol*"))
  (erase-buffer)
  (pos-bol))"#,
    );
    assert_ok_eq("1", &oracle, &neovm);
}

#[test]
fn oracle_pos_eol_at_empty_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-pos-eol*"))
  (erase-buffer)
  (pos-eol))"#,
    );
    assert_ok_eq("1", &oracle, &neovm);
}

#[test]
fn oracle_pos_bol_wrong_type_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(pos-bol 'a)");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}
