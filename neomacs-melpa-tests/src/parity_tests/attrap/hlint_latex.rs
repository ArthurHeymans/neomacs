use expect_test::expect;

use super::assert_attrap_parity;

#[test]
fn attrap_hlint_fixer_removes_unused_pragmas_and_redundant_dollar_operators_with_reported_padding()
{
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-hlint-fixer
           "Use fewer LANGUAGE pragmas\nPerhaps you should remove it."
           "«POINT»{-# LANGUAGE OverloadedStrings #-}«END»\n\nmain = pure ()\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-hlint-fixer
           "Redundant $"
           "result = render «POINT»$«END» value\n"
           0))"##;
    let expect = expect![[
        r#"OK ((((kill-unused t)) "{-# LANGUAGE OverloadedStrings #-}\n\nmain = pure ()\n" ((:ok nil) "main = pure ()\n" 1)) (((kill-dollar t)) "result = render $ value\n" ((:ok nil) "result = render value\n" 17)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_hlint_fixer_removes_only_the_outer_redundant_brackets() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-hlint-fixer
          "Redundant bracket"
          "result = «POINT»(map (transform value) inputs)«END»\n"
          0)"##;
    let expect = expect![[
        r#"OK (((kill-brackets t)) "result = (map (transform value) inputs)\n" ((:ok nil) "result = map (transform value) inputs)" 10))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_hlint_fixer_replaces_a_multiline_found_snippet_with_the_collapsed_suggestion() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-hlint-fixer
          "Found:\n  map (\\x -> transform x)\n      values\nPerhaps:\n  map transform\n      values\nNote: increases sharing\n  [haskell-hlint]"
          "result = «POINT»map (\\x -> transform x)\n  values«END»\nnext = 1\n"
          0)"##;
    let expect = expect![[
        r#"OK (((replace-as-hinted t)) "result = map (\\x -> transform x)\n  values\nnext = 1\n" ((:ok nil) "result = map transform valuesnext = 1\n" 30))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_hlint_fixer_returns_no_repairs_for_unrecognized_advice() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-hlint-fixer
          "Use camelCase"
          "«POINT»snake_case«END» = 1\n"
          nil)"##;
    let expect = expect![[r#"OK (nil "snake_case = 1\n" nil)"#]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_latex_fixer_adds_an_empty_argument_and_performs_the_immediate_ellipsis_rewrite() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-LaTeX-fixer
           "Command terminated with space"
           "Use \\LaTeX«POINT» in prose.\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-LaTeX-fixer
           "You should use \\ldots to achieve an ellipsis."
           "Wait«POINT»... what happened?\n"
           nil))"##;
    let expect = expect![[
        r#"OK ((((add-empty-argument t)) "Use \\LaTeX in prose.\n" ((:ok nil) "Use \\LaTeX{} in prose.\n" 13)) (nil "Wait\\ldots what happened?\n" nil))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_latex_fixer_offers_distinct_opening_and_closing_double_quote_repairs() {
    let elisp_form = r##"(mapcar
          (lambda (option)
            (attrap-test-run-fixer-option
             'attrap-LaTeX-fixer
             "Use either `` or '' as an alternative to `\"'."
             "He said «POINT»\"quoted text\" in prose.\n"
             option))
          '(0 1))"##;
    let expect = expect![[
        r#"OK ((((fix-open-dquote t) (fix-close-dquote t)) "He said \"quoted text\" in prose.\n" ((:ok nil) "He said ``quoted text\" in prose.\n" 11)) (((fix-open-dquote t) (fix-close-dquote t)) "He said \"quoted text\" in prose.\n" ((:ok nil) "He said ''quoted text\" in prose.\n" 11)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_latex_fixer_normalizes_inline_and_line_break_whitespace_to_non_breaking_space() {
    let elisp_form = r##"(mapcar
          (lambda (contents)
            (attrap-test-run-fixer-option
             'attrap-LaTeX-fixer
             "Non-breaking space (`~') should have been used."
             contents
             0))
          '("See Section«POINT» 12 for details.\n"
            "See Section\n   «POINT»\\ref{details} for details.\n"))"##;
    let expect = expect![[
        r#"OK ((((non-breaking-space t)) "See Section 12 for details.\n" ((:ok nil) "See Section~12 for details.\n" 13)) (((non-breaking-space t)) "See Section\n   \\ref{details} for details.\n" ((:ok nil) "See Section~\\ref{details} for details.\n" 13)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_latex_fixer_replaces_interword_spacing_and_repairs_inline_or_line_leading_pagerefs() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-LaTeX-fixer
           "Interword spacing (`\\ ') should perhaps be used."
           "Dr.«POINT» Smith wrote this.\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-LaTeX-fixer
           "Delete this space to maintain correct pagereferences."
           "See page«POINT»   \\pageref{target}.\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-LaTeX-fixer
           "Delete this space to maintain correct pagereferences."
           "See the appendix.\n   «POINT»\\pageref{target} starts here.\n"
           0))"##;
    let expect = expect![[
        r#"OK ((((use-interword-spacing t)) "Dr. Smith wrote this.\n" ((:ok nil) "Dr.\\ Smith wrote this.\n" 6)) (((fix-space-pageref t)) "See page   \\pageref{target}.\n" ((:ok nil) "See page\\pageref{target}.\n" 9)) (((fix-space-pageref t)) "See the appendix.\n   \\pageref{target} starts here.\n" ((:ok nil) "See the appendix.%\n   \\pageref{target} starts here.\n" 19)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_latex_fixer_returns_no_repairs_for_an_unrecognized_warning() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-LaTeX-fixer
          "Overfull \\hbox detected"
          "«POINT»A very long paragraph.\n"
          nil)"##;
    let expect = expect![[r#"OK (nil "A very long paragraph.\n" nil)"#]];

    assert_attrap_parity(elisp_form, expect);
}
