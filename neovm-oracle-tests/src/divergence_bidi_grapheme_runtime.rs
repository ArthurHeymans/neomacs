//! Bidi + grapheme/normalization parity: bidi-string-mark-left-to-right,
//! Hangul NFD/NFC (algorithmic jamo), reverse/string-reverse, char mirroring,
//! bidi-class properties, compose-region, Arabic normalization; plus the
//! string-glyph-split grapheme-cluster and bidi-paragraph-direction divergences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn bd_bidi_class_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?A 'bidi-class)
        (get-char-code-property ?ا 'bidi-class)
        (get-char-code-property ?1 'bidi-class)
        (get-char-code-property ?\s 'bidi-class))"##,
    );
}

#[test]
fn bd_bidi_mark_ltr() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s "abcשלום"))
  (list (stringp (bidi-string-mark-left-to-right s))
        (>= (length (bidi-string-mark-left-to-right s)) (length s))))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: current-bidi-paragraph-direction returns left-to-right for RTL (Hebrew/Arabic) text where GNU detects right-to-left."]
fn divergence_bidi_paragraph_direction_rtl() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello world")
  (list (current-bidi-paragraph-direction)
        (progn (erase-buffer) (insert "שלום") (current-bidi-paragraph-direction))))"##,
    );
}

#[test]
fn bd_char_mirror() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (get-char-code-property ?\( 'mirroring)
        (get-char-code-property ?\[ 'mirroring)
        (get-char-code-property ?a 'mirroring))"##,
    );
}

#[test]
fn bd_compose_region_auto() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert (string ?e #x0301))
  (list (buffer-size) (char-after 1) (char-after 2)))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: string-glyph-split splits a base char + combining mark into separate glyphs (e + U+0301 => 2) instead of one grapheme cluster (GNU => 1)."]
fn divergence_string_glyph_split_grapheme() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (length (string-glyph-split (string ?e #x0301)))
        (length (string-glyph-split "abc"))
        (string-glyph-split "a"))"##,
    );
}

#[test]
fn bd_hangul_nfd() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'ucs-normalize)
(let ((s "한"))
  (list (length s) (length (ucs-normalize-NFD-string s))
        (length (ucs-normalize-NFC-string (ucs-normalize-NFD-string s)))))"##,
    );
}

#[test]
fn bd_nfc_hangul_compose() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'ucs-normalize)
(let ((jamo (string #x1112 #x1161 #x11ab)))
  (list (length jamo) (length (ucs-normalize-NFC-string jamo))
        (string= (ucs-normalize-NFC-string jamo) "한")))"##,
    );
}

#[test]
fn bd_normalize_arabic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'ucs-normalize)
(let ((s "السلام"))
  (list (string= (ucs-normalize-NFC-string s) s)
        (stringp (ucs-normalize-NFD-string s))))"##,
    );
}

#[test]
fn bd_string_reverse_bidi() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (reverse "abc") (reverse "héllo")
        (string-reverse "abcd") (reverse [1 2 3]))"##,
    );
}
