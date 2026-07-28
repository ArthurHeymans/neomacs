use expect_test::expect;

use super::assert_attrap_parity;

#[test]
fn attrap_elisp_fixer_inserts_real_package_and_section_headers_at_the_diagnostic_line() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (apply
             #'attrap-test-run-fixer-option
             'attrap-elisp-fixer
             (append case '(0))))
          '(("The first line should be of the form: \";;; demo.el --- Demonstration package\""
             ";; stale header\n«POINT»(provide 'demo)\n")
            ("You should have a section marked \";;; Commentary:\""
             ";;; demo.el --- Demonstration package\n\n«POINT»This package repairs diagnostics.\n")))"##;
    let expect = expect![[
        r#"OK ((((insert-package t)) ";; stale header\n(provide 'demo)\n" ((:ok nil) ";; stale header\n;;; demo.el --- Demonstration package\n(provide 'demo)\n" 55)) (((insert-section-header t)) ";;; demo.el --- Demonstration package\n\nThis package repairs diagnostics.\n" ((:ok nil) ";;; demo.el --- Demonstration package\n\n;;; Commentary:\nThis package repairs diagnostics.\n" 56)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_elisp_fixer_quotes_the_reported_symbol_without_touching_similar_text() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-elisp-fixer
          "Lisp symbol ‘foo-bar’ should appear in quotes"
          "«POINT»(message \"Use foo-bar before foo-bar-baz\")\n"
          0)"##;
    let expect = expect![[
        r#"OK (((kill-message-period t)) "(message \"Use foo-bar before foo-bar-baz\")\n" ((:ok nil) "(message \"Use `foo-bar' before foo-bar-baz\")\n" 24))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_elisp_fixer_repairs_error_punctuation_and_emacs_capitalization_case_sensitively() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (apply
             #'attrap-test-run-fixer-option
             'attrap-elisp-fixer
             (append case '(0))))
          '(("Error messages should *not* end with a period"
             "«POINT»(error \"Bad input.\")\n")
            ("Name emacs should appear capitalized as Emacs"
             "«POINT»Use emacs with Emacs-compatible files.\n")
            ("The word widget should be capitalized"
             "prefix «POINT»widget remains lower elsewhere: widget\n")))"##;
    let expect = expect![[
        r#"OK ((((kill-message-period t)) "(error \"Bad input.\")\n" ((:ok nil) "(error \"Bad input\")\n" 19)) (((capitalize-emacs t)) "Use emacs with Emacs-compatible files.\n" ((:ok nil) "Use Emacs with Emacs-compatible files.\n" 10)) (((capitalize t)) "prefix widget remains lower elsewhere: widget\n" ((:ok nil) "prefix Widget remains lower elsewhere: widget\n" 14)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_elisp_fixer_removes_mixed_trailing_whitespace_and_adds_exact_sentence_spacing() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (apply
             #'attrap-test-run-fixer-option
             'attrap-elisp-fixer
             (append case '(0))))
          '(("White space found at end of line"
             "prefix\n«POINT»payload \t  \nnext\n")
            ("There should be two spaces after a period"
             "prefix\n«POINT»First sentence. Second sentence.  Third.\n")))"##;
    let expect = expect![[
        r#"OK ((((delete-trailing-space t)) "prefix\npayload \11  \nnext\n" ((:ok nil) "prefix\npayload\nnext\n" 15)) (((add-space t)) "prefix\nFirst sentence. Second sentence.  Third.\n" ((:ok nil) "prefix\nFirst sentence.  Second sentence.  Third.\n" 24)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_elisp_fixer_inserts_empty_documentation_for_both_checkdoc_messages() {
    let elisp_form = r##"(mapcar
          (lambda (message)
            (attrap-test-run-fixer-option
             'attrap-elisp-fixer
             message
             "(defun demo (value)\n«POINT»  (+ value 1))\n"
             0))
          '("This function might as well have a documentation string"
            "The function demo should have documentation"))"##;
    let expect = expect![[
        r#"OK ((((add-empty-doc t)) "(defun demo (value)\n  (+ value 1))\n" ((:ok nil) "(defun demo (value)\n  \"\"\n  (+ value 1))\n" 26)) (((add-empty-doc t)) "(defun demo (value)\n  (+ value 1))\n" ((:ok nil) "(defun demo (value)\n  \"\"\n  (+ value 1))\n" 26)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_elisp_fixer_decodes_and_appends_the_exact_multiline_footer() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-elisp-fixer
          "The footer should be: ;;; demo.el ends here\\n;; Local Variables:\\n;; coding: utf-8\\n;; End:"
          "(provide 'demo)«POINT»\n"
          0)"##;
    let expect = expect![[
        r#"OK (((add-footer t)) "(provide 'demo)\n" ((:ok nil) "(provide 'demo)\n;;; demo.el ends here\n;; Local Variables:\n;; coding: utf-8\n;; End:\n" 83))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_elisp_fixer_merges_an_incomplete_summary_and_adds_missing_punctuation() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (apply
             #'attrap-test-run-fixer-option
             'attrap-elisp-fixer
             (append case '(0))))
          '(("First line is not a complete sentence"
             "«POINT»First summary fragment\n continued description.\n")
            ("First sentence should end with punctuation"
             "Summary without punctuation«POINT»\nLong description follows.\n")))"##;
    let expect = expect![[
        r#"OK ((((merge-lines t)) "First summary fragment\n continued description.\n" ((:ok nil) "First summary fragment continued description.\n" 23)) (((add-punctuation t)) "Summary without punctuation\nLong description follows.\n" ((:ok nil) "Summary without punctuation.\nLong description follows.\n" 29)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_elisp_fixer_returns_all_matching_repairs_in_source_order_and_none_for_unknown_warnings() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-elisp-fixer
           "Name emacs should appear capitalized as Emacs; this word should be capitalized"
           "«POINT»emacs package\n"
           nil)
          (attrap-test-run-fixer-option
           'attrap-elisp-fixer
           "Completely unrelated byte compiler warning"
           "«POINT»(setq value 1)\n"
           nil))"##;
    let expect = expect![[
        r#"OK ((((capitalize-emacs t) (capitalize t)) "emacs package\n" nil) (nil "(setq value 1)\n" nil))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}
