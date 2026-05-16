//! Oracle parity tests for buffer undo, local-value, posix regex, ntake.
//!
//! Covers: `buffer-enable-undo`, `buffer-local-value`,
//! `posix-looking-at`, `posix-string-match`, `ntake`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_buffer_enable_undo_no_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-undo*"))
  (buffer-enable-undo)
  t)"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_buffer_local_value_returns_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (set (make-local-variable 'neovm--test-blv) 77)
  (let ((buf (current-buffer)))
    (buffer-local-value 'neovm--test-blv buf)))"#,
    );
    assert_ok_eq("77", &oracle, &neovm);
}

#[test]
fn oracle_posix_looking_at_matches_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (switch-to-buffer (get-buffer-create "*neovm-test-posix*"))
  (erase-buffer)
  (insert "hello world")
  (goto-char 1)
  (posix-looking-at "hello"))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_posix_string_match_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(posix-string-match "foo" "foobar")"#);
    assert_ok_eq("0", &oracle, &neovm);
}

#[test]
fn oracle_ntake_takes_from_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(r#"(ntake 2 '(a b c d e))"#);
    assert_ok_eq("(a b)", &oracle, &neovm);
}
