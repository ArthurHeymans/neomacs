//! `format "%c"` / `%s` / `princ` mis-encode Private-Use-Area (and similar)
//! chars to eight-bit sentinels.
//!
//! Confirmed: `(aref (format "%c" #xe0a0) 0)` returns #x3fffa0 (= eight-bit
//! sentinel of byte 0xa0) in Neomacs, vs #xe0a0 in GNU. The string-constructor
//! paths (`char-to-string`, `concat`, `make-string`, `insert`) handle the same
//! char correctly — only the format/princ print paths break.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_fp_format_c_pua_e0a0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(aref (format "%c" #xe0a0) 0)"##);
}

#[test]
fn div_fp_format_s_of_glyph_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(let ((s (string #xe0a0))) (aref (format "%s" s) 0))"##);
}

#[test]
fn div_fp_princ_glyph_then_char_after() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer (princ (string #xe0a0) (current-buffer)) (char-after 1))
"##,
    );
}

#[test]
fn div_fp_controls_char_to_string_etc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // These paths handle U+E0A0 correctly (both engines) — control.
    assert_oracle_parity(
        r##"
(list (aref (char-to-string #xe0a0) 0)
      (aref (concat (string #xe0a0)) 0)
      (aref (make-string 1 #xe0a0) 0)
      (with-temp-buffer (insert #xe0a0) (char-after 1)))
"##,
    );
}

#[test]
fn div_fp_format_c_range_probe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Characterize which codepoints format %c mis-encodes.
    assert_oracle_parity(
        r##"
(mapcar (lambda (cp) (aref (format "%c" cp) 0))
        (list #x100 #x2500 #x3042 #xe000 #xe0a0 #xf8ff #xfffd #x1f600 #x10000))
"##,
    );
}

#[test]
fn div_fp_princ_range_probe_char_after() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (cp)
          (with-temp-buffer
            (princ (string cp) (current-buffer))
            (char-after 1)))
        (list #xe000 #xe0a0 #xf8ff #x10000 #x1f600))
"##,
    );
}

#[test]
fn div_fp_format_c_multiple_pua() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (aref (format "%c" #xe000) 0)
      (aref (format "%c" #xe0a0) 0)
      (aref (format "%c" #xf8ff) 0))
"##,
    );
}

#[test]
fn div_fp_format_c_supplementary_plane() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (aref (format "%c" #x1f600) 0)
      (aref (format "%c" #x10000) 0)
      (aref (format "%c" #x10ffff) 0))
"##,
    );
}

#[test]
fn div_fp_prin1_to_string_of_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // prin1 (with quotes) vs princ (without) — both print paths.
    assert_oracle_parity(
        r##"
(list (aref (prin1-to-string (string #xe0a0)) 1)
      (length (princ-to-string (string #xe0a0))))
"##,
    );
}
