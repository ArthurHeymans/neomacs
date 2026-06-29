//! Strict combo oracle probes, batch 90: font XLFD name generation, font-face-
//! attributes conversion, and coding-system-plist introspection.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_q4_font_xlfd_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(font-xlfd-name (font-spec :family "Monospace" :weight 'normal))
"##,
        expect_test::expect![[r#""OK \"-*-Monospace-normal-*-*-*-*-*-*-*-*-*-*-*\"""#]],
    );
}

#[test]
fn div_q4_font_face_attributes_weight_canonicalization() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK (:family "Monospace" :weight regular)
    // Neomacs:   OK (:family "Monospace" :weight normal)
    // font-face-attributes canonicalizes :weight 'normal to the canonical name
    // 'regular in GNU Emacs; Neomacs returns the alias 'normal.
    crate::common::assert_oracle_parity_expect(
        r##"
(font-face-attributes (font-spec :family "Monospace" :weight 'normal))
"##,
        expect_test::expect![[r#""OK (:family \"Monospace\" :weight regular)""#]],
    );
}

#[test]
fn div_q4_coding_system_plist_introspect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (plist-get (coding-system-plist 'utf-8) :mime-charset)
      (plist-get (coding-system-plist 'utf-8) :name)
      (plist-get (coding-system-plist 'iso-8859-1) :mime-charset)
      (plist-get (coding-system-plist 'utf-8-unix) :eol-type))
"##,
        expect_test::expect![[r#""OK (utf-8 utf-8 iso-8859-1 nil)""#]],
    );
}

#[test]
fn div_q4_fontset_info_introspect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((fs (car (fontset-list))))
  (list (stringp fs)
        (condition-case err (fontset-info fs) (error (car err)))))
"##,
        expect_test::expect![[r#""OK (t error)""#]],
    );
}
