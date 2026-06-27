//! Strict combo oracle probes, batch 102: regex RE_DUP_MAX boundary and nested
//! quantifiers (a**, a*+).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_r6_regex_dupmax_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (string-match "a\\{32767\\}" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a\\{32768\\}" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a\\{65535\\}" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a\\{65536\\}" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a\\{100000\\}" "text") (invalid-regexp (cadr err)) (error 'other)))
"##,
    );
}

#[test]
fn div_r6_regex_nested_quantifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (string-match "a**" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a*+" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a++" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a??" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a\\{2\\}*" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "a\\{2,3\\}+" "text") (invalid-regexp (cadr err)) (error 'other)))
"##,
    );
}
