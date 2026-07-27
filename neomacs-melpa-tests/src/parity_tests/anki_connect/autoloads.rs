use expect_test::expect;

use super::{assert_anki_connect_autoload_parity, assert_anki_connect_parity};

#[test]
fn generated_autoload_registers_prefix_without_loading_or_autoloading_public_functions() {
    let elisp_form = r##"(let ((functions
                           '(anki-connect-request
                             anki-connect-deck-names
                             anki-connect-deck-exists-p
                             anki-connect-create-deck
                             anki-connect-ensure-deck
                             anki-connect-model-names
                             anki-connect-model-field-names
                             anki-connect-add-note
                             anki-connect-update-note)))
                      (list
                       (featurep 'anki-connect)
                       (mapcar
                        (lambda (function)
                          (list
                           function
                           (fboundp function)
                           (and
                            (fboundp function)
                            (autoloadp
                             (symbol-function
                              function)))))
                        functions)
                       (locate-library "anki-connect")
                       (assoc
                        (getenv "NEOMACS_PACKAGE_SOURCE")
                        load-history)))"##;
    let expect = expect![[
        r#"OK (nil ((anki-connect-request nil nil) (anki-connect-deck-names nil nil) (anki-connect-deck-exists-p nil nil) (anki-connect-create-deck nil nil) (anki-connect-ensure-deck nil nil) (anki-connect-model-names nil nil) (anki-connect-model-field-names nil nil) (anki-connect-add-note nil nil) (anki-connect-update-note nil nil)) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/anki-connect/20250414.1301/home/.emacs.d/elpa/anki-connect-20250414.1301/anki-connect.el" ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/anki-connect/20250414.1301/home/.emacs.d/elpa/anki-connect-20250414.1301/anki-connect-autoloads.el" (provide . anki-connect-autoloads)))"#
    ]];
    assert_anki_connect_autoload_parity(elisp_form, expect);
}

#[test]
fn loading_installed_source_after_autoloads_materializes_feature_and_complete_api() {
    let elisp_form = r##"(let ((before
                           (featurep 'anki-connect)))
                      (load
                       (locate-library "anki-connect")
                       nil t t)
                      (list
                       before
                       (featurep 'anki-connect)
                       anki-connect-url
                       (mapcar
                        #'fboundp
                        '(anki-connect-request
                          anki-connect-deck-names
                          anki-connect-deck-exists-p
                          anki-connect-create-deck
                          anki-connect-ensure-deck
                          anki-connect-model-names
                          anki-connect-model-field-names
                          anki-connect-add-note
                          anki-connect-update-note))))"##;
    let expect = expect![[r#"OK (nil t "http://127.0.0.1:8765" (t t t t t t t t t))"#]];
    assert_anki_connect_autoload_parity(elisp_form, expect);
}

#[test]
fn reloading_source_preserves_constant_and_feature_but_redefines_function_identity() {
    let elisp_form = r##"(let ((url-before
                           anki-connect-url)
                          (request-before
                           (symbol-function
                            'anki-connect-request)))
                      (load
                       (locate-library "anki-connect")
                       nil t t)
                      (list
                       (eq
                        request-before
                        (symbol-function
                         'anki-connect-request))
                       (equal
                        url-before
                        anki-connect-url)
                       (featurep 'anki-connect)))"##;
    let expect = expect!["OK (nil t t)"];
    assert_anki_connect_parity(elisp_form, expect);
}
