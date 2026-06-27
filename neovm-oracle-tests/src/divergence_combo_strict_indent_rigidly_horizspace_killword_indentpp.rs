//! Strict combo oracle probes, batch 96: indent-rigidly (shift text), delete-
//! horizontal-space, kill-word/backward-kill-word, and indent-pp.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_r0_indent_rigidly_and_horiz_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-temp-buffer
        (insert "line1\nline2\nline3\n")
        (indent-rigidly 1 15 3)
        (buffer-string))
      (with-temp-buffer
        (insert "  hello   world  ")
        (goto-char 5)
        (delete-horizontal-space)
        (buffer-string)))
"##,
    );
}

#[test]
fn div_r0_kill_word_and_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-temp-buffer
        (insert "one two three four")
        (goto-char 1)
        (kill-word 2)
        (buffer-string))
      (with-temp-buffer
        (insert "one two three four")
        (goto-char 19)
        (backward-kill-word 2)
        (buffer-string)))
"##,
    );
}

#[test]
fn div_r0_indent_pp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "((a . 1)\n(b . (2 3))\n(c . 4))")
  (goto-char 1)
  (indent-pp)
  (buffer-string))
"##,
    );
}
