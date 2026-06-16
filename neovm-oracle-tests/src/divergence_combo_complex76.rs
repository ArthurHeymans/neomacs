//! Complex combo batch 76 — number / bignum / float edge cases: bignum
//! arithmetic, exact ratios, float precision, NaN/inf handling, `expt`,
//! `log`, `sqrt`, `mod`/`%` with negatives, `floor`/`ceiling`/`round`
//! behavior and `ash`/`lsh` across widths.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx76_bignum_factorial_and_comparison() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(letrec ((fact (lambda (n) (if (= n 0) 1 (* n (funcall fact (1- n)))))))
  (list (funcall fact 10)
        (funcall fact 20)
        (funcall fact 30)
        (funcall fact 40)
        (number-to-string (funcall fact 50))))
"##,
    );
}

#[test]
fn div_cx76_integer_overflow_into_bignum_seamless() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((n most-positive-fixnum))
  (list n
        (+ n 1)
        (* n 2)
        (* n n)
        (* n n n)
        (expt 2 64)
        (expt 2 128)
        (expt 2 256)))
"##,
    );
}

#[test]
fn div_cx76_ratios_and_exact_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (/ 1 3)
      (/ 6 4)
      (+ 1/2 1/3)
      (- 5/6 1/2)
      (* 2/3 3/4)
      (/ 2/3 4/5)
      (+ 1/2 1)
      (denominator 6/4)
      (numerator 6/4))
"##,
    );
}

#[test]
fn div_cx76_float_precision_and_formatting() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list 0.1
      (+ 0.1 0.2)
      (* 0.1 0.1)
      (- 1.0 0.9)
      (/ 1.0 3.0)
      (* 1.0 1e308)
      1e-300
      (* 1.0 1e-300 1e10))
"##,
    );
}

#[test]
fn div_cx76_nan_inf_predicates_and_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((inf (/ 1.0 0.0))
      (neginf (/ -1.0 0.0))
      (nan (/ 0.0 0.0)))
  (list (numberp inf)
        (floatp inf)
        (= inf inf)
        (eq inf inf)
        (< neginf inf)
        (numberp nan)
        (= nan nan)
        (< nan 0)
        (< 0 nan)
        (= nan nan)))
"##,
    );
}

#[test]
fn div_cx76_modulo_and_remainder_with_negatives() {
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
      (floor 7 3)
      (floor -7 3)
      (ceiling 7 3)
      (ceiling -7 3))
"##,
    );
}

#[test]
fn div_cx76_floor_ceiling_truncate_round_to_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (floor 2.7)
      (ceiling 2.3)
      (truncate 2.7)
      (round 2.5)
      (round 2.4)
      (round 2.6)
      (round 3.5)
      (round -3.5)
      (fround 2.5)
      (ffloor 2.7)
      (fceiling 2.3)
      (ftruncate 2.7))
"##,
    );
}

#[test]
fn div_cx76_ash_lsh_logand_logior_logxor_across_widths() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (ash 1 0)
      (ash 1 10)
      (ash 1 32)
      (ash 1 64)
      (ash 1 128)
      (ash 256 -1)
      (ash -1 -1)
      (lsh 1 10)
      (logand #xff #x0f)
      (logior #xf0 #x0f)
      (logxor #xff #x0f)
      (lognot #x00)
      (logcount 255)
      (logcount -1))
"##,
    );
}

#[test]
fn div_cx76_expt_log_sqrt_pow_with_int_and_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (expt 2 10)
      (expt 2 0)
      (expt 2 -1)
      (expt 2.0 0.5)
      (expt 10 20)
      (log 100)
      (log 100 10)
      (log exp)
      (sqrt 16)
      (sqrt 2)
      (sqrt -1)
      (expt 8 1/3))
"##,
    );
}

#[test]
fn div_cx76_number_predicates_with_bignum_and_floats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((big (expt 2 100))
      (ratio 1/3)
      (flt 3.14))
  (list (integerp big)
        (integerp ratio)
        (integerp flt)
        (floatp big)
        (floatp ratio)
        (floatp flt)
        (numberp big)
        (numberp ratio)
        (numberp flt)
        (wholenump 5)
        (wholenump -1)
        (natnump big)))
"##,
    );
}

#[test]
fn div_cx76_trigonometry_functions_pi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (sin 0)
      (cos 0)
      (tan 0)
      (sin (/ pi 2))
      (cos (/ pi 2))
      (asin 1)
      (acos 0)
      (atan 1)
      (atan 1 1)
      (float pi)
      (float most-positive-fixnum))
"##,
    );
}

#[test]
fn div_cx76_random_with_seed_reproducibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((seed (random))
      (r1 (progn (random seed) (random 1000)))
      (r2 (progn (random seed) (random 1000))))
  (list (= r1 r2)
        (integerp r1)
        (>= r1 0)
        (< r1 1000)))
"##,
    );
}

#[test]
fn div_cx76_format_specifiers_for_bignum_and_ratios() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((big (expt 2 128))
      (ratio 355/113))
  (list (format "%d" big)
        (format "%x" big)
        (format "%o" big)
        (format "%b" big)
        (format "%S" ratio)
        (format "%f" ratio)
        (format "%.10f" ratio)
        (number-to-string big)))
"##,
    );
}

#[test]
fn div_cx76_arithmetic_with_marker_overlay_textprop_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(letrec ((fib (lambda (n) (if (< n 2) n (+ (funcall fib (1- n)) (funcall fib (- n 2)))))))
  (let ((fib10 (funcall fib 10))
        (big (+ most-positive-fixnum (funcall fib 20))))
    (with-temp-buffer
      (buffer-enable-undo)
      (insert (format "fib10=%d big=%s" fib10 big))
      (put-text-property 1 5 'face 'bold)
      (let ((m (set-marker (make-marker) 8))
            (ov (make-overlay 4 14)))
        (overlay-put ov 'face 'italic)
        (overlay-put ov 'evaporate t)
        (narrow-to-region 2 20)
        (let ((state (list fib10 big (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (undo)
          (widen)
          (list state
                (buffer-string) (marker-position m)
                (overlay-start ov)
                (text-properties-at 1)))))))
"##,
    );
}
