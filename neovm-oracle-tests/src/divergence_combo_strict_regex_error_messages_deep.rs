//! Strict combo oracle probes, batch 100: deep malformed-regex error messages
//! and C-level error diagnostic consistency.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_r4_malformed_regex_error_messages_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (string-match "[z-a]" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "[:" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "\\{3,2\\}" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "[a-Z]" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "\\?" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "\\+" "text") (invalid-regexp (cadr err)) (error 'other))
      (condition-case err (string-match "[^]" "text") (invalid-regexp (cadr err)) (error 'other)))
"##,
    );
}

#[test]
fn div_r4_c_level_error_messages() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (format "%d" "x") (wrong-type-argument (cdr err)) (error 'other))
      (condition-case err (aref "abc" nil) (wrong-type-argument (cdr err)) (error 'other))
      (condition-case err (aset [1 2 3] nil 9) (wrong-type-argument (cdr err)) (error 'other))
      (condition-case err (char-to-string nil) (wrong-type-argument (cdr err)) (error 'other))
      (condition-case err (make-string -1 ?x) (wrong-type-argument (cdr err)) (error 'other)))
"##,
    );
}
