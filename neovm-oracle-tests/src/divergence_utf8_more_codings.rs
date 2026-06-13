//! UTF-8 / multibyte *coding-system coverage* divergence probes.
//!
//! Probes whether Neomacs implements the broader coding-system taxonomy that
//! GNU ships (latin-9, windows-1252, iso-8859-7, big5, gbk, shift_jis, euc-jp,
//! koi8-r) via encode/decode round-trips, plus the `coding-system-plist`
//! `:signature` property — the root cause of the `utf-8-with-signature` BOM
//! divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- :signature plist (BOM root cause) --------------------------------------

#[test]
fn div_utf8_coding_system_plist_signature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (plist-get (coding-system-plist 'utf-8) :signature)
      (plist-get (coding-system-plist 'utf-8-with-signature) :signature)
      (plist-get (coding-system-plist 'utf-16) :signature)
      (plist-get (coding-system-plist 'utf-16le) :signature)
      (plist-get (coding-system-plist 'latin-1) :signature))
"#,
    );
}

#[test]
fn div_utf8_coding_system_category_and_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (coding-system-plist 'utf-8-with-signature)
      (plist-get (coding-system-plist 'utf-8-with-signature) :category)
      (plist-get (coding-system-plist 'utf-8-with-signature) :name)
      (plist-get (coding-system-plist 'utf-8-with-signature) :eol-type))
"#,
    );
}

// --- 8-bit coding systems ---------------------------------------------------

#[test]
fn div_utf8_latin9_euro_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let* ((euro (string #x20AC))
       (e (encode-coding-string euro 'latin-9))
       (d (decode-coding-string e 'latin-9)))
  (list (append e nil) (equal euro d) (length e) (string-bytes e)))
"#,
    );
}

#[test]
fn div_utf8_windows1252_smart_quotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((q (encode-coding-string (string #x2019) 'windows-1252))
      (lz (encode-coding-string (string #x201C) 'windows-1252)))
  (list (append q nil) (append lz nil)))
"#,
    );
}

#[test]
fn div_utf8_iso8859_7_greek_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((g (decode-coding-string (unibyte-string 211 212 213) 'iso-8859-7)))
  (list (append g nil) (length g)))
"#,
    );
}

#[test]
fn div_utf8_koi8_r_cyrillic_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((c (decode-coding-string (unibyte-string 193 194 195) 'koi8-r)))
  (list (append c nil) (length c)))
"#,
    );
}

// --- CJK coding systems -----------------------------------------------------

#[test]
fn div_utf8_big5_cjk_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let* ((s "中文測試")
       (e (encode-coding-string s 'big5))
       (d (decode-coding-string e 'big5)))
  (list (append e nil) (equal s d) (length e) (string-bytes e)))
"#,
    );
}

#[test]
fn div_utf8_gbk_cjk_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let* ((s "中文测试")
       (e (encode-coding-string s 'gbk))
       (d (decode-coding-string e 'gbk)))
  (list (append e nil) (equal s d) (length e)))
"#,
    );
}

#[test]
fn div_utf8_shiftjis_japanese_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let* ((s "こんにちは")
       (e (encode-coding-string s 'shift_jis))
       (d (decode-coding-string e 'shift_jis)))
  (list (append e nil) (equal s d) (length e)))
"#,
    );
}

#[test]
fn div_utf8_eucjp_japanese_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let* ((s "日本語テスト")
       (e (encode-coding-string s 'euc-jp))
       (d (decode-coding-string e 'euc-jp)))
  (list (append e nil) (equal s d) (length e)))
"#,
    );
}

// --- coding-system coverage existence ---------------------------------------

#[test]
fn div_utf8_coding_system_existence_broad() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (coding-system-p 'latin-9)
      (coding-system-p 'iso-8859-7)
      (coding-system-p 'windows-1252)
      (coding-system-p 'cp1251)
      (coding-system-p 'big5)
      (coding-system-p 'chinese-big5)
      (coding-system-p 'gbk)
      (coding-system-p 'shift_jis)
      (coding-system-p 'sjis)
      (coding-system-p 'euc-jp)
      (coding-system-p 'koi8-r)
      (coding-system-p 'utf-8-emacs))
"#,
    );
}

// --- detect-coding-string ---------------------------------------------------

#[test]
fn div_utf8_detect_coding_string_bom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (detect-coding-string (unibyte-string 239 187 191 97 98 99))
      (detect-coding-string (unibyte-string 254 255 0 97))
      (detect-coding-string (unibyte-string 255 254 97 0)))
"#,
    );
}
