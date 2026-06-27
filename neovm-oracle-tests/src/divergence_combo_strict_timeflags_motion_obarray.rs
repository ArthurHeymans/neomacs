//! Strict combo oracle probes, batch 11: more format-time flags, fixed-time
//! zone introspection, skip-chars/syntax motion, char/bobp motion under
//! narrowing, number-sequence edges, obarray operations, and copy-sequence
//! over typed sequences.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_e6_format_time_more_flags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t0 (encode-time 5 30 9 4 7 2025 0)))
  (list (format-time-string "%k" t0 0)
        (format-time-string "%l" t0 0)
        (format-time-string "%e" t0 0)
        (format-time-string "%P" t0 0)
        (format-time-string "%C" t0 0)
        (format-time-string "%D" t0 0)
        (format-time-string "%F" t0 0)
        (format-time-string "%R" t0 0)
        (format-time-string "%T" t0 0)
        (format-time-string "%h" t0 0)
        (format-time-string "%n" t0 0)
        (format-time-string "%t" t0 0)))
"##,
    );
}

#[test]
fn div_e6_current_time_zone_fixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (current-time-zone (encode-time 0 0 12 1 1 2025 0) 0)
      (current-time-zone (encode-time 0 0 0 1 7 2025 0) 7200)
      (current-time-zone (encode-time 0 0 0 1 7 2025 0) -28800))
"##,
    );
}

#[test]
fn div_e6_skip_chars_and_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "aaabbb123ccc")
  (goto-char 1)
  (skip-chars-forward "a-z")
  (let ((p1 (point)))
    (skip-chars-forward "0-9")
    (list p1
          (point)
          (progn (skip-chars-backward "c") (point))
          (progn (goto-char 1) (skip-syntax-forward "w") (point))
          (progn (skip-syntax-forward "_") (point)))))
"##,
    );
}

#[test]
fn div_e6_char_motion_under_narrow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (goto-char 3)
  (narrow-to-region 2 5)
  (list (char-before)
        (char-after)
        (following-char)
        (preceding-char)
        (bobp)
        (eobp)
        (bolp)
        (eolp)
        (point-min)
        (point-max)))
"##,
    );
}

#[test]
fn div_e6_number_sequence_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (number-sequence 1 5)
      (number-sequence 1 10 2)
      (number-sequence 5 1 -1)
      (number-sequence 0 1 0.25)
      (number-sequence 1 1)
      (number-sequence 1 0))
"##,
    );
}

#[test]
fn div_e6_obarray_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ob (make-obarray 11)))
  (intern "foo" ob)
  (intern "bar" ob)
  (intern "baz" ob)
  (let ((count 0))
    (mapatoms (lambda (_) (setq count (1+ count))) ob)
    (list count
          (intern-soft "foo" ob)
          (intern-soft "missing" ob)
          (unintern "bar" ob)
          (intern-soft "bar" ob)
          (eq (intern "foo" ob) (intern "foo" ob)))))
"##,
    );
}

#[test]
fn div_e6_copy_sequence_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (copy-sequence "abc")
      (copy-sequence [1 2 3])
      (copy-sequence '(1 2 3))
      (copy-sequence (make-bool-vector 4 t))
      (eq (copy-sequence "abc") "abc")
      (equal (copy-sequence "abc") "abc")
      (length (copy-sequence (make-bool-vector 8 nil))))
"##,
    );
}
