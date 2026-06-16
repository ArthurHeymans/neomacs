//! Complex combo batch 295 — `number` arithmetic deep: `expt` with
//! negative exponents, `log` with base, `sqrt` negative, `gcd`/`lcm`
//! matrix, `floor`/`ceiling`/`round`/`truncate` with all sign combos.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx295_expt_with_negative_and_fractional() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (expt 2 -1)
      (expt 2 -2)
      (expt 2 0.5)
      (expt 2 -0.5)
      (expt 10 -3)
      (expt -1 0.5)
      (expt 8 1/3)
      (expt 0 0)
      (expt 1 1000))
"##,
    )
}

#[test]
fn div_cx295_log_with_various_bases() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (log 100)
      (log 100 10)
      (log exp)
      (log 1)
      (log 256 2)
      (log 1000 10))
"##,
    )
}

#[test]
fn div_cx295_gcd_lcm_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (gcd 12 18)
      (gcd 17 23)
      (gcd 100 75)
      (gcd 0 5)
      (lcm 4 6)
      (lcm 3 7)
      (lcm 12 18)
      (lcm 1 1))
"##,
    )
}

#[test]
fn div_cx295_floor_ceiling_round_truncate_all_signs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (floor 7 3) (floor -7 3) (floor 7 -3) (floor -7 -3)
      (ceiling 7 3) (ceiling -7 3) (ceiling 7 -3) (ceiling -7 -3)
      (round 7 3) (round -7 3) (round 7 -3) (round -7 -3)
      (truncate 7 3) (truncate -7 3) (truncate 7 -3) (truncate -7 -3))
"##,
    )
}

#[test]
fn div_cx295_mod_remainder_all_sign_combos() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (% 7 3) (% -7 3) (% 7 -3) (% -7 -3)
      (mod 7 3) (mod -7 3) (mod 7 -3) (mod -7 -3)
      (% 0 5) (mod 0 5))
"##,
    )
}

#[test]
fn div_cx295_bignum_factorial_fibonacci() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(letrec ((fact (lambda (n) (if (= n 0) 1 (* n (funcall fact (1- n))))))
         (fib (lambda (n) (if (< n 2) n (+ (funcall fib (1- n)) (funcall fib (- n 2)))))))
  (list (funcall fact 10)
        (funcall fact 20)
        (number-to-string (funcall fact 30))
        (funcall fib 10)
        (funcall fib 20)
        (funcall fib 30)))
"##,
    )
}

#[test]
fn div_cx295_ash_lsh_bignum_arguments() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((big (expt 2 64)))
  (list (ash big 1)
        (ash big 10)
        (ash big -1)
        (ash big -10)
        (lsh big 1)
        (logand big (1- big))
        (logior big 1)
        (logxor big big)))
"##,
    )
}

#[test]
fn div_cx295_sqrt_with_negative_returns_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (sqrt 16)
      (sqrt 2)
      (sqrt 0)
      (condition-case e (sqrt -1) (error (cons :err (car e))))
      (condition-case e (sqrt -4) (error (cons :err (car e)))))
"##,
    )
}

#[test]
fn div_cx295_ratio_arithmetic_full_reduction() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (+ 1/2 1/3)
      (- 5/6 1/2)
      (* 2/3 3/4)
      (/ 2/3 4/5)
      (+ 1/2 1/2)
      (* 6/4 2/3)
      (denominator 6/4)
      (numerator 6/4)
      (+ 1/2 0)
      (* 1/3 0))
"##,
    )
}

#[test]
fn div_cx295_number_arith_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(letrec ((fact (lambda (n) (if (= n 0) 1 (* n (funcall fact (1- n)))))))
  (let ((f10 (funcall fact 10))
        (big (expt 2 128))
        (ratio 355/113))
    (with-temp-buffer
      (buffer-enable-undo)
      (insert (format "Number mega: %d %s %s" f10 big ratio))
      (put-text-property 1 6 'face 'bold)
      (let ((m (set-marker (make-marker) 10))
            (ov (make-overlay 4 18)))
        (overlay-put ov 'face 'italic)
        (overlay-put ov 'evaporate t)
        (narrow-to-region 2 25)
        (let ((state (list f10 big ratio
                           (gcd f10 30)
                           (lcm 12 f10)
                           (log big 2)
                           (+ ratio 1/3)
                           (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (undo)
          (widen)
          (list state (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1)))))))
"##,
    )
}
