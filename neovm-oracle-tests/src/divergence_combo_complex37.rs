//! Complex combo batch 37 — fresh subsystems: superword-mode, auto-fill-mode,
//! adaptive-fill, newcomment customization, outline-minor-mode, which-function-mode,
//! electric-pair-mode, electric-indent-mode, abbrev-mode deeper.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx37_superword_mode_forward_word() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (superword-mode 1)
      (insert "snake_case_var camelCase")
      (goto-char 1)
      (forward-word 1)
      (let ((p1 (point)))
        (forward-word 1)
        (list p1 (point))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx37_superword_subword_toggle_word_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "my_camelCase_word")
      (goto-char 1)
      (let ((default-pos (progn (forward-word 1) (point))))
        (goto-char 1)
        (subword-mode 1)
        (let ((sub-pos (progn (forward-word 1) (point))))
          (goto-char 1)
          (superword-mode 1)
          (let ((super-pos (progn (forward-word 1) (point))))
            (list default-pos sub-pos super-pos)))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx37_auto_fill_mode_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (auto-fill-mode 1)
      (let ((fill-column 10))
        (insert "the quick brown fox jumps over")
        (buffer-string)))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx37_adaptive_fill_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((fill-column 20) (adaptive-fill-mode t))
        (insert "// comment line one\n// comment line two more text here\n")
        (fill-region (point-min) (point-max))
        (buffer-string)))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx37_newcomment_custom_comment_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((comment-start "// ") (comment-end ""))
    (insert "line one\nline two\n")
    (comment-region 1 15)
    (let ((commented (buffer-string)))
      (uncomment-region 1 (point-max))
      (list commented (buffer-string))))
"##,
    );
}

#[test]
fn div_cx37_outline_minor_mode_headings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (outline-minor-mode 1)
      (let ((outline-regexp "[*]+ "))
        (insert "* Top\nbody\n** Sub\nmore body\n")
        (goto-char 1)
        (list (outline-level)
              (outline-next-heading)
              (outline-level))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx37_which_function_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (which-function-mode 1)
      (with-temp-buffer
        (emacs-lisp-mode)
        (insert "(defun neo-cx37-fn ()\n  body)\n")
        (goto-char 20)
        (which-function))
      (which-function-mode 0))
  (error (list :errored)))
"##,
    );
}

#[test]
fn div_cx37_electric_pair_mode_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (electric-pair-mode 1)
      (let ((last-command-event ?\())
        (electric-pair-post-self-insert-function))
      (buffer-string))
  (error (list :errored)))
"##,
    );
}

#[test]
fn div_cx37_abbrev_mode_case_preserving_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tbl (make-abbrev-table)))
  (define-abbrev tbl "neokw" "keyword-expanded" nil :case-fixed nil)
  (mapcar (lambda (trigger)
            (with-temp-buffer
              (set (make-local-variable 'local-abbrev-table) tbl)
              (abbrev-mode 1)
              (insert trigger " ")
              (expand-abbrev)
              (buffer-substring 1 (1- (point-max)))))
          '("neokw" "Neokw" "NEOKW")))
"##,
    );
}

#[test]
fn div_cx37_fill_paragraph_with_fill_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((fill-column 20) (fill-prefix "> "))
    (insert "> short text\n> more text that is longer than the column width\n")
    (fill-paragraph)
    (buffer-string)))
"##,
    );
}

#[test]
fn fn_div_cx37_subword_backward_movement() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (subword-mode 1)
      (insert "camelCaseString")
      (goto-char 16)
      (subword-backward 1)
      (point))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx37_comment_inline_and_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "code line\n")
  (let ((comment-start "/* ") (comment-end " */") (comment-style 'multi-line))
    (comment-region 1 10)
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx37_outline_hide_subtree_show() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (outline-mode)
      (insert "* Top\nbody1\n** Sub1\nbody2\n** Sub2\nbody3\n* Top2\nbody4\n")
      (goto-char 1)
      (hide-subtree)
      (let ((visible-count (count-lines (point-min) (point-max))))
        (show-subtree)
        (list visible-count (count-lines (point-min) (point-max)))))
  (error (cons 'errored (car e))))
"##,
    );
}

#[test]
fn div_cx37_coding_system_for_write_inhibit_eol_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (make-temp-file "neo-cx37-eol-")))
  (let ((coding-system-for-write 'utf-8-mac))
    (write-region "line1\nline2\n" nil f nil 'silent))
  (prog1 (with-temp-buffer
           (insert-file-contents f)
           (list (buffer-string) (string-bytes (buffer-string))))
    (ignore-errors (delete-file f))))
"##,
    );
}

#[test]
fn div_cx37_process_exit_code_consistency_call_vs_make() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((call-code (call-process "sh" nil nil nil "-c" "exit 3")))
  (let ((p (make-process :name "neo-cx37-ec" :command '("sh" "-c" "exit 3"))))
    (accept-process-output p 2)
    (list call-code (process-exit-status p))))
"##,
    );
}

#[test]
fn div_cx37_char_syntax_in_fundamental_vs_emacs_lisp_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-temp-buffer (char-syntax ?_))
      (with-temp-buffer (emacs-lisp-mode) (char-syntax ?_))
      (with-temp-buffer (c-mode) (char-syntax ?_)))
"##,
    );
}

#[test]
fn div_cx37_display_property_space_align_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 2 4 'display '(space :align-to 10))
  (current-column))
"##,
    );
}

#[test]
fn div_cx37_overlay_modification_hooks_fire_on_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (fired)
  (with-temp-buffer
    (insert "abcdef")
    (let ((ov (make-overlay 2 5)))
      (overlay-put ov 'face 'bold)
      (overlay-put ov 'modification-hooks
                   (list (lambda (o beg end &rest _) (push :fired fired))))
      (delete-region 3 4)))
  fired)
"##,
    );
}

#[test]
fn div_cx37_cl_loop_with_destructuring_and_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-loop for (k . v) in '((a . 1) (b . 2) (c . 3))
         for i from 0
         collect (list i k v))
"##,
    );
}

#[test]
fn div_cx37_set_buffer_multibyte_then_insert_multibyte_then_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "ascii")
  (set-buffer-multibyte nil)
  (insert (unibyte-string 200 201))
  (set-buffer-multibyte t)
  (list (length (buffer-string)) (point-max) (append (buffer-string) nil)))
"##,
    );
}
