//! Complex combo batch 253 — `syntax-table` comment classes deep:
//! comment styles (`;` line, `/* */` block, `# ` hash), `syntax-ppss`
//! state with nested comments, `comment-start` / `comment-end` per-mode.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx253_syntax_ppss_comment_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(defun foo ()\n  \"docstring\"\n  (body)) ; comment\n")
  (list (nth 4 (syntax-ppss 1))
        (nth 4 (syntax-ppss 30))
        (nth 4 (syntax-ppss 45))
        (nth 4 (syntax-ppss 50))
        (nth 8 (syntax-ppss 50))))
"##,
    )
}

#[test]
fn div_cx253_comment_start_end_per_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (let ((before (list comment-start comment-end comment-start-skip)))
        (setq comment-start "/* ")
        (setq comment-end " */")
        (setq comment-start-skip "/\\*+[ \t]*")
        (list before
              (list comment-start comment-end comment-start-skip))))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx253_forward_comment_line_and_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "code1\n; line comment\ncode2\n/* block\nmultiline */ code3")
  (goto-char 8)
  (forward-comment 1)
  (let ((p1 (point)))
    (forward-comment 1)
    (let ((p2 (point)))
      (forward-comment 1)
      (list p1 p2 (point) (char-after)))))
"##,
    )
}

#[test]
fn div_cx253_comment_syntax_classes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-syntax-table (make-syntax-table))
  (modify-syntax-entry ?\; "<")
  (modify-syntax-entry ?\n ">")
  (modify-syntax-entry ?/ ". 14")
  (modify-syntax-entry ?* ". 23")
  (list (char-syntax ?\;)
        (char-syntax ?\n)
        (char-syntax ?/)
        (char-syntax ?*)))
"##,
    )
}

#[test]
fn div_cx253_parse_partial_sexp_in_block_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "before /* comment\nmultiline\n */ after")
  (list (nth 4 (parse-partial-sexp 1 15))
        (nth 4 (parse-partial-sexp 1 25))
        (nth 4 (parse-partial-sexp 1 35))
        (nth 4 (parse-partial-sexp 1 40))))
"##,
    )
}

#[test]
fn div_cx253_comment_functions_availability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (fboundp 'comment-region)
      (fboundp 'uncomment-region)
      (fboundp 'comment-or-uncomment-region)
      (fboundp 'comment-dwim)
      (fboundp 'comment-indent)
      (boundp 'comment-style)
      (boundp 'comment-column))
"##,
    )
}

#[test]
fn div_cx253_syntax_table_with_string_escapes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "\"string with \\\"escapes\\\"\" rest")
  (list (nth 3 (syntax-ppss 1))
        (nth 3 (syntax-ppss 10))
        (nth 3 (syntax-ppss 30))
        (nth 8 (syntax-ppss 30))))
"##,
    )
}

#[test]
fn div_cx253_comment_search_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "code1\n; first comment\ncode2\n; second comment\ncode3")
  (goto-char 1)
  (list (comment-search-forward 30 t)
        (comment-search-forward 60 t)))
"##,
    )
}

#[test]
fn div_cx253_indent_for_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'indent-for-comment)
          (fboundp 'set-comment-column)
          (boundp 'comment-inline-offset))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx253_comment_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "code1\n; comment line\ncode2\n/* block comment */ code3")
  (put-text-property 1 5 'face 'bold)
  (let ((m (set-marker (make-marker) 10))
        (ov (make-overlay 5 25)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 40)
    (let ((state (list (nth 4 (syntax-ppss 15))
                       (nth 4 (syntax-ppss 30))
                       (char-syntax ?\;)
                       (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1))))
      (undo)
      (widen)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1)))))
"##,
    )
}
