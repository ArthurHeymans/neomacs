//! Complex combo batch 111 — `read` from buffer / stream / minibuffer
//! with malformed data, circular refs, char-read syntax (#), record syntax.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx111_read_from_buffer_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "(alpha beta (gamma delta)) trailing")
  (goto-char 1)
  (let ((obj (read (current-buffer))))
    (list obj (point) (read (current-buffer)))))
"##,
    );
}

#[test]
fn div_cx111_read_special_syntaxes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (condition-case e
              (car (read-from-string s))
            (error (cons :err (car e)))))
        '("[1 2 3]"
          "#(1 2 3)"
          "#(a b c)"
          "#s(record a b c)"
          "?A"
          "?\\C-a"
          "?\\M-a"
          "?\\x41"
          "?\\u00e9"
          "#x10"
          "#o17"
          "#b1010"
          "1.5"
          "1/2"
          "1.0e3"
          "1000000000000000000000000"
          "0.000001"))
"##,
    );
}

#[test]
fn div_cx111_read_with_circular_ref_when_print_circle_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((obj (list 1 2 3)))
  (setcdr (cddr obj) obj)
  (let* ((printed (let ((print-circle t)) (prin1-to-string obj)))
         (read-back (let ((read-circle t))
                      (car (read-from-string printed)))))
    (list printed
          (car read-back)
          (cadr read-back)
          (caddr read-back))))
"##,
    );
}

#[test]
fn div_cx111_read_with_shared_refs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((shared (list 1 2 3))
       (container (list shared shared shared)))
  (let ((printed (let ((print-circle t)) (prin1-to-string container))))
    (list printed
          (car (read-from-string printed)))))
"##,
    );
}

#[test]
fn div_cx111_read_invalid_syntax_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (condition-case e
              (car (read-from-string s))
            (error (car e))))
        '("(open paren"
          "[open bracket"
          "{open brace"
          "."
          "#[invalid"
          "#<<unknown"
          "\"unterminated string"
          "1.2.3"
          "#xZZZ"))
"##,
    );
}

#[test]
fn div_cx111_read_with_comments_and_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (car (read-from-string "  ; comment\n  symbol  ; another\n  after"))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx111_read_multiple_objects_from_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((s "alpha beta gamma delta")
      (pos 0)
      (results nil))
  (while (< pos (length s))
    (let ((res (read-from-string s pos)))
      (push (car res) results)
      (setq pos (cdr res))))
  (nreverse results))
"##,
    );
}

#[test]
fn div_cx111_read_multibyte_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (condition-case e
              (let ((r (read-from-string s)))
                (list (car r) (type-of (car r))))
            (error (cons :err (car e)))))
        '("α"
          "Ω-symbol"
          "世界"
          "café"
          "\"café\""))
"##,
    );
}

#[test]
fn div_cx111_read_string_escaped_quotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (condition-case e
              (car (read-from-string s))
            (error (cons :err (car e)))))
        '("\"simple\""
          "\"with \\\"escaped\\\"\""
          "\"with \\\\ backslash\""
          "\"with \n newline\""
          "\"with \\t tab\""
          "\"\""
          "\"\\u00e9\""))
"##,
    );
}

#[test]
fn div_cx111_read_kv_cons_cells_and_dotted_pairs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (condition-case e
              (car (read-from-string s))
            (error (cons :err (car e)))))
        '("(a . b)"
          "(a b . c)"
          "(a b c . d)"
          "((a . 1) (b . 2))"
          "(nil . nil)"))
"##,
    );
}

#[test]
fn div_cx111_read_byte_code_function_objects() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (car (read-from-string "#[1 \"abc\" nil nil nil]"))
          (car (read-from-string "#s(hash-table size 10 test eq data (a 1 b 2))")))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx111_read_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((data "((name . \"alpha\") (value . 42) (tags . (a b c)))")
       (parsed (car (read-from-string data))))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert data)
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 20)
      (goto-char 1)
      (let ((obj (read (current-buffer))))
        (let ((state (list parsed obj
                           (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (undo)
          (widen)
          (list state (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1))))))
"##,
    );
}
