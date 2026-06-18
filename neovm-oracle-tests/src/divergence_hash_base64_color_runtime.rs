//! md5/secure-hash/sha1, base64/base64url encode-decode, hex/radix number
//! conversions, color-name-to-rgb / rgb-to-hex, subst-char/translate-region,
//! and character predicate parity.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn base64_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (base64-encode-string "Hello, World!")
        (base64-decode-string "SGVsbG8=")
        (base64url-encode-string "subjects?_d" t)
        (base64-encode-string (string-to-unibyte (encode-coding-string "café" 'utf-8))))"##,
    );
}

#[test]
fn hex_string_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (format "%x" 255) (string-to-number "ff" 16)
        (string-to-number "1010" 2) (number-to-string 255))"##,
    );
}

#[test]
fn md5_sha() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (md5 "hello") (secure-hash 'sha1 "hello")
        (secure-hash 'sha256 "abc") (sha1 "test"))"##,
    );
}

#[test]
fn secure_hash_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "content here")
  (list (secure-hash 'md5 (current-buffer))
        (secure-hash 'sha256 (current-buffer) (point-min) 7)))"##,
    );
}

#[test]
fn char_displayable_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (characterp ?A) (characterp 9999999)
        (max-char) (characterp (max-char))
        (logand ?A #xff) (ash ?A -4))"##,
    );
}

#[test]
fn color_conversions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (color-name-to-rgb "white")
        (color-name-to-rgb "black")
        (color-rgb-to-hex 1.0 0.0 0.0)
        (color-rgb-to-hex 0.5 0.5 0.5 2))"##,
    );
}

#[test]
fn subst_translate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((tbl (make-char-table 'translation-table)))
  (aset tbl ?a ?X) (aset tbl ?b ?Y)
  (list (subst-char-in-string ?l ?L "hello")
        (with-temp-buffer (insert "abcabc") (translate-region (point-min) (point-max) tbl) (buffer-string))))"##,
    );
}

#[test]
fn text_quoting() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-replace "'" "X" "it's")
        (replace-regexp-in-string "\\([a-z]\\)\\1" "<\\1>" "aabbcc")
        (subst-char-in-string ?\s ?_ "a b c"))"##,
    );
}
