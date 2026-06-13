//! UTF-8 / multibyte *string primitive* divergence probes.
//!
//! Probes `length`/`string-bytes`, `aref`/`aset`/`substring`/`store-substring`,
//! `concat` mixing unibyte and multibyte, `split-string`, `char-to-string`,
//! `format` with `%c`/`%s`, and `mapconcat` — all over non-ASCII text.  Under a
//! UTF-8-internal model the byte accounting (`string-bytes`) and raw-byte
//! promotion in `concat`/`store-substring` are the likeliest divergence points.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_utf8_str_length_vs_string_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s "héllo世界😀"))
  (list (length s) (string-bytes s) (multibyte-string-p s)))
"#,
    );
}

#[test]
fn div_utf8_str_length_bytes_latin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s "café"))
  (list (length s) (string-bytes s)))
"#,
    );
}

#[test]
fn div_utf8_aref_multibyte_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s "héllo"))
  (list (aref s 0) (aref s 1) (aref s 2) (aref s 4)))
"#,
    );
}

#[test]
fn div_utf8_aset_high_codepoint_growth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s (copy-sequence "abcdef")))
  (aset s 1 #x3042)
  (list (length s) (string-bytes s) (append s nil)))
"#,
    );
}

#[test]
fn div_utf8_substring_multibyte_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s "aébçd"))
  (list (substring s 0 2) (substring s 1 4) (substring s -3) (substring s 2)))
"#,
    );
}

#[test]
fn div_utf8_concat_unibyte_and_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Concatenating a unibyte-with-raw-bytes and a multibyte string must
    // promote the raw bytes to eight-bit characters.
    assert_oracle_parity(
        r#"
(let ((r (concat (unibyte-string 200 201) "xy")))
  (list (multibyte-string-p r) (unibyte-string-p r)
        (length r) (string-bytes r) (append r nil)))
"#,
    );
}

#[test]
fn div_utf8_concat_two_unibyte_stays_unibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((r (concat (unibyte-string 200) (unibyte-string 201))))
  (list (multibyte-string-p r) (unibyte-string-p r)
        (length r) (string-bytes r) (append r nil)))
"#,
    );
}

#[test]
fn div_utf8_split_string_multibyte_sep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(split-string "aébçd" "é")
"#,
    );
}

#[test]
fn div_utf8_char_to_string_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (char-to-string ?é)
      (length (char-to-string ?é))
      (string-bytes (char-to-string ?é))
      (length (char-to-string #x3042))
      (string-bytes (char-to-string #x3042)))
"#,
    );
}

#[test]
fn div_utf8_format_percent_c_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((c1 (format "%c" 233))
      (c2 (format "%c" #x3042))
      (c3 (format "%c" #x1f600)))
  (list c1 c2 c3
        (length c1) (length c2) (length c3)
        (string-bytes c1) (string-bytes c2) (string-bytes c3)))
"#,
    );
}

#[test]
fn div_utf8_format_percent_s_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s "café"))
  (list (format "%s" s) (format "%s%d" s (length s)) (format "%S" s)))
"#,
    );
}

#[test]
fn div_utf8_mapconcat_char_codes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (mapconcat #'char-to-string "aéb" "-")
      (mapconcat (lambda (c) (format "%X" c)) "aéz" ","))
"#,
    );
}

#[test]
fn div_utf8_store_substring_byte_indexed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // store-substring is byte-indexed and can grow the string's byte length.
    assert_oracle_parity(
        r#"
(let ((s (copy-sequence "abcdef")))
  (store-substring s 2 #x3042)
  (list (length s) (string-bytes s) (append s nil)))
"#,
    );
}

#[test]
fn div_utf8_store_substring_raw_byte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s (copy-sequence "abcdef")))
  (store-substring s 1 (unibyte-char-to-multibyte 200))
  (list (length s) (string-bytes s) (append s nil)))
"#,
    );
}

#[test]
fn div_utf8_empty_string_multibyte_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (multibyte-string-p "")
      (unibyte-string-p "")
      (multibyte-string-p (unibyte-string))
      (multibyte-string-p (string-make-multibyte "")))
"#,
    );
}

#[test]
fn div_utf8_string_equal_after_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((literal "café")
      (decoded (decode-coding-string (unibyte-string 99 97 102 233) 'latin-1)))
  (list (equal literal decoded)
        (equal (append literal nil) (append decoded nil))))
"#,
    );
}
