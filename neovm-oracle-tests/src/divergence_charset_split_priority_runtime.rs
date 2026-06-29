//! Charset/mule parity: encode unencodable (substitution bytes), find-charset-
//! string/region, make-char/charset-after/decode-char, encode-char per charset,
//! char-charset with restriction, eol-type vector, decode invalid utf-8; plus
//! split-char and charset-priority-list registry divergences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn char_charset_restrict() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (char-charset ?A '(ascii latin-iso8859-1))
        (char-charset ?\s) (char-charset ?\t))"##,
        expect_test::expect![[r#""OK (ascii ascii ascii)""#]],
    );
}

#[test]
fn decode_invalid_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((d (decode-coding-string (unibyte-string 255 254 65) 'utf-8)))
  (list (length d) (multibyte-string-p d)))"##,
        expect_test::expect![[r#""OK (3 t)""#]],
    );
}

#[test]
fn encode_char_charsets() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (encode-char ?A 'ascii) (encode-char ?あ 'japanese-jisx0208)
        (decode-char 'ascii 65) (charsetp 'japanese-jisx0208))"##,
        expect_test::expect![[r#""OK (65 9250 65 t)""#]],
    );
}

#[test]
fn encode_unencodable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (append (encode-coding-string "a日b" 'us-ascii) nil)
        (append (encode-coding-string "café" 'iso-8859-1) nil)
        (append (encode-coding-string "日" 'iso-8859-1) nil))"##,
        expect_test::expect![[r#""OK ((97 63 98) (99 97 102 233) (32))""#]],
    );
}

#[test]
fn encode_unencodable_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(let ((r (encode-coding-string "héllo" 'us-ascii)))
  (list (length r) (multibyte-string-p r) (append (string-to-unibyte r) nil)))"##,
        expect_test::expect![[r#""OK (5 nil (104 63 108 108 111))""#]],
    );
}

#[test]
fn eol_type_undecided_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (coding-system-eol-type 'undecided)
        (coding-system-eol-type 'utf-8)
        (vectorp (coding-system-eol-type 'undecided)))"##,
        expect_test::expect![[
            r#""OK ([undecided-unix undecided-dos undecided-mac] [utf-8-unix utf-8-dos utf-8-mac] t)""#
        ]],
    );
}

#[test]
fn find_charset_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "hello 日本")
  (sort (mapcar #'symbol-name (find-charset-region (point-min) (point-max))) #'string<))"##,
        expect_test::expect![[r#""OK (\"ascii\" \"unicode-bmp\")""#]],
    );
}

#[test]
fn find_charset_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (find-charset-string "abc")
        (find-charset-string "café")
        (sort (mapcar #'symbol-name (find-charset-string "a日")) #'string<))"##,
        expect_test::expect![[r#""OK ((ascii) (ascii unicode-bmp) (\"ascii\" \"unicode-bmp\"))""#]],
    );
}

#[test]
fn make_char_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (make-char 'ascii 65) (characterp (make-char 'ascii 65)) (charset-after) (decode-char 'ascii 65))"##,
        expect_test::expect![[r#""OK (65 t nil 65)""#]],
    );
}

#[test]
#[ignore = "DIVERGENCE: split-char returns nil instead of (CHARSET CODE...): GNU gives (split-char ?A) => (ascii 65), ?-hiragana-a => (unicode-bmp 48 66)."]
fn divergence_split_char_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (split-char ?A) (split-char ?あ) (split-char ?0))"##,
        expect_test::expect![[r#""OK ((ascii 65) (unicode-bmp 48 66) (ascii 48))""#]],
    );
}

#[test]
fn divergence_charset_priority_list_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r##"(list (> (length (charset-priority-list)) 50)
      (and (memq 'big5 (charset-priority-list)) t)
      (and (memq 'windows-1252 (charset-priority-list)) t))"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}
