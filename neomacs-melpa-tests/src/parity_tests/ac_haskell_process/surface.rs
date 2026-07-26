use expect_test::expect;

use super::assert_ac_haskell_process_parity;

#[test]
fn ac_haskell_process_exact_pin_dependencies_features_private_api_and_source_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-haskell-process
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-haskell-process
                   auto-complete
                   haskell
                   haskell-process))
                (mapcar
                 #'functionp
                 '(haskell-session-maybe
                   haskell-process
                   haskell-process-get-repl-completions
                   in-string-p
                   popup-tip))
                ac-source-haskell-process
                (get
                 'ac-source-haskell-process
                 'variable-documentation)
                (get
                 'ac-source-haskell-process
                 'risky-local-variable)))"##;
    let expect = expect![[
        r#"OK (ac-haskell-process "20150423.1402" ((auto-complete (1 4)) (haskell-mode (13))) (t t t t) (t t t t t) ((available . ac-haskell-process-available-p) (candidates . ac-haskell-process-candidates) (document . ac-haskell-process-doc) (symbol . "h")) "Haskell auto-complete source which uses the current haskell process." t)"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_function_arities_interactive_forms_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)))
               '(ac-haskell-process-available-p
                 ac-haskell-process-candidates
                 ac-haskell-process-doc
                 ac-haskell-process-setup
                 ac-haskell-process-symbol-start-pos
                 ac-haskell-process-popup-doc))"##;
    let expect = expect![[
        r#"OK ((ac-haskell-process-available-p nil nil "Return non-nil if completions are (or might later be) available from this source.") (ac-haskell-process-candidates nil nil "Return a list of completion candidates for the current `ac-prefix'.") (ac-haskell-process-doc (sym) nil "Return the docstring for SYM.") (ac-haskell-process-setup nil (interactive nil) "Add the haskell process completion source to the front of `ac-sources'.\nThis affects only the current buffer.") (ac-haskell-process-symbol-start-pos nil nil "Find the starting position of the symbol at point, unless inside a string.") (ac-haskell-process-popup-doc nil (interactive nil) "Show documentation for the symbol at point in a popup."))"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}

#[test]
fn ac_haskell_process_source_uses_live_callbacks_and_exact_popup_marker() {
    let elisp_form = r##"(mapcar
               (lambda (entry)
                 (list
                  entry
                  (if
                      (stringp
                       (cdr entry))
                      'string
                    (functionp
                     (cdr entry)))))
               ac-source-haskell-process)"##;
    let expect = expect![[
        r#"OK (((available . ac-haskell-process-available-p) t) ((candidates . ac-haskell-process-candidates) t) ((document . ac-haskell-process-doc) t) ((symbol . "h") string))"#
    ]];

    assert_ac_haskell_process_parity(elisp_form, expect);
}
