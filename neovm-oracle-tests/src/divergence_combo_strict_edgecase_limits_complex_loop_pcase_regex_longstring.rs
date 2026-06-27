//! Strict combo oracle probes, batch 86: EDGE-CASE LIMITS — complex cl-loop
//! (multiple into vars, when/else, finally return), pcase with combined
//! backquote/guard/app patterns, regex on CJK/multibyte text, very-long-string
//! operations (10K chars), and deeply-nested print-depth limits.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_q0_complex_cl_loop_accumulators() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-loop for x in '(1 2 3 4 5)
               for y in '(a b c d e)
               maximize x into max-x
               collect (cons y x) into pairs
               finally (return (list max-x (nreverse pairs))))
      (cl-loop for i from 1 to 10
               when (= (% i 2) 0)
               collect i into evens
               else collect i into odds
               finally (return (list evens odds)))
      (cl-loop with sum = 0
               for x across "abc"
               do (setq sum (+ sum x))
               finally (return sum)))
"##,
    );
}

#[test]
fn div_q0_pcase_combined_patterns() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (pcase '(1 (2 3) "str")
        (`(,a (,b ,c) ,s)
         (guard (and (numberp a) (stringp s)))
         (list 'matched a b c s)))
      (pcase 42
        ((and (pred numberp) (pred (> 40))) 'big)
        (_ 'small))
      (pcase "hello"
        ((app length n) (when (> n 3) 'long))))
"##,
    );
}

#[test]
fn div_q0_regex_on_cjk_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (replace-regexp-in-string "[一-龯]+" "#" "日本語テスト English")
      (string-match-p "[[:alpha:]]+" "café日本")
      (replace-regexp-in-string "\\(.\\)\\1" "<\\1>" "aabbccdd"))
"##,
    );
}

#[test]
fn div_q0_very_long_string_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s (make-string 10000 ?x)))
  (list (length s)
        (substring s 5000 5005)
        (length (format "%s" s))
        (string-match "xxxxx" s)
        (length (upcase s))))
"##,
    );
}

#[test]
fn div_q0_deep_nesting_print_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((deep 0))
  (dotimes (i 50 deep) (setq deep (list deep))))
"##,
    );
}
