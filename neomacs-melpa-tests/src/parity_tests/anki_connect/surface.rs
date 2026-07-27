use expect_test::expect;

use super::assert_anki_connect_parity;

#[test]
fn package_constant_feature_and_complete_callable_surface_match() {
    let elisp_form = r##"(list
                      (featurep 'anki-connect)
                      (list
                       (boundp 'anki-connect-url)
                       anki-connect-url
                       (documentation-property
                        'anki-connect-url
                        'variable-documentation))
                      (mapcar
                       (lambda (function)
                         (list
                          function
                          (fboundp function)
                          (help-function-arglist
                           function t)
                          (commandp function)))
                       '(anki-connect-request
                         anki-connect-deck-names
                         anki-connect-deck-exists-p
                         anki-connect-create-deck
                         anki-connect-ensure-deck
                         anki-connect-model-names
                         anki-connect-model-field-names
                         anki-connect-add-note
                         anki-connect-update-note)))"##;
    let expect = expect![[
        r#"OK (t (t "http://127.0.0.1:8765" "URL for anki-connect.") ((anki-connect-request t (action params) nil) (anki-connect-deck-names t nil nil) (anki-connect-deck-exists-p t (deck-name) nil) (anki-connect-create-deck t (deck-name) nil) (anki-connect-ensure-deck t (deck-name) nil) (anki-connect-model-names t nil nil) (anki-connect-model-field-names t (model) nil) (anki-connect-add-note t (deck model field-alist &optional audio) nil) (anki-connect-update-note t (id field-alist &optional tags) nil)))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn source_declares_no_s_dependency_and_recursive_deck_workflow_exposes_missing_runtime_functions() {
    let elisp_form = r##"(let ((descriptor
                           (cadr
                            (assq
                             'anki-connect
                             package-alist))))
                      (list
                       (package-desc-reqs descriptor)
                       (featurep 's)
                       (fboundp 's-split)
                       (fboundp 's-blank?)
                       (condition-case error-data
                           (anki-connect-ensure-deck
                            "Languages::Japanese")
                         (error error-data))
                       (featurep 's)))"##;
    let expect = expect!["OK (((emacs (24 3))) nil nil nil (void-function s-split) nil)"];
    assert_anki_connect_parity(elisp_form, expect);
}

#[test]
fn public_functions_preserve_documentation_and_noninteractive_contract() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (documentation function)
                         (interactive-form function)))
                      '(anki-connect-request
                        anki-connect-deck-names
                        anki-connect-deck-exists-p
                        anki-connect-create-deck
                        anki-connect-ensure-deck
                        anki-connect-model-names
                        anki-connect-model-field-names
                        anki-connect-add-note
                        anki-connect-update-note))"##;
    let expect = expect![[
        r#"OK ((anki-connect-request "Commuicate with anki-connect.\n\nACTION describe the action.\nPARAMS should be an alist." nil) (anki-connect-deck-names "List decks." nil) (anki-connect-deck-exists-p "Check if deck exists." nil) (anki-connect-create-deck "Create deck with hierarchy." nil) (anki-connect-ensure-deck "Create deck hierarchy recursively." nil) (anki-connect-model-names "List models." nil) (anki-connect-model-field-names "List fields in MODEL." nil) (anki-connect-add-note "Add a note to DECK.\n\nMODEL specify the format of the note.\nFIELD-ALIST specify the content of the note.\nAUDIO specify the audio information." nil) (anki-connect-update-note "Modify the note.\n\nFIELD-ALIST specify the content of the note. " nil))"#
    ]];
    assert_anki_connect_parity(elisp_form, expect);
}
