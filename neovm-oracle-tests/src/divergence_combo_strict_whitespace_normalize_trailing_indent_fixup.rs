//! Strict combo oracle probes, batch 95: whitespace normalization —
//! delete-trailing-whitespace, canonically-space-region, delete-indentation,
//! and fixup-whitespace.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_q9_delete_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "line1   \nline2\t\nline3   \n")
  (delete-trailing-whitespace (point-min) (point-max))
  (buffer-string))
"##,
    );
}

#[test]
fn div_q9_canonically_space_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "a  b   c    d")
  (canonically-space-region 1 15)
  (buffer-string))
"##,
    );
}

#[test]
fn div_q9_delete_indentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "line one\n  line two")
  (goto-char 9)
  (delete-indentation)
  (buffer-string))
"##,
    );
}

#[test]
fn div_q9_fixup_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "a   b")
  (goto-char 2)
  (fixup-whitespace)
  (buffer-string))
"##,
    );
}

#[test]
fn div_q9_just_one_space_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "  x   y   z  ")
  (goto-char 1)
  (while (re-search-forward "  +" nil t)
    (replace-match " "))
  (buffer-string))
"##,
    );
}
