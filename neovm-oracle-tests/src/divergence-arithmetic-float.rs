//! Divergence tests: arithmetic edge cases and float semantics.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_bignum_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((a (expt 2 64))
        (b (expt 2 63)))
  (list (+ a b)
        (- a b)
        (* a 2)
        (/ a b)
        (mod a b)
        (1+ a)
        (1- a)))"#,
    );
}

#[test]
fn divergence_float_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (+ 1.0 2.0)
  (/ 10.0 3.0)
  (sqrt 2.0)
  (abs -3.5)
  (max 1 2 3)
  (min 3 2 1)
  (< 1.0 2)
  (> 2 1.5)
  (= 3 3.0))"#,
    );
}

#[test]
fn divergence_float_special_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (isnan 0.0e+NaN)
  (isnan 1.0)
  (< 0.0e+NaN 1.0)
  (= 1.0e+INF 1.0e+INF)
  (< 1.0e+INF most-positive-fixnum))"#,
    );
}

#[test]
fn divergence_bitwise_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (ash 1 10)
  (ash 1 -1)
  (logand 255 15)
  (logior 1 2 4)
  (logxor 15 6)
  (lognot 0)
  (logcount 255))"#,
    );
}

#[test]
fn divergence_trig_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (sin 0.0)
  (cos 0.0)
  (tan 0.0)
  (> (sin 1.5707963267948966) 0.999)
  (< (abs (- (cos 3.141592653589793) -1.0)) 0.0001))"#,
    );
}

#[test]
fn divergence_fixnum_overflow_to_bignum() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (most-positive-fixnum)
  (most-negative-fixnum)
  (1+ (most-positive-fixnum))
  (1- (most-negative-fixnum))
  (> (1+ (most-positive-fixnum)) (most-positive-fixnum)))"#,
    );
}

#[test]
fn division_by_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(condition-case err
    (/ 1 0)
  (arith-error (list 'caught (car err))))"#,
    );
}
