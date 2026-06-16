//! Complex combo batch 38 — extend word-movement/subword/superword vein:
//! backward-word, upcase-word, downcase-word, capitalize-word, kill-word,
//! mark-word, forward/all under subword and superword modes. Plus remaining
//! minor-mode interactions.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx38_subword_backward_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "camelCaseString")
      (goto-char 16)
      (backward-word 1)
      (point))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_subword_upcase_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "camelCaseString")
      (goto-char 1)
      (upcase-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_subword_downcase_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "CamelCaseString")
      (goto-char 1)
      (downcase-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_subword_capitalize_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "camel case string")
      (goto-char 1)
      (capitalize-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_subword_kill_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "camelCaseString rest")
      (goto-char 1)
      (kill-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_subword_backward_kill_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "rest camelCaseString")
      (goto-char 20)
      (backward-kill-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_superword_backward_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (superword-mode 1)
      (insert "snake_case_var rest")
      (goto-char 15)
      (backward-word 1)
      (point))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_superword_upcase_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (superword-mode 1)
      (insert "snake_case_var rest")
      (goto-char 1)
      (upcase-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_superword_kill_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (superword-mode 1)
      (insert "snake_case_var rest")
      (goto-char 1)
      (kill-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_superword_capitalize_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (superword-mode 1)
      (insert "snake_case_var rest")
      (goto-char 1)
      (capitalize-word 1)
      (buffer-string))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_subword_mark_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "camelCaseString rest")
      (goto-char 1)
      (mark-word 1)
      (list (region-beginning) (region-end)))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_subword_forward_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "myCamelVar = someValue")
      (goto-char 1)
      (list (progn (forward-word 1) (point))
            (progn (forward-word 1) (point))
            (progn (forward-word 1) (point))
            (progn (forward-word 1) (point))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx38_display_property_relative_width_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 2 4 'display '(space :relative-width 5))
  (current-column))
"##,
    );
}

#[test]
fn div_cx38_process_exit_code_make_process_exit_0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((p (make-process :name "neo-cx38-e0" :command '("true"))))
  (accept-process-output p 2)
  (list (process-status p) (process-exit-status p)))
"##,
    );
}

#[test]
fn div_cx38_encode_coding_region_utf8_length_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "café世界"))
  (with-temp-buffer
    (insert s)
    (encode-coding-region (point-min) (point-max) 'utf-8)
    (length (buffer-string))))
"##,
    );
}

#[test]
fn div_cx38_overlay_display_string_column_effect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 1 3 'display "XXXXXX")
  (current-column))
"##,
    );
}

#[test]
fn div_cx38_set_buffer_multibyte_corruption_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201 202 203 204 65 66 67))
  (set-buffer-multibyte t)
  (length (buffer-string)))
"##,
    );
}

#[test]
fn div_cx38_coding_system_priority_list_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(length (coding-system-priority-list))
"##,
    );
}

#[test]
fn div_cx38_fill_paragraph_with_long_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((fill-column 10))
    (insert "short thisisaverylongwordthatwontfit end\n")
    (fill-paragraph)
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx38_abbrev_mode_hook_on_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tbl (make-abbrev-table)))
  (define-abbrev tbl "neohook" "expanded" nil)
  (let (hook-fired)
    (with-temp-buffer
      (set (make-local-variable 'local-abbrev-table) tbl)
      (abbrev-mode 1)
      (add-hook 'abbrev-expand-functions
                (lambda (abbrev) (setq hook-fired :hook-fired)) nil t)
      (insert "neohook ")
      (expand-abbrev)
      (list (buffer-string) hook-fired))))
"##,
    );
}

#[test]
fn div_cx38_undo_after_multiple_word_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "hello world foo bar")
  (goto-char 1)
  (upcase-word 1)
  (let ((p1 (point)))
    (capitalize-word 1)
    (list p1 (point) (buffer-string))))
"##,
    );
}
