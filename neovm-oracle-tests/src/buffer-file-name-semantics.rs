//! Oracle parity tests for `buffer-file-name`.
//!
//! GNU implements `buffer-file-name` in `src/buffer.c` via `Fbuffer_file_name`,
//! which calls `BVAR(decode_buffer(buffer), filename)`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_buffer_file_name_current_buffer_nil_for_non_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (get-buffer-create "*neovm-test-nonfile*")
  (set-buffer (get-buffer "*neovm-test-nonfile*"))
  (buffer-file-name))"#,
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_buffer_file_name_nil_arg_means_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (get-buffer-create "*neovm-test-bfn*")
  (set-buffer (get-buffer "*neovm-test-bfn*"))
  (list
   (buffer-file-name)
   (buffer-file-name nil)))"#,
    );
    assert_ok_eq("(nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_buffer_file_name_killed_buffer_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (let ((b (get-buffer-create "*neovm-test-killed*")))
    (kill-buffer b)
    (list (buffer-file-name b) (buffer-live-p b))))"#,
    );
    assert_ok_eq("(nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_buffer_file_name_wrong_type_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(buffer-file-name 42)");
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");

    let (oracle2, neovm2) = eval_oracle_and_neovm("(buffer-file-name 'some-symbol)");
    assert_err_kind(&oracle2, &neovm2, "wrong-type-argument");
}

#[test]
fn oracle_buffer_file_name_too_many_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = eval_oracle_and_neovm("(buffer-file-name nil nil)");
    assert_err_kind(&oracle, &neovm, "wrong-number-of-arguments");
}
