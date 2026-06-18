//! Eight-bit / raw-byte char handling parity: char-charset of raw bytes
//! (eight-bit), string-to-multibyte/unibyte roundtrips, byte<->char
//! conversion, raw bytes in a buffer, max-char, decode keeping eight-bit,
//! decode-char 'eight-bit.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn byte_char_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (multibyte-char-to-unibyte (unibyte-char-to-multibyte 200))
        (unibyte-char-to-multibyte 65) (multibyte-char-to-unibyte ?A)
        (byte-to-string 200) (length (byte-to-string 200)))"##,
    );
}

#[test]
fn char_to_byte_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (multibyte-char-to-unibyte ?λ)
        (char-charset ?λ) (char-charset ?A) (char-charset ?あ)
        (encode-char ?A 'unicode) (decode-char 'eight-bit 200))"##,
    );
}

#[test]
fn decode_eightbit_keep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((d (decode-coding-string (unibyte-string 65 200 66) 'utf-8)))
  (list (length d) (multibyte-string-p d)
        (mapcar (lambda (c) (char-charset c)) (string-to-list d))))"##,
    );
}

#[test]
fn eight_bit_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (char-charset (unibyte-char-to-multibyte 200))
        (char-charset 200) (char-charset 128) (char-charset 255)
        (charsetp 'eight-bit))"##,
    );
}

#[test]
fn eight_bit_in_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert (unibyte-char-to-multibyte 200))
  (insert (unibyte-char-to-multibyte 255))
  (list (buffer-size) (char-after 1) (char-charset (char-after 1))
        (string-bytes (buffer-string))))"##,
    );
}

#[test]
fn max_char_min_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (max-char) (max-char t) (characterp (max-char))
        (char-charset (max-char)) (char-charset #x10FFFF))"##,
    );
}

#[test]
fn raw_byte_to_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s (string-to-multibyte (unibyte-string 200 201 202))))
  (list (multibyte-string-p s) (length s) (mapcar #'identity s)
        (mapcar (lambda (c) (char-charset c)) s)))"##,
    );
}

#[test]
fn string_to_unibyte_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let* ((m (string-to-multibyte (unibyte-string 130 240)))
        (u (string-to-unibyte m)))
  (list (length u) (append u nil) (string= u (unibyte-string 130 240))))"##,
    );
}
