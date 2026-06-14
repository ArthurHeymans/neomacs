//! Calc subsystem divergence probes (calibration).
//!
//! Probes deterministic exact arithmetic via calc-eval (integers, fractions,
//! powers, factorials, gcd, sqrt, binomial) and raw math-* functions. Calc is
//! a large standalone subsystem; a partial port diverges here.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

fn req() {}
#[test]
fn div_calc_int_arith() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "2+2")
        (calc-eval "17*23")
        (calc-eval "1000-333")
        (calc-eval "144/12")))
"##,
    );
}

#[test]
fn div_calc_big_powers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "2^100")
        (calc-eval "3^50")
        (calc-eval "2^10")))
"##,
    );
}

#[test]
fn div_calc_factorial() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "factorial(5)")
        (calc-eval "30!")
        (calc-eval "20!")))
"##,
    );
}

#[test]
fn div_calc_fractions_exact() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "1/3 + 1/6")
        (calc-eval "1/7 * 1/7")
        (calc-eval "2/4")
        (calc-eval "1/3 - 1/6")))
"##,
    );
}

#[test]
fn div_calc_gcd_lcm() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "gcd(12, 18)")
        (calc-eval "gcd(48, 36)")
        (calc-eval "lcm(4, 6)")))
"##,
    );
}

#[test]
fn div_calc_sqrt_and_integer_root() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "sqrt(16)")
        (calc-eval "sqrt(2)^2")
        (calc-eval "2^(1/2)^2")))
"##,
    );
}

#[test]
fn div_calc_modulo_divmod() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "17 % 5")
        (calc-eval "100 % 7")
        (calc-eval "-17 % 5")))
"##,
    );
}

#[test]
fn div_calc_binomial() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "16 choose 4")
        (calc-eval "10 choose 3")
        (calc-eval "5 perm 3")))
"##,
    );
}

#[test]
fn div_calc_raw_math_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (math-mul 6 7)
        (math-add (math-mul 2 3) 4)
        (math-div '(frac 1 3) '(frac 1 6))
        (math-pow 2 10)))
"##,
    );
}

#[test]
fn div_calc_decimal_floats() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "1.5 + 2.5")
        (calc-eval "0.1 + 0.2")
        (calc-eval "2.5 * 4")))
"##,
    );
}

#[test]
fn div_calc_comparison_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "5 > 3")
        (calc-eval "2 = 3")
        (calc-eval "10 <= 10")))
"##,
    );
}

#[test]
fn div_calc_prime_factor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    req();
    assert_oracle_parity(
        r##"
(progn (require 'calc) (require 'calc-ext)
  (list (calc-eval "prfac(360)")
        (calc-eval "prfac(17)")))
"##,
    );
}
