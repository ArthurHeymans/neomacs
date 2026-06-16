//! Complex combo batch 77 — syntax tables deep: char syntax, syntax
//! classes, syntax begin/end, scan-sexps, parse-partial-sexp across
//! nested quotes, comment styles, and per-mode syntax tables.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx77_syntax_class_of_chars_in_default_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (c) (list c (char-syntax c)))
        '(?a ?A ?0 ?_ ?- ?\s ?\t ?\( ?\) ?\" ?\; ?. ?, ?' ?\\ ?# ?$ ?% ?& ?* ?+ ?< ?> ?@ ?/ ?| ?~))
"##,
    );
}

#[test]
fn div_cx77_modify_syntax_entry_locally_in_temp_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((st (copy-syntax-table (syntax-table))))
    (set-syntax-table st)
    (modify-syntax-entry ?_ "w")
    (modify-syntax-entry ?- "_")
    (modify-syntax-entry ?! ".")
    (list (char-syntax ?_)
          (char-syntax ?-)
          (char-syntax ?!)
          (char-syntax ?a))))
"##,
    );
}

#[test]
fn div_cx77_scan_sexps_paren_matching() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "alpha (beta (gamma) delta) epsilon")
  (goto-char 7)
  (list (scan-sexps (point) 1)
        (scan-sexps (point) 2)
        (scan-lists (point) 1 0)
        (scan-lists (point) -1 0)
        (condition-case e (scan-sexps (point) 99) (error (car e)))))
"##,
    );
}

#[test]
fn div_cx77_forward_comment_across_line_and_block_comments() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "before ; line comment\nalpha\nbefore2 /* block\nmulti-line */ after")
  (goto-char 8)
  (forward-comment 1)
  (let ((p1 (point)))
    (forward-comment 1)
    (let ((p2 (point)))
      (forward-comment 1)
      (list p1 p2 (point) (char-after)))))
"##,
    );
}

#[test]
fn div_cx77_parse_partial_sexp_state_machine() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(defun foo ()\n  \"docstring\"\n  (let ((x 1)) ; comment\n    (+ x 1)))")
  (list (parse-partial-sexp 1 10)
        (parse-partial-sexp 1 30)
        (parse-partial-sexp 1 40)
        (parse-partial-sexp 1 60)
        (syntax-ppss 10)
        (syntax-ppss 30)))
"##,
    );
}

#[test]
fn div_cx77_comment_styles_default_semicolon_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "code1\n; full comment\ncode2 ;; inline comment\ncode3\n/* block c1\nblock c2 */ code4")
  (goto-char 1)
  (let ((comment-start-1 (condition-case e (comment-search-forward 30 t) (error :err)))
        (comment-end-1 (condition-case e (comment-forward 1) (error :err))))
    (list comment-start-1 comment-end-1 (point) (char-after))))
"##,
    );
}

#[test]
fn div_cx77_syntax_of_quote_and_string_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "\"string with \\\"escapes\\\" inside\" rest")
  (goto-char 1)
  (let ((in-string-after-1 (nth 3 (syntax-ppss 1)))
        (in-string-after-3 (nth 3 (syntax-ppss 3)))
        (in-string-after-10 (nth 3 (syntax-ppss 10)))
        (in-string-after-30 (nth 3 (syntax-ppss 30)))
        (start-of-string (nth 8 (syntax-ppss 30))))
    (list in-string-after-1 in-string-after-3
          in-string-after-10 in-string-after-30
          start-of-string)))
"##,
    );
}

#[test]
fn div_cx77_syntax_table_make_and_modify_per_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((buf-a (get-buffer-create " *neo-cx77-a*"))
      (buf-b (get-buffer-create " *neo-cx77-b*")))
  (with-current-buffer buf-a
    (set-syntax-table (make-syntax-table))
    (modify-syntax-entry ?@ "w"))
  (with-current-buffer buf-b
    (set-syntax-table (make-syntax-table))
    (modify-syntax-entry ?@ "_"))
  (let ((a-syntax (with-current-buffer buf-a (char-syntax ?@)))
        (b-syntax (with-current-buffer buf-b (char-syntax ?@))))
    (kill-buffer buf-a)
    (kill-buffer buf-b)
    (list a-syntax b-syntax)))
"##,
    );
}

#[test]
fn div_cx77_matching_paren_jump() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "outer (inner [nested {leaf} nested] inner) outer")
  (goto-char 8)        ; at "("
  (let ((forward-paren (scan-lists (point) 1 0)))
    (goto-char forward-paren)
    (backward-list)
    (let ((backward-pos (point)))
      (forward-list)
      (list forward-paren backward-pos (point)))))
"##,
    );
}

#[test]
fn div_cx77_word_motion_under_custom_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (set-syntax-table (make-syntax-table))
  (modify-syntax-entry ?_ "w")
  (modify-syntax-entry ?- "w")
  (insert "snake_case_var camelCase-kebab dotted.name")
  (goto-char 1)
  (forward-word 1)
  (let ((p1 (point)))
    (forward-word 1)
    (let ((p2 (point)))
      (backward-word 1)
      (list p1 p2 (point) (char-after)))))
"##,
    );
}

#[test]
fn div_cx77_syntax_describe_and_class_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (syntax-class-to-char (string-to-syntax "w"))
          (syntax-class-to-char (string-to-syntax "_"))
          (syntax-class-to-char (string-to-syntax "."))
          (syntax-class-to-char (string-to-syntax "\""))
          (syntax-class-to-char (string-to-syntax "<"))
          (syntax-class-to-char (string-to-syntax ">"))
          (string-to-syntax "w")
          (string-to-syntax "( "))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx77_syntax_table_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (set-syntax-table (make-syntax-table))
  (modify-syntax-entry ?_ "w")
  (insert "var_name_1 (call_with_args x y) end_token")
  (put-text-property 1 5 'face 'bold)
  (let ((m (set-marker (make-marker) 12))
        (ov (make-overlay 4 24)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 35)
    (goto-char 1)
    (forward-word 2)
    (forward-comment 1)
    (let ((state (list (point) (char-syntax (char-after))
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (buffer-string)
                       (nth 0 (syntax-ppss (point)))
                       (nth 3 (syntax-ppss (point))))))
      (undo)
      (widen)
      (list state
            (buffer-string) (marker-position m)
            (overlayp ov) (overlay-start ov)
            (text-properties-at 1)))))
"##,
    );
}
