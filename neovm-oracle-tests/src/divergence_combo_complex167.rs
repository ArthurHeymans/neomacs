//! Complex combo batch 167 — `read` / `parse-partial-sexp` /
//! `parse-partial-sexp` for nested structures, scan-lists, syntax-table
//! driven parsing across modes.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx167_parse_partial_sexp_through_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(defun foo ()\n  \"docstring with \\\"escapes\\\" inside\"\n  body)")
  (list (parse-partial-sexp 1 10)
        (parse-partial-sexp 1 30)
        (parse-partial-sexp 1 60)
        (nth 3 (parse-partial-sexp 1 30))
        (nth 8 (parse-partial-sexp 1 30))))
"##,
    );
}

#[test]
fn div_cx167_parse_partial_sexp_with_comments() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(foo ; line comment\n bar) ; another\n after")
  (list (parse-partial-sexp 1 15)
        (nth 4 (parse-partial-sexp 1 15))
        (parse-partial-sexp 1 30)))
"##,
    );
}

#[test]
fn div_cx167_scan_lists_nested_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(one (two (three (four))) one)")
  (goto-char 1)
  (list (scan-lists (point) 1 0)
        (scan-lists (point) 1 1)
        (scan-lists (point) 1 2)
        (scan-lists (point) 1 3)
        (scan-lists (point) -1 0)))
"##,
    );
}

#[test]
fn div_cx167_scan_sexps_paren_jump() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a b (c d) e) f")
  (goto-char 1)
  (list (scan-sexps (point) 1)
        (scan-sexps (point) 2)
        (scan-sexps (point) 3)))
"##,
    );
}

#[test]
fn div_cx167_forward_list_navigate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a) (b) (c)")
  (goto-char 1)
  (condition-case e
      (progn
        (forward-list 1)
        (let ((p1 (point)))
          (forward-list 1)
          (let ((p2 (point)))
            (forward-list 1)
            (list p1 p2 (point)))))
    (error (list :error (car e)))))
"##,
    );
}

#[test]
fn div_cx167_backward_list_navigate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a) (b) (c)")
  (goto-char (point-max))
  (condition-case e
      (progn
        (backward-list 1)
        (let ((p1 (point)))
          (backward-list 1)
          (let ((p2 (point)))
            (backward-list 1)
            (list p1 p2 (point)))))
    (error (list :error (car e)))))
"##,
    );
}

#[test]
fn div_cx167_up_list_navigate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a (b (c (d) e) f) g)")
  (goto-char 14)
  (condition-case e
      (progn
        (up-list 1)
        (let ((p1 (point)))
          (up-list 1)
          (let ((p2 (point)))
            (up-list 1)
            (list p1 p2 (point)))))
    (error (list :error (car e)))))
"##,
    );
}

#[test]
fn div_cx167_down_list_navigate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(a (b (c)))")
  (goto-char 1)
  (condition-case e
      (progn
        (down-list 1)
        (let ((p1 (point)))
          (down-list 1)
          (let ((p2 (point)))
            (down-list 1)
            (list p1 p2 (point)))))
    (error (list :error (car e)))))
"##,
    );
}

#[test]
fn div_cx167_parse_partial_sexp_in_string_with_braces() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "before \"string with {braces} and [brackets]\" after")
  (list (parse-partial-sexp 1 25)
        (nth 3 (parse-partial-sexp 1 25))
        (nth 8 (parse-partial-sexp 1 25))))
"##,
    );
}

#[test]
fn div_cx167_syntax_ppss_cached() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(defun foo ()\n  \"docstring\"\n  (body))")
  (list (syntax-ppss 1)
        (syntax-ppss 15)
        (syntax-ppss 30)
        (syntax-ppss 45)))
"##,
    );
}

#[test]
fn div_cx167_parse_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "(defun mega ()\n  \"docstring\"\n  (let ((x 1))\n    (+ x 1)))")
  (put-text-property 1 8 'face 'bold)
  (let ((m (set-marker (make-marker) 20))
        (ov (make-overlay 4 30)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 35)
    (let ((state (list (parse-partial-sexp 1 30)
                       (nth 3 (syntax-ppss 25))
                       (scan-lists 1 1 0)
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
    );
}
