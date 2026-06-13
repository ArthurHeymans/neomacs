//! UTF-8 / multibyte *print-escape & byte serialization* divergence probes.
//!
//! Probes `print-escape-nonascii` / `print-escape-multibyte` (which emit the
//! internal byte representation as octal escapes), `encode-hex-string`, and
//! `set-buffer-multibyte` toggling.  All depend on the internal byte layout,
//! so eight-bit chars (3 vs 2 byte width) are the likely divergence vector.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- print-escape-nonascii --------------------------------------------------

#[test]
fn div_utf8_print_escape_nonascii_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((print-escape-nonascii t))
  (list (prin1-to-string "café")
        (prin1-to-string "世界")))
"#,
    );
}

#[test]
fn div_utf8_print_escape_nonascii_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Escaped octal of an eight-bit char exposes the 2-vs-3 byte divergence.
    assert_oracle_parity(
        r#"
(let ((print-escape-nonascii t))
  (list (prin1-to-string (decode-coding-string (unibyte-string 200) 'utf-8))
        (prin1-to-string (string-make-multibyte (unibyte-string 200)))))
"#,
    );
}

// --- print-escape-multibyte -------------------------------------------------

#[test]
fn div_utf8_print_escape_multibyte_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((print-escape-multibyte t))
  (list (prin1-to-string "café")
        (prin1-to-string "世界")
        (prin1-to-string (string-make-multibyte (unibyte-string 200)))))
"#,
    );
}

// --- encode-hex-string ------------------------------------------------------

#[test]
fn div_utf8_encode_hex_string_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (encode-hex-string "abc")
      (encode-hex-string "café")
      (encode-hex-string "世界"))
"#,
    );
}

#[test]
fn div_utf8_encode_hex_string_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (encode-hex-string (decode-coding-string (unibyte-string 200 255) 'utf-8))
      (encode-hex-string (string-make-multibyte (unibyte-string 200 255))))
"#,
    );
}

// --- set-buffer-multibyte toggling ------------------------------------------

#[test]
fn div_utf8_set_buffer_multibyte_toggle_with_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café")
  (let ((multibyte-before (buffer-string)))
    (set-buffer-multibyte nil)
    (list (buffer-string)
          (length (buffer-string))
          (multibyte-string-p (buffer-string))
          (append (buffer-string) nil))))
"#,
    );
}

#[test]
fn div_utf8_set_buffer_multibyte_toggle_with_raw_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65))
  (list (length (buffer-string)) (append (buffer-string) nil))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (append (buffer-string) nil)
        (multibyte-string-p (buffer-string))))
"#,
    );
}

// --- prin1 round-trip stability ---------------------------------------------

#[test]
fn div_utf8_prin1_roundtrip_eightbit_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let* ((s (string-make-multibyte (unibyte-string 200 201 255)))
       (p (prin1-to-string s))
       (back (car (read-from-string p))))
  (list (equal s back) p))
"#,
    );
}
