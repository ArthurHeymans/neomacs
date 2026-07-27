use expect_test::expect;

use super::assert_aggressive_fill_paragraph_parity;

#[test]
fn aggressive_fill_paragraph_current_line_tracks_point_across_real_multiline_edits() {
    let elisp_form = r##"(with-temp-buffer
         (insert "first line\nsecond line\nthird")
         (list
          (progn (goto-char 3) (afp-current-line))
          (progn (forward-line 1) (afp-current-line))
          (progn
            (end-of-line)
            (insert " extended")
            (afp-current-line))
          (progn (goto-char (point-max)) (afp-current-line))))"##;
    let expect = expect![[r#"OK ("first line" "second line" "second line extended" "third")"#]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_comment_detection_uses_real_syntax_state_and_boundaries() {
    let elisp_form = r##"(with-temp-buffer
         (emacs-lisp-mode)
         (insert "(message \"not ; comment\") ; actual comment\n(+ 1 2)")
         (list
          (progn
            (goto-char (point-min))
            (search-forward "not")
            (afp-inside-comment?))
          (progn
            (search-forward "actual")
            (afp-inside-comment?))
          (progn
            (goto-char (point-max))
            (afp-inside-comment?))))"##;
    let expect = expect!["OK (nil t nil)"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_repeated_whitespace_distinguishes_spaces_tabs_and_single_space() {
    let elisp_form = r##"(with-temp-buffer
         (insert "a b  c\t d\t\tz")
         (mapcar
          (lambda (needle)
            (goto-char (point-min))
            (search-forward needle)
            (afp-repeated-whitespace?))
          '("a " "b  " "c\t " "d\t\t" "z")))"##;
    let expect = expect!["OK (nil t t t nil)"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_markdown_code_predicate_checks_real_mode_line_and_indent() {
    let elisp_form = r##"(progn
         (unless (fboundp 'markdown-mode)
           (define-derived-mode markdown-mode text-mode "Markdown"))
         (list
          (with-temp-buffer
            (markdown-mode)
            (insert "    let answer = 42;")
            (afp-markdown-inside-code-block?))
          (with-temp-buffer
            (markdown-mode)
            (insert "ordinary prose")
            (afp-markdown-inside-code-block?))
          (with-temp-buffer
            (text-mode)
            (insert "    indented prose")
            (afp-markdown-inside-code-block?))))"##;
    let expect = expect!["OK (0 nil nil)"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_comment_bullet_predicate_handles_markers_indent_and_code() {
    let elisp_form = r##"(with-temp-buffer
         (c++-mode)
         (mapcar
          (lambda (line)
            (erase-buffer)
            (insert line)
            (goto-char (point-max))
            (list line
                  (afp-inside-comment?)
                  (afp-bullet-list-in-comments?)))
          '("// * first item"
            "  // + nested item"
            "// - final item"
            "// ordinary prose"
            "int marker = '*';")))"##;
    let expect = expect![[
        r#"OK (("// * first item" t 0) ("  // + nested item" t 0) ("// - final item" t 0) ("// ordinary prose" t nil) ("int marker = '*';" nil nil))"#
    ]];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_org_table_predicate_handles_table_rows_cells_and_prose() {
    let elisp_form = r##"(with-temp-buffer
         (org-mode)
         (insert "| Name | Value |\n| alpha | 1 |\n\nOrdinary prose")
         (list
          (progn
            (goto-char (point-min))
            (afp-in-org-table?))
          (progn
            (search-forward "alpha")
            (afp-in-org-table?))
          (progn
            (goto-char (point-max))
            (afp-in-org-table?))))"##;
    let expect = expect!["OK (t t nil)"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_org_source_header_predicate_is_case_insensitive_and_mode_scoped() {
    let elisp_form = r##"(let ((lines
                '("#+HEADER: :var x=1"
                  "  #+begin_src emacs-lisp"
                  "#+END_SRC"
                  "#+NAME: example"
                  "#+RESULTS:"
                  "ordinary prose")))
         (list
          (with-temp-buffer
            (org-mode)
            (mapcar
             (lambda (line)
               (erase-buffer)
               (insert line)
               (goto-char (point-max))
               (afp-in-org-src-block-header?))
             lines))
          (with-temp-buffer
            (text-mode)
            (insert "#+BEGIN_SRC emacs-lisp")
            (afp-in-org-src-block-header?))))"##;
    let expect = expect!["OK ((t t t t nil nil) nil)"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}

#[test]
fn aggressive_fill_paragraph_suppression_short_circuits_at_first_truthy_predicate() {
    let elisp_form = r##"(let ((calls nil))
         (let ((afp-suppress-fill-pfunction-list
                (list
                 (lambda () (push 'first calls) nil)
                 (lambda () (push 'second calls) 'stop)
                 (lambda () (push 'third calls) t))))
           (list
            (afp-suppress-fill?)
            (nreverse calls)
            (let ((calls nil))
              (let ((afp-suppress-fill-pfunction-list
                     (list
                      (lambda () (push 'a calls) nil)
                      (lambda () (push 'b calls) nil))))
                (list (afp-suppress-fill?) (nreverse calls)))))))"##;
    let expect = expect!["OK (t (first second) (nil (a b)))"];
    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}
