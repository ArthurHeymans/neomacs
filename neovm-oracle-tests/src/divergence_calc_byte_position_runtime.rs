//! calc-eval parity (precedence, base #, trig/exp/ln, fractions, factorial,
//! round/floor/ceil/abs/gcd) and byte/position conversions (position-bytes /
//! byte-to-position in multibyte buffers, string-bytes/width, char-equal
//! multibyte case-fold, set-buffer-multibyte toggle).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn buffer_byte_positions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "héllo")
  (list (point-max) (- (position-bytes (point-max)) (position-bytes (point-min)))
        (buffer-size)))"##,
    );
}

#[test]
fn calc_eval_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'calc)
(list (calc-eval "16#FF") (calc-eval "2#1010") (calc-eval "8#777"))"##,
    );
}

#[test]
fn calc_eval_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'calc)
(list (calc-eval "2+3*4") (calc-eval "(2+3)*4") (calc-eval "2^3^2")
      (calc-eval "100!") (calc-eval "lcm(12,18)"))"##,
    );
}

#[test]
fn calc_eval_frac() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'calc)
(let ((calc-prefer-frac t))
  (list (calc-eval "1:3 + 1:6") (calc-eval "2:4")))"##,
    );
}

#[test]
fn calc_eval_round_funcs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'calc)
(list (calc-eval "round(3.7)") (calc-eval "floor(3.7)") (calc-eval "ceil(3.2)")
      (calc-eval "abs(-5)") (calc-eval "gcd(48,36)"))"##,
    );
}

#[test]
fn calc_eval_trig() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'calc)
(let ((calc-angle-mode 'rad) (calc-float-format '(float 4)))
  (list (calc-eval "sin(0)") (calc-eval "cos(0)") (calc-eval "exp(0)") (calc-eval "ln(1)")))"##,
    );
}

#[test]
fn char_equal_multibyte_fold() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((case-fold-search t))
  (list (char-equal ?À ?à) (char-equal ?Δ ?δ) (char-equal ?я ?Я)
        (let ((case-fold-search nil)) (char-equal ?À ?à))))"##,
    );
}

#[test]
fn multibyte_buffer_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "abc")
  (let ((m1 (buffer-multibyte-p)))
    (set-buffer-multibyte nil)
    (let ((m2 (buffer-multibyte-p)))
      (set-buffer-multibyte t)
      (list m1 m2 (buffer-multibyte-p)))))"##,
    );
}

#[test]
fn position_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "aé日b")
  (list (position-bytes 1) (position-bytes 2) (position-bytes 3) (position-bytes 4) (position-bytes 5)
        (byte-to-position 1) (byte-to-position 4)))"##,
    );
}

#[test]
fn string_bytes_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-bytes "aé日") (string-width "aé日") (length "aé日")
        (string-bytes (string-to-unibyte (encode-coding-string "é" 'utf-8))))"##,
    );
}
