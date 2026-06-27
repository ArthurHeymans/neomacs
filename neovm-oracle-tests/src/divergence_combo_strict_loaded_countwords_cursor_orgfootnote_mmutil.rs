//! Strict combo oracle probes, batch 73: count-words/line-number-at-pos,
//! what-cursor-position (char info at point), org-footnote parsing, and
//! mm-util (MIME charset detection).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_o7_count_words_and_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello world foo bar\nsecond line here\n")
  (list (count-words (point-min) (point-max))
        (count-lines (point-min) (point-max))
        (line-number-at-pos (point-max))
        (line-number-at-pos 10)))
"##,
    );
}

#[test]
fn div_o7_what_cursor_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "café")
  (goto-char 2)
  (what-cursor-position))
"##,
    );
}

#[test]
fn div_o7_org_footnote_labels() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1] more[fn:mylabel].\n\n[fn:1] First note.\n[fn:mylabel] Second note.\n")
  (sort (org-footnote-all-labels) #'string<))
"##,
        &["org/org.el", "org/org-footnote.el"],
    );
}

#[test]
fn div_o7_mm_util_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity_with_load(
        r##"
(list (mm-coding-system-p 'utf-8)
      (mm-coding-system-p 'nonexistent-probe-cs)
      (length (mm-find-mime-charset "café" 1 4)))
"##,
        &["gnus/mm-util.el"],
    );
}
