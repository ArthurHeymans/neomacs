//! UTF-8 / multibyte *buffer position* and *I/O* divergence probes.
//!
//! Probes the char-position ↔ byte-position mapping (`position-bytes`,
//! `byte-to-position`), `char-after`/`char-before`, `encode`/`decode-coding-region`,
//! narrowing at multibyte boundaries, and file coding round-trips.  A
//! UTF-8-internal model tends to keep char/byte mappings consistent with UTF-8,
//! which can diverge from GNU's internal multibyte byte layout.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- char-position <-> byte-position ----------------------------------------

#[test]
fn div_utf8_position_bytes_multibyte_mapping() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café世界")
  (list (point-max)
        (position-bytes (point-max))
        (position-bytes 4)
        (position-bytes 5)
        (position-bytes 6)
        (byte-to-position 6)))
"#,
    );
}

#[test]
fn div_utf8_byte_to_position_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "aébçd😀")
  (list (mapcar (lambda (p) (position-bytes p)) '(1 2 3 4 5 6))
        (mapcar (lambda (b) (byte-to-position b)) '(1 2 3 4 5))))
"#,
    );
}

#[test]
fn div_utf8_position_bytes_at_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "xéy")
  (list (position-bytes (point-min))
        (position-bytes (point-max))
        (point-min) (point-max)))
"#,
    );
}

// --- char-after / char-before ----------------------------------------------

#[test]
fn div_utf8_char_after_before_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "aébç")
  (goto-char 3)
  (list (char-after) (char-after 1) (char-after 2)
        (preceding-char) (char-before) (following-char)))
"#,
    );
}

#[test]
fn div_utf8_char_after_supplementary_plane() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "a😀b")
  (list (char-after 1) (char-after 2) (char-after 3) (char-after 4)))
"#,
    );
}

// --- encode/decode-coding-region -------------------------------------------

#[test]
fn div_utf8_encode_coding_region_latin1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café")
  (encode-coding-region (point-min) (point-max) 'latin-1)
  (list (buffer-string) (length (buffer-string)) (append (buffer-string) nil)))
"#,
    );
}

#[test]
fn div_utf8_decode_coding_region_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 99 97 102 195 169))
  (decode-coding-region (point-min) (point-max) 'utf-8)
  (list (buffer-string) (point-max) (string-bytes (buffer-string))))
"#,
    );
}

// --- narrowing at multibyte boundaries -------------------------------------

#[test]
fn div_utf8_narrow_to_region_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "aébçd")
  (narrow-to-region 2 5)
  (list (point-min) (point-max) (buffer-string)))
"#,
    );
}

#[test]
fn div_utf8_narrow_then_position_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café世界")
  (narrow-to-region 1 5)
  (list (point-min) (point-max) (buffer-string)
        (position-bytes (point-max))))
"#,
    );
}

// --- buffer-substring / insertion edges ------------------------------------

#[test]
fn div_utf8_buffer_substring_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "héllo世界")
  (list (buffer-substring 2 4)
        (buffer-substring-no-properties 5 7)
        (string-bytes (buffer-substring 1 (point-max)))))
"#,
    );
}

#[test]
fn div_utf8_insert_then_char_position_consistency() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abc")
  (goto-char 2)
  (insert "é世界")
  (list (point) (point-max) (buffer-string)
        (position-bytes (point))))
"#,
    );
}

// --- file coding round trip ------------------------------------------------

#[test]
fn div_utf8_file_roundtrip_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((tmp (make-temp-file "utf8-oracle-")))
  (unwind-protect
      (progn
        (let ((coding-system-for-write 'utf-8-unix))
          (write-region "café世界😀" nil tmp nil 'silent))
        (with-temp-buffer
          (let ((coding-system-for-read 'utf-8-unix))
            (insert-file-contents tmp))
          (list (buffer-string) (point-max) (string-bytes (buffer-string)))))
    (delete-file tmp)))
"#,
    );
}

#[test]
fn div_utf8_file_roundtrip_latin1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((tmp (make-temp-file "latin1-oracle-")))
  (unwind-protect
      (progn
        (let ((coding-system-for-write 'latin-1-unix))
          (write-region "café" nil tmp nil 'silent))
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (let ((coding-system-for-read 'latin-1-unix))
            (insert-file-contents tmp))
          (list (buffer-string) (point-max) (append (buffer-string) nil))))
    (delete-file tmp)))
"#,
    );
}
