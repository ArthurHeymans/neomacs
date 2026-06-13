//! UTF-8 / multibyte *buffer multibyte toggling* divergence probes.
//!
//! Characterizes the data-corruption bug found in `set-buffer-multibyte t`
//! (raw-byte promotion): toggling a unibyte buffer holding raw bytes back to
//! multibyte can drop trailing ASCII bytes. These probes vary the byte pattern
//! to pin exactly when corruption occurs, for a tight teammate reproduction.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_utf8_toggle_unibyte_to_multibyte_trailing_ascii_dropped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Original corruption case: bytes (200 201 65) -> NEO drops trailing 65.
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (append (buffer-string) nil)
        (multibyte-string-p (buffer-string))))
"#,
    );
}

#[test]
fn div_utf8_toggle_unibyte_to_multibyte_raw_bytes_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // No trailing ASCII — does corruption still occur?
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 255))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (append (buffer-string) nil)))
"#,
    );
}

#[test]
fn div_utf8_toggle_unibyte_to_multibyte_leading_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // ASCII first, then raw bytes.
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 65 66 200 201))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (append (buffer-string) nil)))
"#,
    );
}

#[test]
fn div_utf8_toggle_unibyte_to_multibyte_interleaved() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // ASCII and raw bytes interleaved.
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 65 200 66 201 67 255))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (append (buffer-string) nil)))
"#,
    );
}

#[test]
fn div_utf8_toggle_unibyte_to_multibyte_single_raw_byte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (append (buffer-string) nil)
        (multibyte-string-p (buffer-string))))
"#,
    );
}

#[test]
fn div_utf8_toggle_double_roundtrip_multibyte_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // multibyte -> unibyte -> multibyte round trip stability.
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café世界")
  (let ((original (buffer-string)))
    (set-buffer-multibyte nil)
    (set-buffer-multibyte t)
    (list (buffer-string) (equal original (buffer-string))
          (length (buffer-string)) (append (buffer-string) nil))))
"#,
    );
}

#[test]
fn div_utf8_toggle_with_narrowing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 65 200 201 66))
  (narrow-to-region 2 4)
  (set-buffer-multibyte t)
  (list (point-min) (point-max) (append (buffer-string) nil)))
"#,
    );
}

#[test]
fn div_utf8_toggle_unibyte_to_multibyte_preserves_point_max() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // point-max before vs after the toggle.
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 65 66))
  (let ((before (point-max)))
    (set-buffer-multibyte t)
    (list before (point-max) (length (buffer-string)) (append (buffer-string) nil))))
"#,
    );
}
