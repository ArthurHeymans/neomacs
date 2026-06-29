//! Strict combo oracle probes, batch 69: executable-set-magic (shebang
//! insertion), fringe-bitmap introspection, iso-transl (ISO8859 transliteration),
//! and hideshow (hs-minor-mode code folding).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::assert_oracle_parity_with_load;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_o3_executable_set_magic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "line1\nline2\n")
  (executable-set-magic "sh")
  (buffer-string))
"##,
        expect_test::expect![[
            r##""OK \"#!/nix/store/i27rhb3nr65rkrwz36bchkwmav6ggsmn-bash-5.3p9/bin/sh\nline1\nline2\n\"""##
        ]],
    );
}

#[test]
fn div_o3_fringe_bitmap_introspection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (fringe-bitmap-p 'left-triangle)
      (fringe-bitmap-p 'right-arrow)
      (fringe-bitmap-p 'nonexistent-bitmap)
      (> (length (fringe-bitmaps)) 5)
      (memq 'left-triangle (fringe-bitmaps)))
"##,
        expect_test::expect![[r#""ERR (void-function fringe-bitmaps)""#]],
    );
}

#[test]
fn div_o3_iso_transl_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_with_load_expect(
        r##"
(list (iso-transl-decode "A`")
      (iso-transl-decode "E'")
      (iso-transl-decode "u:")
      (iso-transl-decode "n~")
      (length iso-transl-esc-map))
"##,
        &["international/iso-transl.el"],
        expect_test::expect![[r#""ERR (void-function iso-transl-decode)""#]],
    );
}

#[test]
fn div_o3_hideshow_minor_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_with_load_expect(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(defun foo ()\n  (let ((x 1))\n    body))\n")
  (hs-minor-mode 1)
  (goto-char (point-min))
  (hs-hide-block)
  (list (hs-already-hidden-p)
        (get-text-property 2 'invisible)
        (hs-show-block)
        (hs-already-hidden-p)))
"##,
        &["progmodes/hideshow.el"],
        expect_test::expect![[r#""OK (t nil nil nil)""#]],
    );
}
