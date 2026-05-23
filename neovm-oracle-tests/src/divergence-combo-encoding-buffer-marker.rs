//! Divergence tests: encoding + buffer + marker + minibuffer history combos.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_multibyte_insert_marker_positions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "ABC")
  (let ((m1 (copy-marker 1))
        (m2 (copy-marker 4)))
    (goto-char 2)
    (insert "\xc3\xa9\xc3\xa9")
    (let ((s1 (buffer-string))
          (p1 (marker-position m1))
          (p2 (marker-position m2))
          (bs1 (buffer-size)))
      (goto-char (point-max))
      (insert "\xe2\x82\xac")
      (list s1 p1 p2 bs1
            (buffer-string)
            (buffer-size)
            (marker-position m2)
            (= (marker-position m2) (+ p2 1)))))) "#,
    );
}

#[test]
fn divergence_encode_decode_buffer_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello \xc3\xa9 World \xe2\x82\xac")
  (let* ((raw (buffer-string))
         (encoded (encode-coding-string raw 'utf-8))
         (decoded (decode-coding-string encoded 'utf-8)))
    (list (string= raw decoded)
          (length raw)
          (length decoded)
          (= (length raw) (length decoded))
          (string-equal raw decoded)))) "#,
    );
}

#[test]
fn divergence_buffer_local_marker_encoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar test-blm-pos-xxx 0)
  (make-variable-buffer-local 'test-blm-pos-xxx)
  (insert "ABCDE")
  (let ((m (copy-marker 3)))
    (setq test-blm-pos-xxx (marker-position m))
    (goto-char 2)
    (insert "\xc3\xa9\xc3\xa9")
    (let ((new-pos (marker-position m)))
      (list test-blm-pos-xxx
            (= test-blm-pos-xxx 3)
            new-pos
            (> new-pos test-blm-pos-xxx)
            (buffer-string))))) "#,
    );
}

#[test]
fn divergence_charset_conversion_after_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "\xc3\xa9\xc3\xa0\xc3\xb9")
  (let* ((s1 (buffer-string))
         (len1 (length s1)))
    (goto-char 1)
    (insert "X")
    (let* ((s2 (buffer-string))
           (len2 (length s2)))
      (list (= len1 3)
            (= len2 4)
            (string= (substring s2 1) s1)
            (string= s2 (concat "X" s1)))))) "#,
    );
}

#[test]
fn divergence_substring_multibyte_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (let ((s "A\xc3\xa9B\xc3\xa0C"))
    (list (substring s 0 1)
          (substring s 0 2)
          (substring s 1 3)
          (length s)
          (string= (substring s 0 1) "A")
          (string= (substring s 1 2) "\xc3\xa9")
          (string= (substring s 2 3) "B")
          (string= (substring s 3 4) "\xc3\xa0")
          (string= (substring s 4 5) "C")))) "#,
    );
}

#[test]
fn divergence_narrow_multibyte_search_marker() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "AAA\xc3\xa9BBB\xc3\xa0CCC")
  (let ((m (copy-marker 4)))
    (narrow-to-region 4 10)
    (goto-char (point-min))
    (let ((found (re-search-forward "\xc3\xa0" nil t)))
      (widen)
      (list found
            (when found (match-beginning 0))
            (when found (match-end 0))
            (marker-position m)
            (buffer-string))))) "#,
    );
}

#[test]
fn divergence_replace_multibyte_preserves_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "A\xc3\xa9B\xc3\xa0C")
  (let ((m1 (copy-marker 1))
        (m2 (copy-marker 2))
        (m3 (copy-marker 3))
        (m4 (copy-marker 4))
        (m5 (copy-marker 5)))
    (goto-char 1)
    (undo-boundary)
    (while (re-search-forward "\xc3\xa9\\|\xc3\xa0" nil t)
      (replace-match "X"))
    (list (buffer-string)
          (mapcar 'marker-position (list m1 m2 m3 m4 m5))))) "#,
    );
}

#[test]
fn divergence_case_change_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (let ((lower "\xc3\xa9\xc3\xa0\xc3\xb9")
        (upper "\xc3\x89\xc3\x80\xc3\x99"))
    (list (string= (upcase lower) upper)
          (string= (downcase upper) lower)
          (string= (upcase "hello") "HELLO")
          (string= (downcase "HELLO") "hello")
          (length lower)
          (length upper)
          (= (length lower) (length upper))))) "#,
    );
}

#[test]
fn divergence_buffer_multibyte_undo_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "START")
  (undo-boundary)
  (goto-char 3)
  (insert "\xc3\xa9")
  (undo-boundary)
  (goto-char (point-max))
  (insert "\xc3\xa0")
  (let ((s1 (buffer-string))
        (bs1 (buffer-size)))
    (primitive-undo 1 buffer-undo-list)
    (let ((s2 (buffer-string))
          (bs2 (buffer-size)))
      (primitive-undo 1 buffer-undo-list)
      (list s1 bs1 s2 bs2
            (buffer-string) (buffer-size)
            (string= (buffer-string) "START"))))) "#,
    );
}

#[test]
fn divergence_char_after_multibyte_positions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "A\xc3\xa9B\xc3\xa0C\xc3\xb9D")
  (list (char-after 1)
        (char-after 2)
        (char-after 3)
        (char-after 4)
        (char-after 5)
        (= (char-after 1) 65)
        (= (char-after 3) 66)
        (= (char-after 5) 67)
        (= (char-after 7) 68)
        (= (aref (buffer-string) 1) (char-after 2)))) "#,
    );
}
