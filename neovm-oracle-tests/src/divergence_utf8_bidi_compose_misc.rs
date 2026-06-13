//! UTF-8 / multibyte *bidi, composition & misc* divergence probes.
//!
//! Probes the Unicode bidi-mirroring table (`bidi-mirror-char`), `buffer-hash`
//! over multibyte text, multibyte symbol names (`intern`/`symbol-name`),
//! `translate-region` with a char-table, `find-composition`, and text-property
//! run computation over multibyte regions.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- bidi-mirror-char (Unicode Bidi_Mirroring table) ------------------------

#[test]
fn div_utf8_bidi_mirror_char_brackets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (bidi-mirror-char ?\()
      (bidi-mirror-char ?\))
      (bidi-mirror-char ?<)
      (bidi-mirror-char ?>)
      (bidi-mirror-char ?\[)
      (bidi-mirror-char ?\])
      (bidi-mirror-char ?\x3008)   ; ⟨
      (bidi-mirror-char ?\x3009)   ; ⟩
      (bidi-mirror-char ?\x2208)   ; ∈
      (bidi-mirror-char ?a))       ; non-mirroring -> nil
"#,
    );
}

// --- buffer-hash over multibyte ---------------------------------------------

#[test]
fn div_utf8_buffer_hash_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (with-temp-buffer (insert "café世界") (buffer-hash))
      (with-temp-buffer (insert (decode-coding-string (unibyte-string 200) 'utf-8))
        (buffer-hash))
      (with-temp-buffer (insert "aéb") (buffer-hash)))
"#,
    );
}

// --- multibyte symbol names -------------------------------------------------

#[test]
fn div_utf8_intern_multibyte_symbol_names() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s1 (intern "café"))
      (s2 (intern "世界")))
  (list (symbol-name s1)
        (symbol-name s2)
        (eq s1 (intern "café"))
        (eq s2 (intern "世界"))
        (intern-soft "café")
        (length (symbol-name s2))))
"#,
    );
}

#[test]
fn div_utf8_intern_multibyte_symbol_identity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((ob (obarray)))
  (let ((a (intern "λ-table" ob))
        (b (intern "λ-table" ob)))
    (list (eq a b) (symbol-name a)
          (eq (intern-soft "λ-table" ob) a))))
"#,
    );
}

// --- translate-region with a char-table -------------------------------------

#[test]
fn div_utf8_translate_region_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((ct (make-char-table 'translation-table)))
  (aset ct ?a ?A)
  (aset ct ?é ?É)
  (aset ct ?\x3042 ?\x3044)
  (with-temp-buffer
    (insert "caféあ")
    (translate-region (point-min) (point-max) ct)
    (list (buffer-string) (point-max) (append (buffer-string) nil))))
"#,
    );
}

// --- find-composition -------------------------------------------------------

#[test]
fn div_utf8_find_composition_explicit_compose() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(condition-case err
    (with-temp-buffer
      (insert "abc")
      (compose-region 1 3 "")
      (find-composition 1 nil nil t))
  (error (cons (car err) 'errored)))
"#,
    );
}

// --- text-property runs over multibyte --------------------------------------

#[test]
fn div_utf8_text_property_runs_over_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café世界x")
  (put-text-property 1 3 'face 'bold)
  (put-text-property 4 6 'face 'italic)
  (list (next-single-property-change 1 'face)
        (next-single-property-change 3 'face)
        (text-property-any 1 8 'face 'italic)
        (next-property-change 1)))
"#,
    );
}

// --- emoji ZWJ sequence accounting ------------------------------------------

#[test]
fn div_utf8_emoji_zwj_sequence_accounting() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "👨‍👩‍👧")
  (list (length (buffer-string))
        (string-bytes (buffer-string))
        (point-max)
        (append (buffer-string) nil)))
"#,
    );
}
