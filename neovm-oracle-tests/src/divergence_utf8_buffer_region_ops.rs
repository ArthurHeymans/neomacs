//! UTF-8 / multibyte *buffer region operations* divergence probes.
//!
//! Probes buffer-substring, insert-buffer-substring, delete/transpose chars,
//! and region coding over buffers that hold multibyte and eight-bit raw-byte
//! characters — the surfaces where the eight-bit handling bugs reappear.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- buffer-substring with eight-bit chars ----------------------------------

#[test]
fn div_utf8_buffer_substring_with_recovered_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert (decode-coding-string (unibyte-string 65 200 201 66) 'utf-8))
  (list (buffer-substring 1 (point-max))
        (buffer-substring 2 4)
        (append (buffer-substring 1 (point-max)) nil)))
"#,
    );
}

#[test]
fn div_utf8_buffer_substring_constructed_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert (string-make-multibyte (unibyte-string 65 200 201 66)))
  (list (append (buffer-substring 1 (point-max)) nil)
        (append (buffer-substring 2 4) nil)))
"#,
    );
}

// --- insert-buffer-substring ------------------------------------------------

#[test]
fn div_utf8_insert_buffer_substring_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((src (generate-new-buffer " *s*"))
      (dst (generate-new-buffer " *d*")))
  (with-current-buffer src (insert "café世界"))
  (with-current-buffer dst (insert-buffer-substring src))
  (let ((r (with-current-buffer dst (buffer-string))))
    (kill-buffer src)
    (kill-buffer dst)
    (list r (length r) (append r nil))))
"#,
    );
}

#[test]
fn div_utf8_insert_buffer_substring_eightbit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((src (generate-new-buffer " *s*"))
      (dst (generate-new-buffer " *d*")))
  (with-current-buffer src
    (insert (decode-coding-string (unibyte-string 200 201 65) 'utf-8)))
  (with-current-buffer dst (insert-buffer-substring src 2 3))
  (let ((r (with-current-buffer dst (buffer-string))))
    (kill-buffer src)
    (kill-buffer dst)
    (list (append r nil) (length r))))
"#,
    );
}

// --- delete / transpose over multibyte --------------------------------------

#[test]
fn div_utf8_delete_char_over_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "a世界b")
  (goto-char 2)
  (delete-char 1)
  (list (buffer-string) (point)))
"#,
    );
}

#[test]
fn div_utf8_delete_backward_char_over_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "a世界b")
  (goto-char 4)
  (delete-backward-char 1)
  (list (buffer-string) (point)))
"#,
    );
}

#[test]
fn div_utf8_transpose_chars_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "aébç")
  (goto-char 2)
  (transpose-chars 1)
  (list (buffer-string) (point)))
"#,
    );
}

#[test]
fn div_utf8_transpose_words_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café thé world")
  (goto-char 2)
  (transpose-words 1)
  (list (buffer-string)))
"#,
    );
}

// --- region coding ----------------------------------------------------------

#[test]
fn div_utf8_encode_coding_region_utf16_in_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "AB")
  (encode-coding-region (point-min) (point-max) 'utf-16)
  (list (length (buffer-string)) (append (buffer-string) nil)))
"#,
    );
}

#[test]
fn div_utf8_encode_coding_region_with_signature_in_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // encode-coding-region 'utf-8-with-signature' — does it emit BOM?
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "abc")
  (encode-coding-region (point-min) (point-max) 'utf-8-with-signature)
  (list (length (buffer-string)) (append (buffer-string) nil)))
"#,
    );
}
