//! Strict combo oracle probes, batch 62: Unicode case conversion — downcase/
//! upcase/capitalize/upcase-initials of Latin, Greek, Cyrillic, German
//! sharp-s, and case-table operations. Case folding/tables are complex and
//! under-tested (only case-fold-search was touched before).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_n2_case_ascii_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (downcase "HELLO WORLD")
      (upcase "hello world")
      (capitalize "hello world")
      (upcase-initials "hello world foo")
      (upcase-initials-region 1 5))
"##,
    );
}

#[test]
fn div_n2_case_unicode_latin_greek() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (downcase "ÉÀÇÜ")
      (upcase "éàçü")
      (capitalize "éàçü ñoño")
      (downcase "ΑΒΓΔΕ")
      (upcase "αβγδε")
      (capitalize "αβγ αβγ"))
"##,
    );
}

#[test]
fn div_n2_case_cyrillic_and_sharp_s() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (downcase "ЖЕНЯ")
      (upcase "женя")
      (capitalize "женя москва")
      (upcase "ßstraße")
      (downcase "STRASSE")))
"##,
    );
}

#[test]
fn div_n2_case_region_and_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-temp-buffer
        (insert "Hello World")
        (upcase-region 1 6)
        (buffer-string))
      (with-temp-buffer
        (insert "Hello World")
        (capitalize-region 1 11)
        (buffer-string))
      (with-temp-buffer
        (insert "HELLO")
        (downcase-region 1 5)
        (buffer-string)))
"##,
    );
}

#[test]
fn div_n2_case_table_introspect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (char-equal ?a (downcase ?A))
      (eq (upcase ?a) ?A)
      (char-equal ?é (downcase ?É))
      (eq (upcase ?é) ?É)
      (upcase-word 0)
      (capitalize-region 1 1)))
"##,
    );
}
