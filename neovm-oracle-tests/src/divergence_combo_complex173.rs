//! Complex combo batch 173 — `number` / `bignum` / `ratio` / `float`
//! extreme edge cases: precision overflow, signed zero, denormals,
//! most-positive-fixnum boundary, expt chains.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx173_fixnum_bignum_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((mpf most-positive-fixnum)
      (mnf most-negative-fixnum))
  (list mpf mnf
        (1+ mpf)
        (1- mnf)
        (* mpf 2)
        (* mnf 2)
        (expt 2 60)
        (expt 2 62)
        (expt 2 64)))
"##,
    );
}

#[test]
fn div_cx173_float_precision_overflow_underflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (* 1.0 1e308)
      (* 1.0 1e309)
      (* 1.0 1e-308)
      (* 1.0 1e-324)
      (/ 1.0 0.0)
      (/ -1.0 0.0)
      (/ 0.0 0.0)
      (+ 0.5 0.5 0.0)
      (- 0.0 0.0))
"##,
    );
}

#[test]
fn div_cx173_signed_zero_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (eq 0.0 0.0)
      (eq 0.0 -0.0)
      (= 0.0 0.0)
      (= 0.0 -0.0)
      (< 0.0 -0.0)
      (< -0.0 0.0)
      (= 0.0 0)
      (eq 0.0 0))
"##,
    );
}

#[test]
fn div_cx173_ratio_arithmetic_no_overflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (+ 1/2 1/3)
      (- 1/2 1/3)
      (* 1/2 1/3)
      (/ 1/2 1/3)
      (+ 1/2 1)
      (+ 1/2 0.5)
      (denominator 6/4)
      (numerator 6/4)
      (denominator 1/3)
      (numerator 1/3))
"##,
    );
}

#[test]
fn div_cx173_expt_chains_int_vs_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (expt 2 0)
      (expt 2 1)
      (expt 2 64)
      (expt 2 128)
      (expt 2 256)
      (expt 2 0.5)
      (expt 2 -1)
      (expt 2 -0.5)
      (expt 10 20)
      (expt 10 -20)
      (expt 0 0)
      (expt 0 1)
      (expt 1 100))
"##,
    );
}

#[test]
fn div_cx173_floor_ceiling_round_with_negative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (floor 2.7)
      (floor -2.7)
      (ceiling 2.3)
      (ceiling -2.3)
      (round 2.5)
      (round 3.5)
      (round -2.5)
      (round -3.5)
      (truncate 2.7)
      (truncate -2.7)
      (ffloor 2.7)
      (fceiling 2.3)
      (fround 2.5)
      (ftruncate 2.7))
"##,
    );
}

#[test]
fn div_cx173_modulo_with_negative_divisor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (% 7 3)
      (% -7 3)
      (% 7 -3)
      (% -7 -3)
      (mod 7 3)
      (mod -7 3)
      (mod 7 -3)
      (mod -7 -3)
      (mod 7.5 3)
      (mod -7.5 3))
"##,
    );
}

#[test]
fn div_cx173_bignum_factorial_via_reduction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((nums (list 10 20 30 40 50)))
  (mapcar (lambda (n)
            (cl-reduce #'* (number-sequence 1 n)))
          nums))
"##,
    );
}

#[test]
fn div_cx173_bignum_arithmetic_with_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((big (expt 2 256)))
  (list (format "%d" big)
        (format "%x" big)
        (format "%o" big)
        (format "%b" big)
        (length (format "%d" big))))
"##,
    );
}

#[test]
fn div_cx173_ash_overflow_to_bignum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((mpf most-positive-fixnum))
  (list mpf
        (ash mpf 1)
        (ash mpf 10)
        (ash mpf 64)
        (ash mpf -1)
        (ash mpf -10)
        (ash -1 -1)
        (ash -1 -64)))
"##,
    );
}

#[test]
fn div_cx173_float_formatting_precision() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (format "%f" 3.141592653589793)
      (format "%.2f" 3.141592653589793)
      (format "%.10f" 3.141592653589793)
      (format "%e" 3.141592653589793)
      (format "%g" 3.141592653589793)
      (format "%g" 0.000000001)
      (format "%g" 1000000000.0))
"##,
    );
}

#[test]
fn div_cx173_number_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((big (expt 2 128))
      (ratio 355/113)
      (pi-approx 3.141592653589793))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert (format "big=%s ratio=%s pi=%.15f" big ratio pi-approx))
    (put-text-property 1 5 'face 'bold)
    (let ((m (set-marker (make-marker) 10))
          (ov (make-overlay 4 18)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 25)
      (let ((state (list (integerp big)
                        (> big most-positive-fixnum)
                        (format "%d" big)
                        (number-to-string ratio)
                        (buffer-string)
                        (marker-position m)
                        (overlay-start ov) (overlay-end ov)
                        (text-properties-at 1))))
        (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}
