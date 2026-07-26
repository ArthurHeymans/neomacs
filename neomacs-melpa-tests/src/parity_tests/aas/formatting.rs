use expect_test::expect;

use super::{assert_aas_parity, assert_aas_signal_parity};

#[test]
fn aas_format_doc_prefers_variable_documentation_and_replaces_both_quote_styles() {
    let elisp_form = r##"(progn
               (put
                'neomacs-aas-documented
                'variable-documentation
                "Variable uses `alpha' and 'plain'.")
               (fset
                'neomacs-aas-documented
                (lambda ()
                  "Function uses `ignored'."
                  nil))
               (list
                (aas--format-doc-to-org
                 'neomacs-aas-documented)
                (progn
                  (put
                   'neomacs-aas-documented
                   'variable-documentation
                   nil)
                  (aas--format-doc-to-org
                   'neomacs-aas-documented))))"##;
    let expect =
        expect![[r#"OK ("Variable uses ~alpha~ and ~plain~." "Function uses ‘ignored’.")"#]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_format_snippet_array_escapes_org_syntax_uses_one_shot_descriptions_and_ignores_conditions() {
    let elisp_form = r##"(aas--format-snippet-array
              '(:cond beginning-of-line-p
                :expansion-desc "First description"
                "a b|~" "FIRST"
                "plain" "SECOND"
                :expansion-desc nil
                :cond end-of-line-p
                "third" (yas "BODY")
                "disabled" nil))"##;
    let expect = expect![[
        r#"OK (("~a␣b❘∽~" "First description") ("~plain~" "SECOND") ("~third~" (yas "BODY")) ("~disabled~" nil))"#
    ]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_format_snippet_array_is_non_destructive_and_preserves_function_expansions() {
    let elisp_form = r##"(let* ((source
                     '("fn" forward-char
                       "text" "value"))
                    (copy
                     (copy-tree source))
                    (result
                     (aas--format-snippet-array
                      source)))
               (list
                result
                source
                (equal source copy)))"##;
    let expect = expect![[
        r#"OK ((("~fn~" forward-char) ("~text~" "value")) ("fn" forward-char "text" "value") t)"#
    ]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_format_snippet_array_rejects_unknown_keywords_with_exact_error() {
    let elisp_form = r##"(aas--format-snippet-array
              '(:unknown "value"
                "key" "expansion"))"##;
    let expect = expect![[r#"ERR (error "Unknown keyword: :unknown")"#]];

    assert_aas_signal_parity(elisp_form, expect);
}
