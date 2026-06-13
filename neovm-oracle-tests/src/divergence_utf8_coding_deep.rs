//! UTF-8 / multibyte *coding* deep probes — BOM/signature, UTF-16, eight-bit
//! byte width, and charset classification.
//!
//! Follow-up to `divergence_utf8_coding.rs`, expanding the three confirmed
//! divergence themes: (a) `-with-signature` BOM handling, (b) internal byte
//! width of eight-bit raw-byte characters, (c) eight-bit charset
//! classification.  Also probes UTF-16 endianness/BOM which is structurally
//! similar and a likely additional divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- coding-system existence / aliasing -------------------------------------

#[test]
fn div_utf8_coding_system_p_signature_and_utf16() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (coding-system-p 'utf-8)
      (coding-system-p 'utf-8-with-signature)
      (coding-system-p 'utf-8-with-signature-unix)
      (coding-system-p 'utf-16)
      (coding-system-p 'utf-16le)
      (coding-system-p 'utf-16be)
      (coding-system-p 'utf-16le-with-signature))
"#,
    );
}

// --- BOM / signature on encode ----------------------------------------------

#[test]
fn div_utf8_encode_signature_byte_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (length (encode-coding-string "abc" 'utf-8))
      (length (encode-coding-string "abc" 'utf-8-with-signature))
      (append (encode-coding-string "abc" 'utf-8-with-signature) nil)
      (string-bytes (encode-coding-string "abc" 'utf-8-with-signature)))
"#,
    );
}

#[test]
fn div_utf8_encode_signature_multibyte_payload() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((e (encode-coding-string "café" 'utf-8-with-signature)))
  (list (length e) (string-bytes e) (append e nil)))
"#,
    );
}

#[test]
fn div_utf8_decode_signature_strips_bom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((d (decode-coding-string (unibyte-string 239 187 191 97 98 99) 'utf-8-with-signature)))
  (list (length d) (string-bytes d) (append d nil)))
"#,
    );
}

// --- UTF-16 endianness / BOM ------------------------------------------------

#[test]
fn div_utf8_utf16_be_with_bom_encode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((e (encode-coding-string "AB" 'utf-16)))
  (list (length e) (string-bytes e) (append e nil)))
"#,
    );
}

#[test]
fn div_utf8_utf16_le_no_bom_encode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((e (encode-coding-string "AB" 'utf-16le)))
  (list (length e) (string-bytes e) (append e nil)))
"#,
    );
}

#[test]
fn div_utf8_utf16_be_no_bom_encode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((e (encode-coding-string "AB" 'utf-16be)))
  (list (length e) (string-bytes e) (append e nil)))
"#,
    );
}

#[test]
fn div_utf8_utf16_decode_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((bytes (unibyte-string 254 255 0 65 0 66)))
  (list (decode-coding-string bytes 'utf-16)
        (length (decode-coding-string bytes 'utf-16))))
"#,
    );
}

#[test]
fn div_utf8_utf16_encode_supplementary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((e (encode-coding-string "😀" 'utf-16)))
  (list (length e) (append e nil)))
"#,
    );
}

// --- eight-bit raw-byte width -----------------------------------------------

#[test]
fn div_utf8_eightbit_char_bytes_per_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Per-char byte cost of eight-bit characters — GNU reports 2.
    assert_oracle_parity(
        r#"
(list (char-bytes (unibyte-char-to-multibyte 128))
      (char-bytes (unibyte-char-to-multibyte 160))
      (char-bytes (unibyte-char-to-multibyte 200))
      (char-bytes (unibyte-char-to-multibyte 255)))
"#,
    );
}

#[test]
fn div_utf8_eightbit_string_bytes_byte_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((m (string-make-multibyte (unibyte-string 128 129 200 255))))
  (list (length m) (string-bytes m) (append m nil)))
"#,
    );
}

#[test]
fn div_utf8_eightbit_mixed_string_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // ASCII + eight-bit + multibyte (é) in one string.
    assert_oracle_parity(
        r#"
(let ((s (concat "a"
                 (string-make-multibyte (unibyte-string 200))
                 "é")))
  (list (length s) (string-bytes s) (append s nil)))
"#,
    );
}

#[test]
fn div_utf8_char_bytes_table_with_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(mapcar #'char-bytes
        (list ?a ?é ?\x3042
              (unibyte-char-to-multibyte 200)
              (unibyte-char-to-multibyte 255)
              #x3FFFFF))
"#,
    );
}

// --- eight-bit charset classification ---------------------------------------

#[test]
fn div_utf8_char_charset_eightbit_range() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(mapcar (lambda (b) (char-charset (unibyte-char-to-multibyte b)))
        (list 128 160 200 254 255))
"#,
    );
}

#[test]
fn div_utf8_encode_decode_char_eightbit_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((c (unibyte-char-to-multibyte 200)))
  (list (encode-char c 'eight-bit)
        (decode-char 'eight-bit 200)
        (char-charset (decode-char 'eight-bit 200))))
"#,
    );
}

// --- charset text properties on decode --------------------------------------

#[test]
fn div_utf8_decode_coding_string_charset_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Does in-memory latin-1 decode (not file I/O) also attach a charset prop?
    assert_oracle_parity(
        r#"
(let ((d (decode-coding-string (unibyte-string 99 97 102 233) 'latin-1)))
  (list d (text-properties-at 0 d) (length d)))
"#,
    );
}
