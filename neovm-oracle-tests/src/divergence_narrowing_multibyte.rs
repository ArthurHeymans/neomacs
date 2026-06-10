//! Divergence tests: narrowing, region, multibyte, and buffer edge cases.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_narrow_insert_extends() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefghij")
  (narrow-to-region 3 7)
  (goto-char 7)
  (insert "XYZ")
  (list (point-min) (point-max) (buffer-string)))"#,
    );
}

#[test]
fn divergence_narrow_delete_shrinks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefghij")
  (narrow-to-region 3 7)
  (delete-region 3 5)
  (list (point-min) (point-max) (buffer-string)))"#,
    );
}

#[test]
fn divergence_save_restriction_nested_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefghij")
  (narrow-to-region 3 7)
  (save-restriction
    (widen)
    (goto-char 10)
    (insert "Z"))
  (list (point-min) (point-max) (buffer-string)))"#,
    );
}

#[test]
fn divergence_buffer_string_with_narrowing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "abcdefghij")
  (narrow-to-region 3 7)
  (list (buffer-string)
        (buffer-substring (point-min) (point-max))
        (buffer-substring-no-properties 4 6)))"#,
    );
}

#[test]
fn divergence_position_bytes_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert "ābc中d")
  (list (position-bytes 1)
        (position-bytes 2)
        (position-bytes 3)
        (position-bytes 4)
        (position-bytes 5)
        (point-max)
        (buffer-size)))"#,
    );
}

#[test]
fn divergence_insert_char_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (insert-char ?ā 3)
  (insert-char ?中 2)
  (list (buffer-string) (buffer-size) (point)))"#,
    );
}

#[test]
fn divergence_decode_encode_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((s "ābc中"))
  (list (encode-coding-string s 'utf-8)
        (decode-coding-string (encode-coding-string s 'utf-8) 'utf-8)
        (length s)
        (string-bytes s)))"#,
    );
}

#[test]
fn divergence_char_syntax_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list (char-syntax ?a)
              (char-syntax ?0)
              (char-syntax ? )
              (char-syntax ?\n)
              (char-syntax ?()
              (char-syntax ?\")
              (char-syntax ?\\))"#,
    );
}

#[test]
fn divergence_upcase_downcase_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list (upcase "hello world")
              (downcase "HELLO WORLD")
              (upcase-initials "hello world")
              (capitalize "hello world"))"#,
    );
}

#[test]
fn divergence_string_to_number_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list (string-to-number \"42\")\n              (string-to-number \"3.14\")\n              (string-to-number \"0xff\")\n              (string-to-number \"abc\")\n              (string-to-number \"42abc\")\n              (string-to-number \"\"))",
    );
}

#[test]
fn divergence_number_to_string_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list (number-to-string 42)
              (number-to-string -3)
              (number-to-string 3.14)
              (number-to-string (expt 2 64)))"#,
    );
}

#[test]
fn divergence_buffer_file_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (list (buffer-file-name)
        (buffer-file-name (current-buffer))))"#,
    );
}

#[test]
fn divergence_kill_buffer_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((buf (get-buffer-create " *test-kill-restore*")))
  (with-current-buffer buf (insert "hello"))
  (kill-buffer buf)
  (list (get-buffer " *test-kill-restore*")
        (buffer-live-p buf)
        (buffer-name buf)))"#,
    );
}

#[test]
fn divergence_get_buffer_create_vs_generate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((a (get-buffer-create " *test-gbc*"))
        (b (generate-new-buffer " *test-gbc*")))
  (unwind-protect
      (list (buffer-name a)
            (string-match-p "\\` \\*test-gbc\\*-[0-9]+\\'" (buffer-name b))
            (not (eq a b))
            (eq a (get-buffer " *test-gbc*"))
            (eq b (get-buffer (buffer-name b))))
    (kill-buffer a)
    (kill-buffer b)))"#,
    );
}
