//! Complex combo batch 233 — `calc` deep: `math-eval`, `calc-eval` with
//! algebraic simplification, `math-read-expr`, radix conversions, and
//! symbolic computation availability.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx233_calc_eval_basic_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (list (calc-eval "1 + 2")
            (calc-eval "3 * 4")
            (calc-eval "10 / 3")
            (calc-eval "2^10")
            (calc-eval "sqrt(16)")
            (calc-eval "10!")
            (calc-eval "gcd(12, 18)")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_eval_algebraic_simplification() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (list (calc-eval "2x + 3x")
            (calc-eval "(a + b)^2")
            (calc-eval "sin(0)")
            (calc-eval "ln(1)")
            (calc-eval "exp(0)")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_radix_conversions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (let ((calc-number-radix 16))
        (list (calc-eval "255")
              (calc-eval "16")
              (let ((calc-number-radix 2)) (calc-eval "10"))
              (let ((calc-number-radix 8)) (calc-eval "64")))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_matrix_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (list (calc-eval "[1, 2; 3, 4] * [5, 6; 7, 8]")
            (calc-eval "det([1, 2; 3, 4])")
            (calc-eval "[1, 2, 3] + [4, 5, 6]")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_fraction_and_rational() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (list (calc-eval "1 / 3 + 1 / 6")
            (calc-eval "2 / 4")
            (calc-eval "1 / 2 * 2 / 3")
            (calc-eval "6 / 4")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_eval_trigonometric() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (list (calc-eval "sin(0)")
            (calc-eval "cos(0)")
            (calc-eval "tan(0)")
            (calc-eval "asin(1)")
            (calc-eval "acos(0)")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_math_read_expr_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'math-read-expr)
          (fboundp 'math-evaluate-expr)
          (fboundp 'calc-do-alg-entry)
          (boundp 'calc-language))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_modes_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (boundp 'calc-angle-mode)
          (boundp 'calc-complex-mode)
          (boundp 'calc-infinite-mode)
          (boundp 'calc-symbolic-mode)
          (boundp 'calc-display-just))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_log_and_exponential() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (list (calc-eval "log(100, 10)")
            (calc-eval "log10(100)")
            (calc-eval "ln(exp(1))")
            (calc-eval "exp(1)")
            (calc-eval "10^3")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx233_calc_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'calc)
      (let ((result (calc-eval "(1 + 2) * 3")))
        (with-temp-buffer
          (buffer-enable-undo)
          (insert (format "Calc mega: %s" result))
          (put-text-property 1 5 'face 'bold)
          (let ((m (set-marker (make-marker) 8))
                (ov (make-overlay 4 14)))
            (overlay-put ov 'face 'italic)
            (overlay-put ov 'evaporate t)
            (narrow-to-region 2 18)
            (let ((state (list result
                               (calc-eval "42")
                               (buffer-string)
                               (marker-position m)
                               (overlay-start ov) (overlay-end ov)
                               (text-properties-at 1))))
              (undo)
              (widen)
              (list state (buffer-string) (marker-position m)
                    (overlay-start ov) (overlay-end ov)
                    (text-properties-at 1)))))))
  (error (list :errored (car e))))
"##,
    );
}
