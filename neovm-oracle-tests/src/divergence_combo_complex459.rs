/// Batch 459: eldoc, mode-line, header-line, paren, electric, visual-line deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx459_eldoc_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eldoc)
  (with-temp-buffer
    (emacs-lisp-mode)
    (setq eldoc-documentation-strategy 'eldoc-documentation-default)
    (eldoc-print-current-symbol-info)))"##,
    );
}

#[test]
fn div_cx459_mode_line_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (setq mode-line-format '("%b" ":" mode-line-position))
  (let ((s (format-mode-line mode-line-format)))
    (stringp s)))"##,
    );
}

#[test]
fn div_cx459_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'paren)
  (with-temp-buffer
    (insert "(hello)")
    (show-paren-mode 1)
    (list (facep 'show-paren-match)
          (facep 'show-paren-mismatch))))"##,
    );
}

#[test]
fn div_cx459_electric_pair_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'elec-pair)
  (with-temp-buffer
    (electric-pair-mode 1)
    (insert "(")
    (insert "[")
    (buffer-string)))"##,
    );
}

#[test]
fn div_cx459_visual_line_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (visual-line-mode 1)
  (setq fill-column 20)
  (insert "aaa bbb ccc ddd eee fff ggg hhh iii jjj kkk lll")
  (buffer-string))"##,
    );
}

#[test]
fn div_cx459_electric_indent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (emacs-lisp-mode)
  (electric-indent-mode 1)
  (insert "(defun foo ()\n  ")
  (buffer-string))"##,
    );
}

#[test]
fn div_cx459_global_display_line_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'display-line-numbers)
  (with-temp-buffer
    (insert "a\nb\nc\n")
    (display-line-numbers-mode 1)
    (display-line-numbers-mode)))"##,
    );
}

#[test]
fn div_cx459_font_lock_fontify_keywords() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun foo (x) (* x 2))")
    (font-lock-fontify-buffer)
    (list (get-text-property 2 'face)
          (get-text-property 8 'face))))"##,
    );
}

#[test]
fn div_cx459_subword_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'subword)
  (with-temp-buffer
    (insert "fooBarBaz")
    (subword-mode 1)
    (goto-char 1)
    (forward-word 1)
    (point)))"##,
    );
}

#[test]
fn div_cx459_whitespace_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'whitespace)
  (with-temp-buffer
    (insert "  leading spaces\n\ttab\n trailing  ")
    (whitespace-mode 1)
    (list whitespace-mode
          (boundp 'whitespace-style))))"##,
    );
}

#[test]
fn div_cx459_align_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "a  1\nbb 22\nccc  333\n")
  (align-regexp (point-min) (point-max) "\\(\\s-*\\)[0-9]+")
  (buffer-string))"##,
    );
}

#[test]
fn div_cx459_sort_regexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "b 2\na 1\nc 3\n")
  (sort-regexp-fields nil "\\([a-z]+\\)" "\\1" (point-min) (point-max))
  (buffer-string))"##,
    );
}

#[test]
fn div_cx459_repeat_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'repeat)
  (list (boundp 'repeat-mode) (fboundp 'repeat))))"##,
    );
}

#[test]
fn div_cx459_close_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'tab-bar)
  (list (boundp 'tab-bar-mode)
        (boundp 'tab-bar-tab-name-format))))"##,
    );
}

#[test]
fn div_cx459_display_cursor_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (list (boundp 'cursor-type)
        (default-value 'cursor-type)))"##,
    );
}
