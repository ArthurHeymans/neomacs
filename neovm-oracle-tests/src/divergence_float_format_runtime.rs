//! Float formatting parity: %g trailing-zero stripping, %e/%.Ne exponent,
//! %.Nf rounding (round-half-to-even), width/pad/sign/space flags, -0.0,
//! %d/%x/%o of bignums, very large/small floats, number-to-string +
//! string-to-number roundtrip; plus the %E/%G uppercase-conversion divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn ff_float_to_string_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((vals '(0.1 0.2 0.3 1.5 3.14159265358979 1e100 1e-100)))
  (cl-every (lambda (v) (= v (string-to-number (number-to-string v)))) vals))"##,
    );
}

#[test]
fn ff_format_d_bignum() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%d" (expt 2 70)) (format "%d" (- (expt 2 70)))
        (format "%x" (expt 2 64)) (format "%o" (expt 2 30)))"##,
    );
}

#[test]
fn ff_format_e_exponent() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%e" 1.0) (format "%e" 12345.678) (format "%e" 0.000123) (format "%.2e" 999.9))"##,
    );
}

#[test]
fn ff_format_f_precision() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%.0f" 2.5) (format "%.0f" 3.5) (format "%.3f" 3.14159)
        (format "%f" 0.1) (format "%.10f" (/ 1.0 3.0)) (format "%.0f" 0.5))"##,
    );
}

#[test]
fn ff_format_f_width_pad() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%10.2f" 3.14) (format "%-10.2f|" 3.14)
        (format "%010.2f" 3.14) (format "%+.2f" 3.14) (format "% .2f" 3.14))"##,
    );
}

#[test]
fn ff_format_g_trailing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%g" 1.0) (format "%g" 1.5) (format "%g" 100000.0)
        (format "%g" 1000000.0) (format "%g" 0.0001) (format "%g" 0.00001) (format "%g" 1.23456789))"##,
    );
}

#[test]
fn ff_format_large_small_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%g" 1e308) (format "%g" 1e-308) (format "%f" 1e20)
        (format "%.2e" 1.7976931348623157e308))"##,
    );
}

#[test]
fn ff_format_negative_zero() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%f" -0.0) (format "%g" -0.0) (format "%e" -0.0)
        (format "%.1f" -0.04) (format "%d" -0))"##,
    );
}

#[test]
fn ff_format_percent_combos() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%5.1f%%" 95.5) (format "%d/%d" 3 4)
        (format "%+05d" 42) (format "%x %X" 255 255) (format "%#b" 5))"##,
    );
}

#[test]
fn ff_number_to_string_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (number-to-string 1.0) (number-to-string 0.1)
        (number-to-string 1e20) (number-to-string (/ 2.0 3.0)) (number-to-string -0.0))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: format accepts %E and %G (uppercase float conversions) and produces output, whereas GNU signals (error \"Invalid format operation %E\" / %G). neomacs is more permissive than GNU's format here."]
fn divergence_format_uppercase_e_g() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (condition-case e (format "%E" 1.0) (error 'err))
      (condition-case e (format "%G" 1500.0) (error 'err)))"##,
    );
}
