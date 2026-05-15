//! Oracle parity tests for GNU Lisp syntax parse state APIs.
//!
//! GNU Emacs implements `parse-partial-sexp` and `scan-sexps` in
//! `src/syntax.c`, with `syntax-ppss` in `lisp/emacs-lisp/syntax.el`
//! adding cache and syntax-propertize behavior on top.  These tests compare
//! full parser state values against GNU rather than asserting simplified
//! expectations locally.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_parse_partial_sexp_lisp_string_comment_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'lisp-mode)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(alpha \"str ; not comment\" ; real comment\n  (beta gamma))")
    (let ((inside-string (save-excursion
                           (goto-char (point-min))
                           (search-forward "not")
                           (point)))
          (inside-comment (save-excursion
                            (goto-char (point-min))
                            (search-forward "real")
                            (point)))
          (inside-inner-list (save-excursion
                               (goto-char (point-min))
                               (search-forward "beta")
                               (point))))
      (list
       (parse-partial-sexp (point-min) (point-max))
       (syntax-ppss inside-string)
       (syntax-ppss inside-comment)
       (syntax-ppss inside-inner-list)
       (point)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_parse_partial_sexp_targetdepth_stopbefore_commentstop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'lisp-mode)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(outer (inner one) ; comment here\n  \"string\" tail)")
    (let ((end (point-max)))
      (list
       (save-excursion
         (parse-partial-sexp (point-min) end 1))
       (point)
       (save-excursion
         (parse-partial-sexp (point-min) end nil t))
       (point)
       (save-excursion
         (parse-partial-sexp (point-min) end nil nil nil t))
       (point)
       (save-excursion
         (parse-partial-sexp (point-min) end nil nil nil 'syntax-table))
       (point)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_syntax_ppss_cache_flush_after_buffer_mutation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'lisp-mode)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(message \"hello\")\n")
    (let ((initial (syntax-ppss (point-max))))
      (goto-char (point-min))
      (insert ";; leading comment\n")
      (let ((after-insert (syntax-ppss (point-max))))
        (delete-region (point-min) (line-beginning-position 2))
        (list initial
              after-insert
              (syntax-ppss (point-max)))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_scan_sexps_comments_and_unbalanced_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'lisp-mode)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(a (b c) ; ignored\n   \"d e\") tail (unterminated")
    (let ((parse-sexp-ignore-comments t)
          (from (point-min)))
      (list
       (scan-sexps from 1)
       (scan-sexps from 2)
       (scan-sexps (point-max) -1)
       (condition-case err
           (scan-sexps (save-excursion
                         (goto-char (point-min))
                         (search-forward "unterminated")
                         (match-beginning 0))
                       1)
         (error (list (car err) (cadr err))))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
