//! Divergence tests: memory-layout, fixnum overflow, bignum edge cases.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_fixnum_arithmetic_overflow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (1+ most-positive-fixnum)
  (1- most-negative-fixnum)
  (+ most-positive-fixnum 1)
  (- most-negative-fixnum 1))"#,
    );
}

#[test]
fn divergence_bignum_multiply() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((big (* most-positive-fixnum most-positive-fixnum)))
  (list (> big most-positive-fixnum)
        (integerp big)
        (> (length (number-to-string big)) 10)))"#,
    );
}

#[test]
fn divergence_bignum_expt() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((big (expt 2 100)))
  (list big
        (= big 1267650600228229401496703205376)
        (integerp big)))"#,
    );
}

#[test]
fn divergence_bignum_division() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((big (expt 10 50))
         (div (expt 10 25))
         (result (/ big div)))
  (list result
        (= result (expt 10 25))
        (mod big div)))"#,
    );
}

#[test]
fn divergence_float_bignum_mixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (float (expt 2 100))
  (> (float (expt 2 100)) 0.0)
  (integerp (expt 2 100))
  (floatp (float (expt 2 100))))"#,
    );
}

#[test]
fn divergence_ash_bitwise_shift() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (ash 1 10)
  (ash 1 -1)
  (ash most-positive-fixnum 1)
  (ash -1 8))"#,
    );
}

#[test]
fn divergence_logand_logior() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (logand #xFF #x0F)
  (logior #xF0 #x0F)
  (logxor #xFF #x0F)
  (lognot 0)
  (lognot -1))"#,
    );
}

#[test]
fn divergence_logbitp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (logbitp 0 1)
  (logbitp 1 1)
  (logbitp 7 128)
  (logcount 255)
  (integer-length 255))"#,
    );
}

#[test]
fn divergence_bignum_equality() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((a (expt 2 100))
        (b (expt 2 100)))
  (list (eq a b)
        (eql a b)
        (equal a b)
        (= a b)))"#,
    );
}

#[test]
fn divergence_bignum_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((big1 (expt 2 100))
        (big2 (expt 2 101)))
  (list (< big1 big2)
        (> big2 big1)
        (<= big1 big2)
        (>= big2 big1)
        (/= big1 big2)))"#,
    );
}
