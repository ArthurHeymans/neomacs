//! UTF-8 / multibyte *charset-conversion deep* divergence probes.
//!
//! Targets the three-way distinction between interpreting raw bytes as
//! (`string-make-multibyte` vs `string-as-multibyte` vs `decode-coding-string`),
//! legacy iso-2022 charset construction (`make-char`), `ucs-normalize`
//! NFC/NFD/NFKC, and charset dimension tables.  `string-make-multibyte` of a
//! valid UTF-8 byte sequence is a canonical UTF-8-internal divergence: GNU
//! treats each byte as a raw eight-bit char and does NOT decode, whereas a
//! UTF-8-internal reimpl tends to decode the sequence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- string-make-multibyte must NOT decode UTF-8 ----------------------------

#[test]
fn div_utf8_string_make_multibyte_utf8_bytes_not_decoded() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Bytes 195 169 are UTF-8 for é. string-make-multibyte must NOT decode
    // them; it must produce two distinct eight-bit chars.
    assert_oracle_parity(
        r#"
(let ((m (string-make-multibyte (unibyte-string 195 169))))
  (list (length m) (string-bytes m) (append m nil)
        (multibyte-string-p m)))
"#,
    );
}

#[test]
fn div_utf8_make_vs_as_vs_decode_three_way() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // The same bytes interpreted three different ways must give three
    // distinct results in GNU.
    assert_oracle_parity(
        r#"
(let ((bytes (unibyte-string 195 169 226 130 172)))   ; é, € as UTF-8
  (list (length (string-make-multibyte bytes))
        (length (string-as-multibyte bytes))
        (length (decode-coding-string bytes 'utf-8))
        (append (decode-coding-string bytes 'utf-8) nil)
        (append (string-make-multibyte bytes) nil)))
"#,
    );
}

#[test]
fn div_utf8_string_make_multibyte_each_byte_is_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((m (string-make-multibyte (unibyte-string 240 159 152 128))))  ; emoji UTF-8
  (list (length m) (append m nil)
        (mapcar #'char-charset (append m nil))))
"#,
    );
}

// --- legacy iso-2022 charset construction -----------------------------------

#[test]
fn div_utf8_make_char_legacy_iso2022_charsets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (condition-case err (make-char 'japanese-jisx0208 36 34) (error (cons 'err (car err))))
      (condition-case err (make-char 'chinese-gb2312 48 48) (error (cons 'err (car err))))
      (condition-case err (make-char 'korean-ksc5601 33 33) (error (cons 'err (car err))))
      (condition-case err (make-char 'latin-iso8859-1 41) (error (cons 'err (car err)))))
"#,
    );
}

#[test]
fn div_utf8_make_char_legacy_charset_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(condition-case err
    (let ((c (make-char 'japanese-jisx0208 36 34)))
      (list c
            (char-charset c)
            (encode-char c 'japanese-jisx0208)))
  (error (cons 'err (car err))))
"#,
    );
}

// --- insert raw bytes into a multibyte buffer -------------------------------

#[test]
fn div_utf8_insert_unibyte_into_multibyte_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "a")
  (insert (unibyte-string 200))
  (list (point-max)
        (multibyte-string-p (buffer-string))
        (append (buffer-string) nil)))
"#,
    );
}

// --- ucs-normalize NFC / NFD / NFKC -----------------------------------------

#[test]
fn div_utf8_ucs_normalize_compose_decompose() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (ucs-normalize-string "café" 'NFC)
      (ucs-normalize-string "café" 'NFD)
      (length (ucs-normalize-string "café" 'NFD))
      (append (ucs-normalize-string "café" 'NFD) nil)
      (ucs-normalize-string (string #xFB01) 'NFKC)
      (length (ucs-normalize-string (string #xFB01) 'NFKC))
      (equal (ucs-normalize-string "café" 'NFC)
             (ucs-normalize-string (concat "cafe" (string #x301)) 'NFC)))
"#,
    );
}

#[test]
fn div_utf8_ucs_normalize_korean_hangul() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Hangul has algorithmic (not table) composition in NFC.
    assert_oracle_parity(
        r#"
(let* ((composed (string #xAC00))                 ; 가
       (decomposed (ucs-normalize-string composed 'NFD)))
  (list (length composed) (length decomposed)
        (append decomposed nil)
        (ucs-normalize-string decomposed 'NFC)))
"#,
    );
}

// --- charset dimension tables -----------------------------------------------

#[test]
fn div_utf8_charset_dimensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (charset-dimension 'ascii)
      (charset-dimension 'latin-iso8859-1)
      (charset-dimension 'japanese-jisx0208)
      (charset-dimension 'unicode)
      (charset-dimension 'eight-bit)
      (length (charset-list)))
"#,
    );
}
