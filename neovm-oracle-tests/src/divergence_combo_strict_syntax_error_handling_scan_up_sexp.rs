//! Strict combo oracle probes, batch 98: syntax error handling — scan-lists,
//! up-list, forward-sexp, backward-sexp with excessive counts / unbalanced
//! parens, checking whether scan-error vs wrong-type-argument is signaled.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_r2_syntax_error_handling_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a b (c d))")
  (goto-char 1)
  (list (condition-case err (scan-lists 1 99 0) (scan-error (car err)) (error 'other-error))
        (condition-case err (scan-lists 1 -99 0) (scan-error (car err)) (error 'other-error))
        (condition-case err (up-list 99) (scan-error (car err)) (error 'other-error))
        (condition-case err (up-list -99) (scan-error (car err)) (error 'other-error))
        (condition-case err (forward-sexp 99) (scan-error (car err)) (error 'other-error))
        (condition-case err (backward-sexp 99) (scan-error (car err)) (error 'other-error))))
"##,
    );
}

#[test]
fn div_r2_scan_sexps_error_handling() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a) (b) (c)")
  (goto-char 1)
  (list (condition-case err (scan-sexps 1 99) (scan-error (car err)) (error 'other-error))
        (condition-case err (scan-sexps 1 -99) (scan-error (car err)) (error 'other-error))
        (scan-sexps 1 1)
        (scan-sexps 1 2)))
"##,
    );
}
