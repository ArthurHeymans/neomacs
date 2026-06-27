//! Strict combo oracle probes, batch 99: error-condition consistency —
//! wrong-type-argument, args-out-of-range, arith-error, and invalid-regexp
//! error types from malformed regexes.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_r3_error_condition_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (car 1) (wrong-type-argument (car err)) (error 'other))
      (condition-case err (aref [1 2] 5) (args-out-of-range (car err)) (error 'other))
      (condition-case err (aset "abc" 5 ?x) (args-out-of-range (car err)) (error 'other))
      (condition-case err (/ 1 0) (arith-error (car err)) (error 'other))
      (condition-case err (string-to-number nil) (wrong-type-argument (car err)) (error 'other)))
"##,
    );
}

#[test]
fn div_r3_malformed_regex_error_handling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (string-match "(" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "*" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "[a-" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "\\(a" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "\\)" "text") (invalid-regexp (cadr err)) (error 'other)))
"##,
    );
}

#[test]
fn div_r3_buffer_read_only_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc")
  (setq buffer-read-only t)
  (list (condition-case err (insert "x") (buffer-read-only (car err)) (error 'other))
        (condition-case err (erase-buffer) (buffer-read-only (car err)) (error 'other))
        (condition-case err (delete-char -1) (buffer-read-only (car err)) (error 'other))))
"##,
    );
}
