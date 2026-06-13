//! UTF-8 / multibyte *character operation* and *regex* divergence probes.
//!
//! Targets `char-width` (Neomacs uses a hardcoded fallback table vs GNU's
//! `characters.el` default `char-width-table`), case operations on non-ASCII
//! (notably ß → "SS"), `char-charset` taxonomy, and regex semantics
//! (`[:alpha:]`, `\w`, `[a-z]`) over multibyte text.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- char-width (highest-yield divergence target) ---------------------------

#[test]
fn div_utf8_char_width_ascii_and_latin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(mapcar #'char-width
        (list ?a ?A ?1 ?\s ?\t ?\n ?é ?\x100 ?\x250))
"#,
    );
}

#[test]
fn div_utf8_char_width_cjk_and_hangul() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(mapcar #'char-width
        (list ?\x3042 ?\x4e2d ?\xac00 ?\xff21 ?\xff41))
"#,
    );
}

#[test]
fn div_utf8_char_width_emoji_and_supplementary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(mapcar #'char-width
        (list ?\x1f600 ?\x1f680 ?\x10000 ?\x10300))
"#,
    );
}

#[test]
fn div_utf8_char_width_combining_and_zero_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Combining marks and zero-width characters must be width 0 in GNU.
    assert_oracle_parity(
        r#"
(mapcar #'char-width
        (list ?\x300 ?\x301 ?\x302     ; combining diacritics
              ?\x200d                    ; zero-width joiner
              ?\x200b                    ; zero-width space
              ?\xfeff                    ; BOM / ZWNBSP
              ?\xad))                    ; soft hyphen
"#,
    );
}

#[test]
fn div_utf8_char_width_control_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(mapcar #'char-width
        (list 0 7 8 27 127 128))
"#,
    );
}

// --- case operations on non-ASCII ------------------------------------------

#[test]
fn div_utf8_upcase_german_sharp_s() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // ß upcases to "SS" (two chars) in GNU; a likely divergence under simple
    // Unicode mapping.
    assert_oracle_parity(
        r#"
(list (upcase "straße") (length (upcase "straße"))
      (downcase "STRASSE")
      (upcase "groß"))
"#,
    );
}

#[test]
fn div_utf8_upcase_downcase_accented() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (upcase "café") (downcase "CAFÉ")
      (upcase "résümé") (downcase "RÉSUMÉ")
      (capitalize "héllo wörld")
      (upcase-initials "café résumé"))
"#,
    );
}

#[test]
fn div_utf8_char_upcase_downcase_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (char-upcase ?a) (char-upcase ?é) (char-upcase ?ß)
      (char-downcase ?A) (char-downcase ?É))
"#,
    );
}

#[test]
fn div_utf8_upcase_greek_and_cyrillic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (upcase "άρχή") (downcase "ΑΡΧΗ")
      (upcase "привет") (downcase "ПРИВЕТ"))
"#,
    );
}

// --- char-charset taxonomy --------------------------------------------------

#[test]
fn div_utf8_char_charset_classification() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Charset names (symbols) may differ between implementations.
    assert_oracle_parity(
        r#"
(list (char-charset ?a)
      (char-charset ?é)
      (char-charset ?\x100)
      (char-charset ?\x3042)
      (char-charset ?\x4e2d)
      (char-charset (unibyte-char-to-multibyte 200)))
"#,
    );
}

#[test]
fn div_utf8_charsetp_and_encoding_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (charset-p 'ascii) (charset-p 'unicode)
      (charset-p 'eight-bit)
      (encode-char ?a 'ascii)
      (encode-char ?é 'unicode)
      (decode-char 'unicode #xe9))
"#,
    );
}

// --- regex semantics over multibyte ----------------------------------------

#[test]
fn div_utf8_regex_ascii_class_vs_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(progn
  (string-match "[a-z]+" "héllo")
  (list (match-beginning 0) (match-end 0)))
"#,
    );
}

#[test]
fn div_utf8_regex_posix_alpha_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(progn
  (string-match "[[:alpha:]]+" "héllo wörld")
  (list (match-beginning 0) (match-end 0)))
"#,
    );
}

#[test]
fn div_utf8_regex_word_constituent_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // \w word-constituent includes non-ASCII letters under multibyte syntax.
    assert_oracle_parity(
        r#"
(progn
  (string-match "\\w+" "héllo_café")
  (list (match-beginning 0) (match-end 0)))
"#,
    );
}

#[test]
fn div_utf8_regex_match_multibyte_literal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (string-match "café" "le café est prêt")
      (progn (string-match "café" "le café est prêt") (match-end 0))
      (string-match "[éèê]" "rêve")
      (string-match "世界" "你好世界"))
"#,
    );
}

#[test]
fn div_utf8_regexp_quote_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (regexp-quote "a.b+é?c")
      (regexp-quote "世界")
      (length (regexp-quote "café+")))
"#,
    );
}

#[test]
fn div_utf8_skip_chars_forward_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abcéxyz")
  (skip-chars-forward "a-z")
  (point))
"#,
    );
}

#[test]
fn div_utf8_skip_chars_forward_nonascii_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café-x")
  (skip-chars-forward "a-fé")
  (point))
"#,
    );
}
