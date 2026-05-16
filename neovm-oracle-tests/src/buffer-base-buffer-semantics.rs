//! Oracle parity tests for `buffer-base-buffer`.
//!
//! GNU implements `buffer-base-buffer` in `src/buffer.c` via `Fbuffer_base_buffer`,
//! which returns the base buffer of an indirect buffer.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_buffer_base_buffer_nil_for_ordinary_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (get-buffer-create "*neovm-test-bbb-ordinary*")
  (buffer-base-buffer (get-buffer "*neovm-test-bbb-ordinary*")))"#,
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_buffer_base_buffer_nil_arg_means_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (get-buffer-create "*neovm-test-bbb-current*")
  (set-buffer (get-buffer "*neovm-test-bbb-current*"))
  (buffer-base-buffer))"#,
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_buffer_base_buffer_nil_arg_same_as_no_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (get-buffer-create "*neovm-test-bbb-nil*")
  (set-buffer (get-buffer "*neovm-test-bbb-nil*"))
  (eq (buffer-base-buffer) (buffer-base-buffer nil)))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_buffer_base_buffer_wrong_type_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(buffer-base-buffer 42)");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_buffer_base_buffer_too_many_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(buffer-base-buffer nil nil)");
    assert_err_kind(&oracle, &neovm, "wrong-number-of-arguments");
}
