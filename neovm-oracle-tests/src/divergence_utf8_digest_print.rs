//! UTF-8 / multibyte *byte-representation-sensitive* divergence probes.
//!
//! Operations whose result depends on the internal byte layout of a string:
//! `md5`, `secure-hash`, `base64-encode-string`, `prin1-to-string`/`read`
//! round-trips, and `%S` printing.  These expose the eight-bit byte-width
//! divergence (and any print/read asymmetry) through new surfaces.
//!
//! Also pins a Neomacs internal inconsistency: `decode-coding-string` recovery
//! vs `string-make-multibyte` produce eight-bit chars with different internal
//! byte widths.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- hashes (byte-representation sensitive) ---------------------------------

#[test]
fn div_utf8_md5_ascii_and_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (md5 "abc")
      (md5 "café")
      (md5 "世界")
      (md5 "héllo"))
"#,
    );
}

#[test]
fn div_utf8_md5_of_recovered_eightbit_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // md5 hashes the internal bytes; eight-bit byte width (2 vs 3) diverges.
    assert_oracle_parity(
        r#"
(list (md5 (decode-coding-string (unibyte-string 200 201 255) 'utf-8))
      (md5 (string-make-multibyte (unibyte-string 200 201 255))))
"#,
    );
}

#[test]
fn div_utf8_secure_hash_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (secure-hash 'sha256 "café")
      (secure-hash 'sha1 "世界")
      (secure-hash 'md5 "héllo")
      (secure-hash 'sha512 "a😀b"))
"#,
    );
}

// --- base64 (byte-representation sensitive) ---------------------------------

#[test]
fn div_utf8_base64_multibyte_and_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (base64-encode-string "abc")
      (base64-encode-string "café")
      (base64-encode-string "世界")
      (base64-encode-string (decode-coding-string (unibyte-string 200 255) 'utf-8))
      (base64-encode-string (string-make-multibyte (unibyte-string 200 255))))
"#,
    );
}

// --- prin1 / read round-trip ------------------------------------------------

#[test]
fn div_utf8_prin1_multibyte_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let* ((s "café世界")
       (p (prin1-to-string s))
       (back (car (read-from-string p))))
  (list p (equal s back) (length p)))
"#,
    );
}

#[test]
fn div_utf8_prin1_eightbit_representation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // How eight-bit chars are printed (octal \NNN vs \xNN) and whether they
    // round-trip through read.
    assert_oracle_parity(
        r#"
(let* ((raw (decode-coding-string (unibyte-string 200 201 255) 'utf-8))
       (p (prin1-to-string raw))
       (back (car (read-from-string p))))
  (list p (equal raw back) (append back nil)))
"#,
    );
}

#[test]
fn div_utf8_format_S_multibyte_and_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (format "%S" "café")
      (format "%S" "世界")
      (format "%S" (decode-coding-string (unibyte-string 200) 'utf-8))
      (format "%S" (string-make-multibyte (unibyte-string 200))))
"#,
    );
}

// --- the pinned inconsistency -----------------------------------------------

#[test]
fn div_utf8_pinned_decode_vs_make_eightbit_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: both construction paths yield identical 2-byte eight-bit chars.
    // Neomacs: decode-coding-string recovery yields 3-byte storage while
    // string-make-multibyte yields 2-byte storage -> internal inconsistency.
    assert_oracle_parity(
        r#"
(let ((d (decode-coding-string (unibyte-string 200) 'utf-8))
      (m (string (unibyte-char-to-multibyte 200))))
  (list (string-bytes d) (string-bytes m)
        (equal d m)
        (append d nil) (append m nil)))
"#,
    );
}

// --- aref on unibyte vs multibyte raw bytes ---------------------------------

#[test]
fn div_utf8_aref_unibyte_vs_multibyte_indexing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((u (unibyte-string 200 201))
      (m (string-make-multibyte (unibyte-string 200 201))))
  (list (aref u 0) (aref u 1)
        (aref m 0) (aref m 1)))
"#,
    );
}
