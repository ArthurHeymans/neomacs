use expect_test::expect;

use super::{assert_anki_editor_autoload_parity, assert_anki_editor_parity};

#[test]
fn generated_autoload_exposes_minor_mode_and_transient_entry_without_loading_features() {
    let elisp_form = r##"(list
                      (featurep 'anki-editor)
                      (featurep 'anki-editor-ui)
                      (mapcar
                       (lambda (function)
                         (list
                          function
                          (fboundp function)
                          (and
                           (fboundp function)
                           (autoloadp
                            (symbol-function function)))
                          (and
                           (fboundp function)
                           (commandp function))))
                       '(anki-editor-mode
                         anki-editor-ui
                         anki-editor-push-notes
                         anki-editor-note-at-point))
                      (locate-library "anki-editor")
                      (locate-library "anki-editor-ui"))"##;
    let expect = expect![[
        r#"OK (nil nil ((anki-editor-mode t t t) (anki-editor-ui t t t) (anki-editor-push-notes nil nil nil) (anki-editor-note-at-point nil nil nil)) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/anki-editor/20260714.1156/home/.emacs.d/elpa/anki-editor-20260714.1156/anki-editor.el" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/anki-editor/20260714.1156/home/.emacs.d/elpa/anki-editor-20260714.1156/anki-editor-ui.el")"#
    ]];
    assert_anki_editor_autoload_parity(elisp_form, expect);
}

#[test]
fn invoking_mode_autoload_loads_main_source_and_rejects_non_org_buffer_before_network() {
    let elisp_form = r##"(let ((before
                           (featurep 'anki-editor)))
                      (list
                       before
                       (with-temp-buffer
                         (condition-case error-data
                             (anki-editor-mode 1)
                           (error error-data)))
                       (featurep 'anki-editor)
                       (featurep 'anki-editor-ui)
                       (autoloadp
                        (symbol-function
                         'anki-editor-ui))))"##;
    let expect =
        expect![[r#"OK (nil (user-error "anki-editor only works in org-mode buffers") t nil t)"#]];
    assert_anki_editor_autoload_parity(elisp_form, expect);
}

#[test]
fn installed_package_descriptor_and_source_files_match_exact_melpa_transaction() {
    let elisp_form = r##"(let ((descriptor
                           (cadr
                            (assq
                             'anki-editor
                             package-alist))))
                      (list
                       (package-desc-name descriptor)
                       (package-version-join
                        (package-desc-version
                         descriptor))
                       (package-desc-reqs descriptor)
                       (package-desc-kind descriptor)
                       (file-name-nondirectory
                        (locate-library
                         "anki-editor"))
                       (file-name-nondirectory
                        (locate-library
                         "anki-editor-ui"))))"##;
    let expect = expect![[
        r#"OK (anki-editor "20260714.1156" ((emacs (29 1))) nil "anki-editor.el" "anki-editor-ui.el")"#
    ]];
    assert_anki_editor_parity(elisp_form, expect);
}
