//! Strict combo oracle probes, batch 90: font XLFD name generation, font-face-
//! attributes conversion, and coding-system-plist introspection.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_q4_font_xlfd_and_face_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((fs (font-spec :family "Monospace" :weight 'normal)))
  (list (condition-case err (font-xlfd-name fs) (error (cons 'err (car err))))
        (condition-case err (font-face-attributes fs) (error (cons 'err (car err))))))
"##,
    );
}

#[test]
fn div_q4_coding_system_plist_introspect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (plist-get (coding-system-plist 'utf-8) :mime-charset)
      (plist-get (coding-system-plist 'utf-8) :name)
      (plist-get (coding-system-plist 'iso-8859-1) :mime-charset)
      (plist-get (coding-system-plist 'utf-8-unix) :eol-type))
"##,
    );
}

#[test]
fn div_q4_fontset_info_introspect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((fs (car (fontset-list))))
  (list (stringp fs)
        (condition-case err (fontset-info fs) (error (car err)))))
"##,
    );
}
