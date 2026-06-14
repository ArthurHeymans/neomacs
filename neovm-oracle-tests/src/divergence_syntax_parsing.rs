//! Syntax parsing divergence probes (calibration).
//!
//! Probes parse-partial-sexp state vectors (paren depth, in-string,
//! in-comment, quoted, comment-style) across various buffer contents,
//! scan-lists, scan-sexps, and forward-sexp/list/up-list/down-list navigation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_sp_parse_partial_basic_depth() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b) c)")
  (parse-partial-sexp 1 4))
"##,
    );
}

#[test]
fn div_sp_parse_partial_into_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(concat \"abc")
  (parse-partial-sexp 1 12))
"##,
    );
}

#[test]
fn div_sp_parse_partial_into_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "; abc comment")
  (parse-partial-sexp 1 8))
"##,
    );
}

#[test]
fn div_sp_parse_partial_nested_parens() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(((a)))")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_parse_partial_escaped_quote_in_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "\"a\\\"b")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_parse_partial_semicolon_in_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "\"a;b\"")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_scan_lists_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b) c) x")
  (scan-lists 1 1))
"##,
    );
}

#[test]
fn div_sp_scan_lists_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "x (a (b) c)")
  (scan-lists 12 -1))
"##,
    );
}

#[test]
fn div_sp_scan_sexps_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "a b c d")
  (scan-sexps 1 2))
"##,
    );
}

#[test]
fn div_sp_forward_sexp_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a b) (c d)")
  (goto-char 1)
  (list (progn (forward-sexp) (point))
        (progn (forward-sexp) (point))))
"##,
    );
}

#[test]
fn div_sp_forward_list_up_down() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b c) d)")
  (goto-char 1)
  (list (progn (forward-list) (point))
        (progn (backward-list) (point))
        (progn (down-list) (point))))
"##,
    );
}

#[test]
fn div_sp_up_list_from_inner() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b c) d)")
  (goto-char 5)
  (condition-case err (progn (up-list) (point)) (error (car err))))
"##,
    );
}

#[test]
fn div_sp_parse_partial_quoted_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "\\(a\\)")
  (parse-partial-sexp 1 5))
"##,
    );
}

#[test]
fn div_sp_parse_partial_box_quotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(comment \"text\")")
  (parse-partial-sexp 1 10))
"##,
    );
}

#[test]
fn div_sp_parse_partial_oldstate_continue() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a \"str\" (b))")
  (let* ((s1 (parse-partial-sexp 1 5))
         (s2 (parse-partial-sexp 5 9 nil nil s1)))
    (list s1 s2)))
"##,
    );
}

#[test]
fn div_sp_unbalanced_paren_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(a (b")
  (condition-case err (scan-lists 1 1) (scan-error (list 'scan-error)) (error (car err))))
"##,
    );
}
