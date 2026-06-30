//! Strict combo oracle probes, batch 311: parse-partial-sexp state variants,
//! further characterizing the batch-147 last-complete-start divergence.
//! States inside string, inside comment, at various nesting depths, and
//! after quote/backquote/unquote.
//! Uses assert_oracle_parity_expect format.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_parse_partial_sexp_string_comment_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(defun foo ()\n  \"doc\"\n  ;; a comment\n  (bar \"str\" (nested)))\n")
  (let ((probe (lambda (target)
                 (save-excursion
                   (goto-char (point-min))
                   (search-forward target)
                   (parse-partial-sexp (point-min) (point))))))
    (list (funcall probe "doc")
          (funcall probe "a comment")
          (funcall probe "str")
          (funcall probe "nested")
          (funcall probe ")))"))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_parse_partial_sexp_nesting_depth_quote_backquote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(let ((x 1)) (foo 'sym `(bar ,baz) (qux)))")
  (let ((probe (lambda (target)
                 (save-excursion
                   (goto-char (point-min))
                   (search-forward target)
                   (parse-partial-sexp (point-min) (point))))))
    (list (funcall probe "x 1")
          (funcall probe "'sym")
          (funcall probe ",baz")
          (funcall probe "qux"))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_syntax_ppss_cache_invalidation_across_edit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(with-temp-buffer
  (emacs-lisp-mode)
  (insert "(foo bar)")
  (let ((ppss1 (syntax-ppss 4)))
    (insert "(baz ")
    (let ((ppss2 (syntax-ppss 7)))
      (list (nth 0 ppss1)
            (nth 0 ppss2)
            (nth 1 ppss2)
            (nth 3 ppss2)))))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
