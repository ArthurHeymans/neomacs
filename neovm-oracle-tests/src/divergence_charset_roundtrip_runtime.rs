//! Non-UTF charset round-trips (shift_jis, euc-jp, gbk, big5, koi8-r,
//! iso-2022-jp) and char encode/decode; plus the iso-8859-15 decode
//! charset-text-property gap.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn charset_encode_specific() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (encode-coding-char ?A 'utf-8)
        (multibyte-char-to-unibyte ?é)
        (decode-char 'latin-iso8859-1 233)
        (encode-char ?A 'ascii))"##,
    );
}

#[test]
fn euc_jp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s "テスト123"))
  (list (string= s (decode-coding-string (encode-coding-string s 'euc-jp) 'euc-jp))
        (length (encode-coding-string s 'euc-jp))))"##,
    );
}

#[test]
fn gb_big5() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s1 "中文") (s2 "繁體"))
  (list (string= s1 (decode-coding-string (encode-coding-string s1 'gbk) 'gbk))
        (string= s2 (decode-coding-string (encode-coding-string s2 'big5) 'big5))))"##,
    );
}

#[test]
fn iso2022_jp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s "漢字テスト"))
  (string= s (decode-coding-string (encode-coding-string s 'iso-2022-jp) 'iso-2022-jp)))"##,
    );
}

#[test]
fn koi8_cyrillic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s "Привет"))
  (list (string= s (decode-coding-string (encode-coding-string s 'koi8-r) 'koi8-r))
        (length (encode-coding-string s 'koi8-r))))"##,
    );
}

#[test]
fn shift_jis() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s "日本語 abc"))
  (list (string= s (decode-coding-string (encode-coding-string s 'shift_jis) 'shift_jis))
        (length (encode-coding-string s 'shift_jis))))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: decode-coding-string with iso-8859-15 does not attach the `charset` text property; GNU tags decoded chars with (charset iso-8859-15)."]
fn divergence_decode_iso8859_15_charset_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (decode-coding-string (unibyte-string 233 164) 'iso-8859-15)
      (get-text-property 0 'charset (decode-coding-string (unibyte-string 164) 'iso-8859-15)))"##,
    );
}
