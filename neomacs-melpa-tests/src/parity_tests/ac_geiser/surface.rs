use expect_test::expect;

use super::assert_ac_geiser_parity;

#[test]
fn ac_geiser_exact_pin_dependencies_features_private_api_and_source_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-geiser package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-geiser geiser auto-complete))
                (mapcar
                 #'functionp
                 '(geiser-repl--live-p
                   geiser-completion--complete
                   geiser-doc--get-docstring
                   geiser-autodoc--str*))
                ac-source-geiser
                (get 'ac-source-geiser
                     'variable-documentation)))"##;
    let expect = expect![[
        r#"OK (ac-geiser "20200318.824" ((geiser (0 5)) (auto-complete (1 4))) (t t t) (nil nil nil nil) ((candidates . ac-source-geiser-candidates) (symbol . "g") (document . ac-geiser-documentation)) "Source for geiser completion")"#
    ]];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_function_arities_interactive_forms_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)))
               '(ac-source-geiser-candidates
                 ac-geiser-documentation
                 ac-geiser-setup))"##;
    let expect = expect![[
        r#"OK ((ac-source-geiser-candidates nil nil "Return a possibly-empty list of completions for the symbol at point.") (ac-geiser-documentation (symbol-name) nil nil) (ac-geiser-setup nil (interactive nil) "Add the geiser completion source to the front of `ac-sources'.\nThis affects only the current buffer."))"#
    ]];

    assert_ac_geiser_parity(elisp_form, expect);
}

#[test]
fn ac_geiser_source_uses_live_function_symbols_and_exact_popup_marker() {
    let elisp_form = r##"(let ((candidate-entry
                    (assq
                     'candidates
                     ac-source-geiser))
                   (document-entry
                    (assq
                     'document
                     ac-source-geiser))
                   (symbol-entry
                    (assq
                     'symbol
                     ac-source-geiser)))
               (list
                candidate-entry
                (functionp
                 (cdr candidate-entry))
                document-entry
                (functionp
                 (cdr document-entry))
                symbol-entry
                (length ac-source-geiser)))"##;
    let expect = expect![[
        r#"OK ((candidates . ac-source-geiser-candidates) t (document . ac-geiser-documentation) t (symbol . "g") 3)"#
    ]];

    assert_ac_geiser_parity(elisp_form, expect);
}
