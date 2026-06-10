//! Divergence tests: real arithmetic & number behavioral differences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_bignum_arithmetic_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((a (expt 2 100))
        (b (expt 2 99)))
  (list (= a (* b 2))
        (< b a)
        (= (mod a 7) (mod (* b 2) 7))
        (/ a b)
        (1+ (expt 10 50))
        (= (expt 2 64) 18446744073709551616))) ",
    );
}

#[test]
fn divergence_float_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (floatp 1.0e+INF)
  (isnan 1.0e+NaN)
  (isnan 0.0)
  (< 0.0 -0.0)
  (= 0.0 -0.0)
  (/ 1.0 0.0)
  (/ -1.0 0.0)
  (= (+ 1.0e+INF 1.0) 1.0e+INF)
  (= (* 0.0 1.0e+INF) (* 0.0 1.0e+INF))) ",
    );
}

#[test]
fn division_rounding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (/ 7 2)
  (/ 7 -2)
  (/ -7 2)
  (/ -7 -2)
  (mod 7 3)
  (mod -7 3)
  (mod 7 -3)
  (% 7 3)
  (% -7 3)) ",
    );
}

#[test]
fn divergence_trig_math() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((pi 3.141592653589793))
  (list (< (abs (- (cos 0.0) 1.0)) 1e-10)
        (< (abs (- (sin pi) 0.0)) 1e-6)
        (< (abs (- (sqrt 2.0) 1.414213562)) 1e-6)
        (< (abs (- (log (exp 1.0)) 1.0)) 1e-10)
        (= (abs -5) 5)
        (= (max 1 2 3) 3)
        (= (min 1 2 3) 1))) ",
    );
}

#[test]
fn divergence_number_type_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (integerp 42)
  (integerp 42.0)
  (floatp 42)
  (floatp 42.0)
  (numberp 42)
  (numberp 42.0)
  (numberp \"42\")
  (natnump 0)
  (natnump -1)
  (natnump (expt 2 200))
  (zerop 0)
  (zerop 0.0)
  (cl-typep 42 'integer)
  (cl-typep 42.0 'float)) ",
    );
}

#[test]
fn divergence_comparison_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (= 1 1)
  (= 1 1.0)
  (equal 1 1)
  (equal 1 1.0)
  (eql 1 1)
  (eql 1 1.0)
  (eq 1 1)
  (/= 1 2)
  (>= 5 5)
  (> 5 4)
  (<= 3 3)) ",
    );
}

#[test]
fn divergence_bitwise_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (ash 1 4)
  (ash 16 -1)
  (logand 15 6)
  (logior 5 10)
  (logxor 5 3)
  (lognot 0)
  (lognot -1)
  (= (ash (expt 2 63) -63) 1)) ",
    );
}

#[test]
fn divergence_random_and_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (number-to-string 42)
  (number-to-string 3.14)
  (string-to-number \"42\")
  (string-to-number \"3.14\")
  (string-to-number \"0xff\" 16)
  (string-to-number \"1010\" 2)) ",
    );
}

#[test]
fn divergence_rounding_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (floor 3.7)
  (floor -3.7)
  (ceiling 3.2)
  (ceiling -3.2)
  (round 3.5)
  (round 2.5)
  (round -2.5)
  (truncate 3.7)
  (truncate -3.7)) ",
    );
}

#[test]
fn divergence_bignum_comparison() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((big1 (expt 2 256))
        (big2 (expt 2 256)))
  (list (= big1 big2)
        (equal big1 big2)
        (eq big1 big2)
        (eql big1 big2)
        (< big1 (1+ big2))
        (<= big1 big2))) ",
    );
}
