//! Per-char char-to-string matrix over the sentinel-collision PUA ranges.
//!
//! Neomacs internal raw-byte sentinels (U+E080-E0FF) and unibyte sentinels
//! (U+E300-E3FF) collide with real Private Use Area chars; char-to-string /
//! format "%c" / princ corrupt those PUA chars into eight-bit sentinels.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_fpm_E080() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe080) 0)");
}

#[test]
fn div_fpm_E081() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe081) 0)");
}

#[test]
fn div_fpm_E082() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe082) 0)");
}

#[test]
fn div_fpm_E083() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe083) 0)");
}

#[test]
fn div_fpm_E084() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe084) 0)");
}

#[test]
fn div_fpm_E085() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe085) 0)");
}

#[test]
fn div_fpm_E086() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe086) 0)");
}

#[test]
fn div_fpm_E087() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe087) 0)");
}

#[test]
fn div_fpm_E088() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe088) 0)");
}

#[test]
fn div_fpm_E089() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe089) 0)");
}

#[test]
fn div_fpm_E08A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe08a) 0)");
}

#[test]
fn div_fpm_E08B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe08b) 0)");
}

#[test]
fn div_fpm_E08C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe08c) 0)");
}

#[test]
fn div_fpm_E08D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe08d) 0)");
}

#[test]
fn div_fpm_E08E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe08e) 0)");
}

#[test]
fn div_fpm_E08F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe08f) 0)");
}

#[test]
fn div_fpm_E090() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe090) 0)");
}

#[test]
fn div_fpm_E091() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe091) 0)");
}

#[test]
fn div_fpm_E092() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe092) 0)");
}

#[test]
fn div_fpm_E093() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe093) 0)");
}

#[test]
fn div_fpm_E094() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe094) 0)");
}

#[test]
fn div_fpm_E095() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe095) 0)");
}

#[test]
fn div_fpm_E096() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe096) 0)");
}

#[test]
fn div_fpm_E097() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe097) 0)");
}

#[test]
fn div_fpm_E098() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe098) 0)");
}

#[test]
fn div_fpm_E099() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe099) 0)");
}

#[test]
fn div_fpm_E09A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe09a) 0)");
}

#[test]
fn div_fpm_E09B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe09b) 0)");
}

#[test]
fn div_fpm_E09C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe09c) 0)");
}

#[test]
fn div_fpm_E09D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe09d) 0)");
}

#[test]
fn div_fpm_E09E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe09e) 0)");
}

#[test]
fn div_fpm_E09F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe09f) 0)");
}

#[test]
fn div_fpm_E0A0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a0) 0)");
}

#[test]
fn div_fpm_E0A1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a1) 0)");
}

#[test]
fn div_fpm_E0A2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a2) 0)");
}

#[test]
fn div_fpm_E0A3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a3) 0)");
}

#[test]
fn div_fpm_E0A4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a4) 0)");
}

#[test]
fn div_fpm_E0A5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a5) 0)");
}

#[test]
fn div_fpm_E0A6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a6) 0)");
}

#[test]
fn div_fpm_E0A7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a7) 0)");
}

#[test]
fn div_fpm_E0A8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a8) 0)");
}

#[test]
fn div_fpm_E0A9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0a9) 0)");
}

#[test]
fn div_fpm_E0AA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0aa) 0)");
}

#[test]
fn div_fpm_E0AB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ab) 0)");
}

#[test]
fn div_fpm_E0AC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ac) 0)");
}

#[test]
fn div_fpm_E0AD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ad) 0)");
}

#[test]
fn div_fpm_E0AE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ae) 0)");
}

#[test]
fn div_fpm_E0AF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0af) 0)");
}

#[test]
fn div_fpm_E0B0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b0) 0)");
}

#[test]
fn div_fpm_E0B1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b1) 0)");
}

#[test]
fn div_fpm_E0B2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b2) 0)");
}

#[test]
fn div_fpm_E0B3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b3) 0)");
}

#[test]
fn div_fpm_E0B4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b4) 0)");
}

#[test]
fn div_fpm_E0B5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b5) 0)");
}

#[test]
fn div_fpm_E0B6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b6) 0)");
}

#[test]
fn div_fpm_E0B7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b7) 0)");
}

#[test]
fn div_fpm_E0B8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b8) 0)");
}

#[test]
fn div_fpm_E0B9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0b9) 0)");
}

#[test]
fn div_fpm_E0BA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ba) 0)");
}

#[test]
fn div_fpm_E0BB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0bb) 0)");
}

#[test]
fn div_fpm_E0BC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0bc) 0)");
}

#[test]
fn div_fpm_E0BD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0bd) 0)");
}

#[test]
fn div_fpm_E0BE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0be) 0)");
}

#[test]
fn div_fpm_E0BF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0bf) 0)");
}

#[test]
fn div_fpm_E0C0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c0) 0)");
}

#[test]
fn div_fpm_E0C1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c1) 0)");
}

#[test]
fn div_fpm_E0C2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c2) 0)");
}

#[test]
fn div_fpm_E0C3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c3) 0)");
}

#[test]
fn div_fpm_E0C4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c4) 0)");
}

#[test]
fn div_fpm_E0C5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c5) 0)");
}

#[test]
fn div_fpm_E0C6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c6) 0)");
}

#[test]
fn div_fpm_E0C7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c7) 0)");
}

#[test]
fn div_fpm_E0C8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c8) 0)");
}

#[test]
fn div_fpm_E0C9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0c9) 0)");
}

#[test]
fn div_fpm_E0CA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ca) 0)");
}

#[test]
fn div_fpm_E0CB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0cb) 0)");
}

#[test]
fn div_fpm_E0CC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0cc) 0)");
}

#[test]
fn div_fpm_E0CD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0cd) 0)");
}

#[test]
fn div_fpm_E0CE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ce) 0)");
}

#[test]
fn div_fpm_E0CF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0cf) 0)");
}

#[test]
fn div_fpm_E0D0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d0) 0)");
}

#[test]
fn div_fpm_E0D1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d1) 0)");
}

#[test]
fn div_fpm_E0D2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d2) 0)");
}

#[test]
fn div_fpm_E0D3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d3) 0)");
}

#[test]
fn div_fpm_E0D4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d4) 0)");
}

#[test]
fn div_fpm_E0D5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d5) 0)");
}

#[test]
fn div_fpm_E0D6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d6) 0)");
}

#[test]
fn div_fpm_E0D7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d7) 0)");
}

#[test]
fn div_fpm_E0D8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d8) 0)");
}

#[test]
fn div_fpm_E0D9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0d9) 0)");
}

#[test]
fn div_fpm_E0DA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0da) 0)");
}

#[test]
fn div_fpm_E0DB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0db) 0)");
}

#[test]
fn div_fpm_E0DC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0dc) 0)");
}

#[test]
fn div_fpm_E0DD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0dd) 0)");
}

#[test]
fn div_fpm_E0DE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0de) 0)");
}

#[test]
fn div_fpm_E0DF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0df) 0)");
}

#[test]
fn div_fpm_E0E0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e0) 0)");
}

#[test]
fn div_fpm_E0E1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e1) 0)");
}

#[test]
fn div_fpm_E0E2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e2) 0)");
}

#[test]
fn div_fpm_E0E3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e3) 0)");
}

#[test]
fn div_fpm_E0E4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e4) 0)");
}

#[test]
fn div_fpm_E0E5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e5) 0)");
}

#[test]
fn div_fpm_E0E6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e6) 0)");
}

#[test]
fn div_fpm_E0E7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e7) 0)");
}

#[test]
fn div_fpm_E0E8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e8) 0)");
}

#[test]
fn div_fpm_E0E9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0e9) 0)");
}

#[test]
fn div_fpm_E0EA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ea) 0)");
}

#[test]
fn div_fpm_E0EB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0eb) 0)");
}

#[test]
fn div_fpm_E0EC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ec) 0)");
}

#[test]
fn div_fpm_E0ED() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ed) 0)");
}

#[test]
fn div_fpm_E0EE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ee) 0)");
}

#[test]
fn div_fpm_E0EF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ef) 0)");
}

#[test]
fn div_fpm_E0F0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f0) 0)");
}

#[test]
fn div_fpm_E0F1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f1) 0)");
}

#[test]
fn div_fpm_E0F2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f2) 0)");
}

#[test]
fn div_fpm_E0F3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f3) 0)");
}

#[test]
fn div_fpm_E0F4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f4) 0)");
}

#[test]
fn div_fpm_E0F5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f5) 0)");
}

#[test]
fn div_fpm_E0F6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f6) 0)");
}

#[test]
fn div_fpm_E0F7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f7) 0)");
}

#[test]
fn div_fpm_E0F8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f8) 0)");
}

#[test]
fn div_fpm_E0F9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0f9) 0)");
}

#[test]
fn div_fpm_E0FA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0fa) 0)");
}

#[test]
fn div_fpm_E0FB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0fb) 0)");
}

#[test]
fn div_fpm_E0FC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0fc) 0)");
}

#[test]
fn div_fpm_E0FD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0fd) 0)");
}

#[test]
fn div_fpm_E0FE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0fe) 0)");
}

#[test]
fn div_fpm_E0FF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe0ff) 0)");
}

#[test]
fn div_fpm_E300() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe300) 0)");
}

#[test]
fn div_fpm_E301() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe301) 0)");
}

#[test]
fn div_fpm_E302() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe302) 0)");
}

#[test]
fn div_fpm_E303() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe303) 0)");
}

#[test]
fn div_fpm_E304() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe304) 0)");
}

#[test]
fn div_fpm_E305() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe305) 0)");
}

#[test]
fn div_fpm_E306() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe306) 0)");
}

#[test]
fn div_fpm_E307() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe307) 0)");
}

#[test]
fn div_fpm_E308() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe308) 0)");
}

#[test]
fn div_fpm_E309() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe309) 0)");
}

#[test]
fn div_fpm_E30A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe30a) 0)");
}

#[test]
fn div_fpm_E30B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe30b) 0)");
}

#[test]
fn div_fpm_E30C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe30c) 0)");
}

#[test]
fn div_fpm_E30D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe30d) 0)");
}

#[test]
fn div_fpm_E30E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe30e) 0)");
}

#[test]
fn div_fpm_E30F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe30f) 0)");
}

#[test]
fn div_fpm_E310() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe310) 0)");
}

#[test]
fn div_fpm_E311() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe311) 0)");
}

#[test]
fn div_fpm_E312() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe312) 0)");
}

#[test]
fn div_fpm_E313() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe313) 0)");
}

#[test]
fn div_fpm_E314() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe314) 0)");
}

#[test]
fn div_fpm_E315() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe315) 0)");
}

#[test]
fn div_fpm_E316() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe316) 0)");
}

#[test]
fn div_fpm_E317() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe317) 0)");
}

#[test]
fn div_fpm_E318() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe318) 0)");
}

#[test]
fn div_fpm_E319() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe319) 0)");
}

#[test]
fn div_fpm_E31A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe31a) 0)");
}

#[test]
fn div_fpm_E31B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe31b) 0)");
}

#[test]
fn div_fpm_E31C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe31c) 0)");
}

#[test]
fn div_fpm_E31D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe31d) 0)");
}

#[test]
fn div_fpm_E31E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe31e) 0)");
}

#[test]
fn div_fpm_E31F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe31f) 0)");
}

#[test]
fn div_fpm_E320() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe320) 0)");
}

#[test]
fn div_fpm_E321() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe321) 0)");
}

#[test]
fn div_fpm_E322() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe322) 0)");
}

#[test]
fn div_fpm_E323() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe323) 0)");
}

#[test]
fn div_fpm_E324() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe324) 0)");
}

#[test]
fn div_fpm_E325() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe325) 0)");
}

#[test]
fn div_fpm_E326() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe326) 0)");
}

#[test]
fn div_fpm_E327() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe327) 0)");
}

#[test]
fn div_fpm_E328() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe328) 0)");
}

#[test]
fn div_fpm_E329() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe329) 0)");
}

#[test]
fn div_fpm_E32A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe32a) 0)");
}

#[test]
fn div_fpm_E32B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe32b) 0)");
}

#[test]
fn div_fpm_E32C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe32c) 0)");
}

#[test]
fn div_fpm_E32D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe32d) 0)");
}

#[test]
fn div_fpm_E32E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe32e) 0)");
}

#[test]
fn div_fpm_E32F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe32f) 0)");
}

#[test]
fn div_fpm_E330() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe330) 0)");
}

#[test]
fn div_fpm_E331() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe331) 0)");
}

#[test]
fn div_fpm_E332() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe332) 0)");
}

#[test]
fn div_fpm_E333() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe333) 0)");
}

#[test]
fn div_fpm_E334() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe334) 0)");
}

#[test]
fn div_fpm_E335() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe335) 0)");
}

#[test]
fn div_fpm_E336() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe336) 0)");
}

#[test]
fn div_fpm_E337() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe337) 0)");
}

#[test]
fn div_fpm_E338() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe338) 0)");
}

#[test]
fn div_fpm_E339() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe339) 0)");
}

#[test]
fn div_fpm_E33A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe33a) 0)");
}

#[test]
fn div_fpm_E33B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe33b) 0)");
}

#[test]
fn div_fpm_E33C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe33c) 0)");
}

#[test]
fn div_fpm_E33D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe33d) 0)");
}

#[test]
fn div_fpm_E33E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe33e) 0)");
}

#[test]
fn div_fpm_E33F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe33f) 0)");
}

#[test]
fn div_fpm_E340() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe340) 0)");
}

#[test]
fn div_fpm_E341() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe341) 0)");
}

#[test]
fn div_fpm_E342() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe342) 0)");
}

#[test]
fn div_fpm_E343() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe343) 0)");
}

#[test]
fn div_fpm_E344() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe344) 0)");
}

#[test]
fn div_fpm_E345() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe345) 0)");
}

#[test]
fn div_fpm_E346() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe346) 0)");
}

#[test]
fn div_fpm_E347() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe347) 0)");
}

#[test]
fn div_fpm_E348() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe348) 0)");
}

#[test]
fn div_fpm_E349() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe349) 0)");
}

#[test]
fn div_fpm_E34A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe34a) 0)");
}

#[test]
fn div_fpm_E34B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe34b) 0)");
}

#[test]
fn div_fpm_E34C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe34c) 0)");
}

#[test]
fn div_fpm_E34D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe34d) 0)");
}

#[test]
fn div_fpm_E34E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe34e) 0)");
}

#[test]
fn div_fpm_E34F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe34f) 0)");
}

#[test]
fn div_fpm_E350() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe350) 0)");
}

#[test]
fn div_fpm_E351() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe351) 0)");
}

#[test]
fn div_fpm_E352() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe352) 0)");
}

#[test]
fn div_fpm_E353() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe353) 0)");
}

#[test]
fn div_fpm_E354() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe354) 0)");
}

#[test]
fn div_fpm_E355() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe355) 0)");
}

#[test]
fn div_fpm_E356() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe356) 0)");
}

#[test]
fn div_fpm_E357() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe357) 0)");
}

#[test]
fn div_fpm_E358() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe358) 0)");
}

#[test]
fn div_fpm_E359() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe359) 0)");
}

#[test]
fn div_fpm_E35A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe35a) 0)");
}

#[test]
fn div_fpm_E35B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe35b) 0)");
}

#[test]
fn div_fpm_E35C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe35c) 0)");
}

#[test]
fn div_fpm_E35D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe35d) 0)");
}

#[test]
fn div_fpm_E35E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe35e) 0)");
}

#[test]
fn div_fpm_E35F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe35f) 0)");
}

#[test]
fn div_fpm_E360() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe360) 0)");
}

#[test]
fn div_fpm_E361() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe361) 0)");
}

#[test]
fn div_fpm_E362() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe362) 0)");
}

#[test]
fn div_fpm_E363() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe363) 0)");
}

#[test]
fn div_fpm_E364() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe364) 0)");
}

#[test]
fn div_fpm_E365() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe365) 0)");
}

#[test]
fn div_fpm_E366() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe366) 0)");
}

#[test]
fn div_fpm_E367() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe367) 0)");
}

#[test]
fn div_fpm_E368() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe368) 0)");
}

#[test]
fn div_fpm_E369() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe369) 0)");
}

#[test]
fn div_fpm_E36A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe36a) 0)");
}

#[test]
fn div_fpm_E36B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe36b) 0)");
}

#[test]
fn div_fpm_E36C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe36c) 0)");
}

#[test]
fn div_fpm_E36D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe36d) 0)");
}

#[test]
fn div_fpm_E36E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe36e) 0)");
}

#[test]
fn div_fpm_E36F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe36f) 0)");
}

#[test]
fn div_fpm_E370() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe370) 0)");
}

#[test]
fn div_fpm_E371() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe371) 0)");
}

#[test]
fn div_fpm_E372() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe372) 0)");
}

#[test]
fn div_fpm_E373() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe373) 0)");
}

#[test]
fn div_fpm_E374() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe374) 0)");
}

#[test]
fn div_fpm_E375() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe375) 0)");
}

#[test]
fn div_fpm_E376() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe376) 0)");
}

#[test]
fn div_fpm_E377() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe377) 0)");
}

#[test]
fn div_fpm_E378() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe378) 0)");
}

#[test]
fn div_fpm_E379() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe379) 0)");
}

#[test]
fn div_fpm_E37A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe37a) 0)");
}

#[test]
fn div_fpm_E37B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe37b) 0)");
}

#[test]
fn div_fpm_E37C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe37c) 0)");
}

#[test]
fn div_fpm_E37D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe37d) 0)");
}

#[test]
fn div_fpm_E37E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe37e) 0)");
}

#[test]
fn div_fpm_E37F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe37f) 0)");
}

#[test]
fn div_fpm_E380() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe380) 0)");
}

#[test]
fn div_fpm_E381() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe381) 0)");
}

#[test]
fn div_fpm_E382() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe382) 0)");
}

#[test]
fn div_fpm_E383() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe383) 0)");
}

#[test]
fn div_fpm_E384() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe384) 0)");
}

#[test]
fn div_fpm_E385() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe385) 0)");
}

#[test]
fn div_fpm_E386() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe386) 0)");
}

#[test]
fn div_fpm_E387() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe387) 0)");
}

#[test]
fn div_fpm_E388() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe388) 0)");
}

#[test]
fn div_fpm_E389() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe389) 0)");
}

#[test]
fn div_fpm_E38A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe38a) 0)");
}

#[test]
fn div_fpm_E38B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe38b) 0)");
}

#[test]
fn div_fpm_E38C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe38c) 0)");
}

#[test]
fn div_fpm_E38D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe38d) 0)");
}

#[test]
fn div_fpm_E38E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe38e) 0)");
}

#[test]
fn div_fpm_E38F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe38f) 0)");
}

#[test]
fn div_fpm_E390() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe390) 0)");
}

#[test]
fn div_fpm_E391() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe391) 0)");
}

#[test]
fn div_fpm_E392() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe392) 0)");
}

#[test]
fn div_fpm_E393() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe393) 0)");
}

#[test]
fn div_fpm_E394() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe394) 0)");
}

#[test]
fn div_fpm_E395() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe395) 0)");
}

#[test]
fn div_fpm_E396() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe396) 0)");
}

#[test]
fn div_fpm_E397() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe397) 0)");
}

#[test]
fn div_fpm_E398() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe398) 0)");
}

#[test]
fn div_fpm_E399() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe399) 0)");
}

#[test]
fn div_fpm_E39A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe39a) 0)");
}

#[test]
fn div_fpm_E39B() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe39b) 0)");
}

#[test]
fn div_fpm_E39C() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe39c) 0)");
}

#[test]
fn div_fpm_E39D() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe39d) 0)");
}

#[test]
fn div_fpm_E39E() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe39e) 0)");
}

#[test]
fn div_fpm_E39F() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe39f) 0)");
}

#[test]
fn div_fpm_E3A0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a0) 0)");
}

#[test]
fn div_fpm_E3A1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a1) 0)");
}

#[test]
fn div_fpm_E3A2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a2) 0)");
}

#[test]
fn div_fpm_E3A3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a3) 0)");
}

#[test]
fn div_fpm_E3A4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a4) 0)");
}

#[test]
fn div_fpm_E3A5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a5) 0)");
}

#[test]
fn div_fpm_E3A6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a6) 0)");
}

#[test]
fn div_fpm_E3A7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a7) 0)");
}

#[test]
fn div_fpm_E3A8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a8) 0)");
}

#[test]
fn div_fpm_E3A9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3a9) 0)");
}

#[test]
fn div_fpm_E3AA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3aa) 0)");
}

#[test]
fn div_fpm_E3AB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ab) 0)");
}

#[test]
fn div_fpm_E3AC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ac) 0)");
}

#[test]
fn div_fpm_E3AD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ad) 0)");
}

#[test]
fn div_fpm_E3AE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ae) 0)");
}

#[test]
fn div_fpm_E3AF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3af) 0)");
}

#[test]
fn div_fpm_E3B0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b0) 0)");
}

#[test]
fn div_fpm_E3B1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b1) 0)");
}

#[test]
fn div_fpm_E3B2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b2) 0)");
}

#[test]
fn div_fpm_E3B3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b3) 0)");
}

#[test]
fn div_fpm_E3B4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b4) 0)");
}

#[test]
fn div_fpm_E3B5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b5) 0)");
}

#[test]
fn div_fpm_E3B6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b6) 0)");
}

#[test]
fn div_fpm_E3B7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b7) 0)");
}

#[test]
fn div_fpm_E3B8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b8) 0)");
}

#[test]
fn div_fpm_E3B9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3b9) 0)");
}

#[test]
fn div_fpm_E3BA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ba) 0)");
}

#[test]
fn div_fpm_E3BB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3bb) 0)");
}

#[test]
fn div_fpm_E3BC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3bc) 0)");
}

#[test]
fn div_fpm_E3BD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3bd) 0)");
}

#[test]
fn div_fpm_E3BE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3be) 0)");
}

#[test]
fn div_fpm_E3BF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3bf) 0)");
}

#[test]
fn div_fpm_E3C0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c0) 0)");
}

#[test]
fn div_fpm_E3C1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c1) 0)");
}

#[test]
fn div_fpm_E3C2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c2) 0)");
}

#[test]
fn div_fpm_E3C3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c3) 0)");
}

#[test]
fn div_fpm_E3C4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c4) 0)");
}

#[test]
fn div_fpm_E3C5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c5) 0)");
}

#[test]
fn div_fpm_E3C6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c6) 0)");
}

#[test]
fn div_fpm_E3C7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c7) 0)");
}

#[test]
fn div_fpm_E3C8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c8) 0)");
}

#[test]
fn div_fpm_E3C9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3c9) 0)");
}

#[test]
fn div_fpm_E3CA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ca) 0)");
}

#[test]
fn div_fpm_E3CB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3cb) 0)");
}

#[test]
fn div_fpm_E3CC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3cc) 0)");
}

#[test]
fn div_fpm_E3CD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3cd) 0)");
}

#[test]
fn div_fpm_E3CE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ce) 0)");
}

#[test]
fn div_fpm_E3CF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3cf) 0)");
}

#[test]
fn div_fpm_E3D0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d0) 0)");
}

#[test]
fn div_fpm_E3D1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d1) 0)");
}

#[test]
fn div_fpm_E3D2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d2) 0)");
}

#[test]
fn div_fpm_E3D3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d3) 0)");
}

#[test]
fn div_fpm_E3D4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d4) 0)");
}

#[test]
fn div_fpm_E3D5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d5) 0)");
}

#[test]
fn div_fpm_E3D6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d6) 0)");
}

#[test]
fn div_fpm_E3D7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d7) 0)");
}

#[test]
fn div_fpm_E3D8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d8) 0)");
}

#[test]
fn div_fpm_E3D9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3d9) 0)");
}

#[test]
fn div_fpm_E3DA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3da) 0)");
}

#[test]
fn div_fpm_E3DB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3db) 0)");
}

#[test]
fn div_fpm_E3DC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3dc) 0)");
}

#[test]
fn div_fpm_E3DD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3dd) 0)");
}

#[test]
fn div_fpm_E3DE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3de) 0)");
}

#[test]
fn div_fpm_E3DF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3df) 0)");
}

#[test]
fn div_fpm_E3E0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e0) 0)");
}

#[test]
fn div_fpm_E3E1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e1) 0)");
}

#[test]
fn div_fpm_E3E2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e2) 0)");
}

#[test]
fn div_fpm_E3E3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e3) 0)");
}

#[test]
fn div_fpm_E3E4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e4) 0)");
}

#[test]
fn div_fpm_E3E5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e5) 0)");
}

#[test]
fn div_fpm_E3E6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e6) 0)");
}

#[test]
fn div_fpm_E3E7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e7) 0)");
}

#[test]
fn div_fpm_E3E8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e8) 0)");
}

#[test]
fn div_fpm_E3E9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3e9) 0)");
}

#[test]
fn div_fpm_E3EA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ea) 0)");
}

#[test]
fn div_fpm_E3EB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3eb) 0)");
}

#[test]
fn div_fpm_E3EC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ec) 0)");
}

#[test]
fn div_fpm_E3ED() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ed) 0)");
}

#[test]
fn div_fpm_E3EE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ee) 0)");
}

#[test]
fn div_fpm_E3EF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ef) 0)");
}

#[test]
fn div_fpm_E3F0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f0) 0)");
}

#[test]
fn div_fpm_E3F1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f1) 0)");
}

#[test]
fn div_fpm_E3F2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f2) 0)");
}

#[test]
fn div_fpm_E3F3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f3) 0)");
}

#[test]
fn div_fpm_E3F4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f4) 0)");
}

#[test]
fn div_fpm_E3F5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f5) 0)");
}

#[test]
fn div_fpm_E3F6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f6) 0)");
}

#[test]
fn div_fpm_E3F7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f7) 0)");
}

#[test]
fn div_fpm_E3F8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f8) 0)");
}

#[test]
fn div_fpm_E3F9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3f9) 0)");
}

#[test]
fn div_fpm_E3FA() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3fa) 0)");
}

#[test]
fn div_fpm_E3FB() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3fb) 0)");
}

#[test]
fn div_fpm_E3FC() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3fc) 0)");
}

#[test]
fn div_fpm_E3FD() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3fd) 0)");
}

#[test]
fn div_fpm_E3FE() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3fe) 0)");
}

#[test]
fn div_fpm_E3FF() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(aref (char-to-string #xe3ff) 0)");
}
