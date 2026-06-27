//! Strict combo oracle probes, batch 10: extended # read syntax, number
//! parsing edges, bignum/negative bitwise ops, extended format-time flags,
//! 1-arg vs 2-arg floor/ceiling/round, min/max/abs over mixed types, and
//! sxhash determinism.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_e5_read_hash_syntax_more() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (read "#s(foo 1 2 3)")
      (read "#&3\"abc\"")
      (type-of (read "#s(foo 1 2 3)"))
      (aref (read "#s(foo 1 2 3)") 0)
      (condition-case err (read "#?x") (invalid-read-syntax (car err)))
      (condition-case err (read "#0a") (invalid-read-syntax (car err)))
      (condition-case err (read "#z1") (invalid-read-syntax (car err))))
"##,
    );
}

#[test]
fn div_e5_number_parsing_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-to-number "0x1f")
      (string-to-number "1_000")
      (string-to-number "+5")
      (string-to-number "-0")
      (string-to-number ".5")
      (string-to-number "1e1000")
      (string-to-number "  42  ")
      (string-to-number "3.14abc")
      (string-to-number "")
      (string-to-number "inf")
      (string-to-number "0b101"))
"##,
    );
}

#[test]
fn div_e5_bitwise_bignum_negatives() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (logand -1 255)
      (logior 5 3)
      (logxor 5 3)
      (logand (expt 2 64) (1- (expt 2 64)))
      (lognot 0)
      (lognot 5)
      (logand -1)
      (logior)
      (ash -8 -1)
      (ash -1 100)
      (logcount (lognot 0)))
"##,
    );
}

#[test]
fn div_e5_format_time_zone_flags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t0 (encode-time 0 0 12 15 6 2025 0)))
  (list (format-time-string "%z" t0 0)
        (format-time-string "%N" t0 0)
        (format-time-string "%s" t0 0)
        (format-time-string "%V" t0 0)
        (format-time-string "%G" t0 0)))
"##,
    );
}

#[test]
fn div_e5_format_time_colon_zone_modifier() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs expands the ":" modifier on %z: %:z -> "+00:00",
    // %::z -> "+00:00:00", %:::z -> "+00".  Neomacs does not recognize the
    // ":" modifier and emits the spec literally.
    assert_oracle_parity(
        r##"
(let ((t0 (encode-time 0 0 12 15 6 2025 0)))
  (list (format-time-string "%:z" t0 0)
        (format-time-string "%::z" t0 0)
        (format-time-string "%:::z" t0 0)))
"##,
    );
}

#[test]
fn div_e5_floor_ceiling_1arg_and_2arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (floor 3.7)
      (ceiling 3.2)
      (round 2.5)
      (truncate -3.7)
      (floor 3.7 2)
      (ceiling 3.2 2)
      (floor 7 3)
      (round -2.5)
      (floor most-positive-fixnum 3))
"##,
    );
}

#[test]
fn div_e5_min_max_abs_mixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (max 1 2 3)
      (min 3 1 2)
      (max -1 -2 -3)
      (abs -5)
      (abs -5.5)
      (max 1 2.0 3)
      (min 1 1.0)
      (apply #'max '(1 2 3 4))
      (max (expt 2 40) (expt 2 39))))
"##,
    );
}

#[test]
fn div_e5_sxhash_determinism() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (sxhash "abc")
      (sxhash "abc")
      (sxhash-eq 'sym)
      (sxhash-eq 'sym)
      (sxhash '(1 2 3))
      (sxhash [1 2 3])
      (integerp (sxhash 42))
      (integerp (sxhash-eq 42)))
"##,
    );
}
