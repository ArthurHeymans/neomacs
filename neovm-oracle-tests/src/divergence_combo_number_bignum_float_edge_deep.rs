//! Deep combo: number arithmetic + bignum + float edge cases + comparison.
//! Tests numeric precision, overflow, and type interactions.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_bignum_multiplication_precision() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((big (* 123456789 987654321)))\n\
         (list big (integerp big) (> big most-positive-fixnum))))",
    );
}

#[test]
fn deficiency_float_integer_mixed_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (type-of (+ 1 2.0))\n\
         (type-of (* 3 4.5))\n\
         (type-of (/ 10 3))\n\
         (type-of (/ 10 3.0))\n\
         (/ 10 3.0)))",
    );
}

#[test]
fn deficiency_comparison_with_mixed_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (= 1 1.0)\n\
         (= 1 1.0 1)\n\
         (< 1 2.0)\n\
         (<= 1 1.0)\n\
         (> 2.0 1)\n\
         (>= 1.0 1)))",
    );
}

#[test]
fn deficiency_float_special_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (isnan? 0.0e+NaN)\n\
         (/ 1.0 0.0)\n\
         (/ -1.0 0.0)\n\
         (isnan? (/ 0.0 0.0))\n\
         (float-nan-p 0.0e+NaN)))",
    );
}

#[test]
fn deficiency_ash_with_negative_shift() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (ash 16 -1)\n\
         (ash 16 -2)\n\
         (ash 16 -4)\n\
         (ash 1 60)\n\
         (ash (ash 1 60) -58)))",
    );
}

#[test]
fn deficiency_logand_logior_logxor_patterns() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (logand #b1100 #b1010)\n\
         (logior #b1100 #b1010)\n\
         (logxor #b1100 #b1010)\n\
         (lognot 0)\n\
         (logand #xFF #x0F)\n\
         (logior #xF0 #x0F)))",
    );
}

#[test]
fn deficiency_abs_mod_round_trunc_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (abs -42)\n\
         (abs -3.14)\n\
         (mod 10 3)\n\
         (mod -10 3)\n\
         (round 3.5)\n\
         (round 4.5)\n\
         (truncate -3.7)\n\
         (floor -3.7)\n\
         (ceiling -3.7)))",
    );
}

#[test]
fn deficiency_number_to_string_formatting() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (number-to-string 42)\n\
         (number-to-string -3.14)\n\
         (string-to-number \"42\")\n\
         (string-to-number \"3.14\")\n\
         (string-to-number \"ff\" 16)\n\
         (string-to-number \"1010\" 2)))",
    );
}

#[test]
fn deficiency_expt_sqrt_with_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (expt 2 10)\n\
         (expt 2 0)\n\
         (expt 2 -1)\n\
         (sqrt 4)\n\
         (sqrt 2)\n\
         (> (sqrt 2) 1.414)))",
    );
}

#[test]
fn deficiency_gcd_lcm_with_multiple_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (gcd 12 8)\n\
         (gcd 100 75 50)\n\
         (lcm 4 6)\n\
         (lcm 3 4 5)\n\
         (gcd 0 5)\n\
         (lcm 0 5)))",
    );
}
