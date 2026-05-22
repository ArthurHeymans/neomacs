//! Divergence tests: syntax table, parse-partial, scan-lists deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_syntax_table_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'make-syntax-table)
  (fboundp 'copy-syntax-table)
  (fboundp 'set-syntax-table)
  (fboundp 'syntax-table)
  (fboundp 'modify-syntax-entry))"#,
    );
}

#[test]
fn divergence_char_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (char-syntax ?a)
  (char-syntax ? )
  (char-syntax ?()
  (char-syntax ?))
  (char-syntax ?\")
  (char-syntax ?\;)) "#,
    );
}

#[test]
fn divergence_parse_partial_sexp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "(foo (bar baz) quux)")
  (list (parse-partial-sexp 1 5)
        (parse-partial-sexp 1 20)
        (scan-lists 1 1 0)
        (scan-lists 1 1 1))) "#,
    );
}

#[test]
fn divergence_forward_sexps() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "(foo bar) (baz quux)")
  (goto-char 1)
  (forward-sexp 1)
  (let ((pos1 (point)))
    (forward-sexp 1)
    (list pos1 (point)
          (progn (backward-sexp 1) (point))
          (progn (backward-sexp 1) (point))))) "#,
    );
}

#[test]
fn divergence_scan_lists_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "(a (b c) d (e (f g) h) i)")
  (list (scan-lists 1 1 0)
        (scan-lists 1 2 0)
        (scan-lists 1 -1 0)
        (scan-lists 1 1 1))) "#,
    );
}

#[test]
fn divergence_forward_comment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "foo ; comment\nbar")
  (list (forward-comment 1)
        (point)
        (forward-comment -1)
        (point))) "#,
    );
}

#[test]
fn divergence_syntax_class() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((st (syntax-table)))
  (list (aref (syntax-table) ?a)
        (aref (syntax-table) ?()
        (aref (syntax-table) ?)
        (syntax-class (aref (syntax-table) ?a))
        (syntax-class (aref (syntax-table) ?()) ))) "#,
    );
}

#[test]
fn divergence_indent_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "(defun foo ()\n  (bar))")
  (list (fboundp 'calculate-lisp-indent)
        (fboundp 'lisp-indent-function)
        (parse-partial-sexp 1 22))) "#,
    );
}

#[test]
fn divergence_matching_paren() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'matching-paren)
  (matching-paren ?()
  (matching-paren ?))
  (matching-paren ?a)
  (matching-paren ?{)) "#,
    );
}

#[test]
fn divergence_syntax_ppss() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "(foo \"bar\\\"baz\" quux)")
  (let ((ppss (parse-partial-sexp 1 21)))
    (list (nth 0 ppss)
          (nth 3 ppss)
          (nth 8 ppss)))) "#,
    );
}
