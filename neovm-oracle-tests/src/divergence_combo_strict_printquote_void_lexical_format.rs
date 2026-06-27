//! Strict combo oracle probes, batch 15: print-quoted, void/bound checks and
//! void-variable/void-function signaling, make-symbol vs intern identity,
//! let/let*/lexical-let shadowing and shared-mutable closures, format error
//! cases, invalid-codepoint string construction, and with-output-to-string.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_f0_print_quoted_and_macros() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (let ((print-quoted t)) (prin1-to-string '(quote x)))
      (let ((print-quoted t)) (prin1-to-string (list 'function 'foo)))
      (let ((print-quoted t)) (prin1-to-string ''(a b c)))
      (let ((print-quoted nil)) (prin1-to-string ''x))
      (read "'x")
      (read "#'foo"))
"##,
    );
}

#[test]
fn div_f0_void_and_bound_checks() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (boundp 'tab-width)
      (boundp 'nonexistent-probe-var-xyz)
      (fboundp 'car)
      (fboundp 'nonexistent-probe-fn-xyz)
      (progn (defvar probe-tmp-var-f0 5) (boundp 'probe-tmp-var-f0))
      (condition-case err (setq nonexistent-probe-var-xyz-f0 1)
        (void-variable (car err)))
      (condition-case err nonexistent-probe-var-xyz-f0b
        (void-variable (car err)))
      (condition-case err (nonexistent-probe-fn-call-f0 1)
        (void-function (car err))))
"##,
    );
}

#[test]
fn div_f0_make_symbol_vs_intern_identity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s1 (make-symbol "probe"))
      (s2 (make-symbol "probe")))
  (list (eq s1 s2)
        (eq (intern "foo") (intern "foo"))
        (symbol-name s1)
        (symbolp s1)
        (intern-soft "car")
        (not (eq s1 (intern "probe")))))
"##,
    );
}

#[test]
fn div_f0_let_lexical_shadowing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (let ((x 1) (y 2)) (let ((x 3)) (list x y)))
      (let* ((x 1) (y (1+ x))) (list x y))
      (let ((x 1)) (let ((x 2)) (let ((x 3)) x)))
      (lexical-let ((counter 0))
        (let ((f1 (lambda () (setq counter (1+ counter)))))
          (list (funcall f1) (funcall f1) (funcall f1)))))
"##,
    );
}

#[test]
fn div_f0_format_error_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (format "%") (error (car err)))
      (condition-case err (format "%5") (error (car err)))
      (condition-case err (format "%d" 1 2) (error (car err)))
      (condition-case err (format "%d" "x") (error (car err)))
      (condition-case err (format "%s" 1 2) (error (car err)))
      (format "%s" 1))
"##,
    );
}

#[test]
fn div_f0_string_invalid_codepoints() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case err (make-string 3 1114112) (error (car err)))
      (condition-case err (string 1114112) (error (car err)))
      (condition-case err (char-to-string 1114112) (error (car err)))
      (make-string 3 65)
      (string 65 66 67)
      (length (make-string 3 128578)))
"##,
    );
}

#[test]
fn div_f0_with_output_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-output-to-string (princ "hello") (princ " world"))
      (with-output-to-string (print 'a) (princ "b"))
      (with-temp-message "probe-temp-msg" (current-message)))
"##,
    );
}
