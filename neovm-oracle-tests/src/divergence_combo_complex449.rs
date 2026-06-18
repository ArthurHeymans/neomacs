//! Complex combo batch 449 — 15 fresh edge probes: fill-region-paragraph,
//! string-to-multibyte idempotent, fillarray, copy-alist nested,
//! copy-tree vectors, with-demoted-errors, dash/underscore syntax,
//! char-table range large, string-lessp/greaterp, file-absolute-relative,
//! encode-time decoded-time struct, string-trim/blank-p, format obarray,
//! interactive lambda specs, function alias-p.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx449_fill_region_paragraph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (text-mode)
  (insert "This is a long line that should be filled at the specified column boundary for testing purposes")
  (fill-region-as-paragraph (point-min) (point-max) nil)
  (buffer-string))"##,
    );
}

#[test]
fn div_cx449_string_to_multibyte_idempotent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s "cafe世界"))
  (list (string-multibyte-p s)
        (string-multibyte-p (string-to-multibyte s))
        (equal s (string-to-multibyte s))))"##,
    );
}

#[test]
fn div_cx449_fillarray_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((v (vector 1 2 3))
      (s "abc"))
  (fillarray v 0)
  (fillarray s ?x)
  (list v s))"##,
    );
}

#[test]
fn div_cx449_copy_alist_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((al '((a . 1) (b (c d) . e))))
  (let ((copy (copy-alist al)))
    (setcdr (assq 'a al) 99)
    (list (cdr (assq 'a al)) (cdr (assq 'a copy))))))"##,
    );
}

#[test]
fn div_cx449_copy_tree_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((tree '(a [1 2] (b . c))))
  (let ((copy (copy-tree tree t)))
    (aset (cadr tree) 0 99)
    (list (aref (cadr tree) 0) (aref (cadr copy) 0))))"##,
    );
}

#[test]
fn div_cx449_with_demoted_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (with-demoted-errors "DEMO: %S" (car 1 2))
      (with-demoted-errors "DEMO: %S" (+ 1 2)))"##,
    );
}

#[test]
fn div_cx449_dash_underscore_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((st (make-syntax-table)))
  (modify-syntax-entry ?_ "w" st)
  (modify-syntax-entry ?- "w" st)
  (with-temp-buffer
    (set-syntax-table st)
    (insert "foo_bar baz-qux")
    (goto-char 1) (forward-word) (forward-word) (point)))"##,
    );
}

#[test]
fn div_cx449_char_table_range_large() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ct (make-char-table 'category-table)))
  (set-char-table-range ct 32 126 'w)
  (list (char-table-range ct 65) (char-table-range ct 31)))"##,
    );
}

#[test]
fn div_cx449_string_lessp_greaterp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-lessp "abc" "def")
      (string-greaterp "xyz" "abc")
      (string-lessp "cafe" "cafe")
      (string-lessp "abc" "abcd"))"##,
    );
}

#[test]
fn div_cx449_file_absolute_relative() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (file-name-absolute-p "/tmp/test.el")
      (file-relative-name "/tmp/test.el" "/tmp"))"##,
    );
}

#[test]
fn div_cx449_encode_decoded_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((dt (decode-time (encode-time 0 0 12 16 6 2024 nil))))
      (list (decoded-time-year dt) (decoded-time-month dt) (decoded-time-day dt)))
  (error (car e)))"##,
    );
}

#[test]
fn div_cx449_string_trim_clean() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-trim "  hello  ")
      (string-trim-left "  hello")
      (string-trim-right "hello  ")
      (string-blank-p "  ")
      (string-blank-p "abc"))"##,
    );
}

#[test]
fn div_cx449_format_S_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((obs (obarray-default)))
  (format "%S" obs))"##,
    );
}

#[test]
fn div_cx449_interactive_lambda_specs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (lambda (x) (interactive "P") x)))
  (interactive-form f))"##,
    );
}

#[test]
fn div_cx449_function_alias_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((sym (make-symbol "neo-cx449-fa")))
  (defalias sym 'forward-char)
  (list (fboundp sym)
        (symbol-function sym)
        (condition-case e (indirect-function sym) (error (car e)))))"##,
    );
}
