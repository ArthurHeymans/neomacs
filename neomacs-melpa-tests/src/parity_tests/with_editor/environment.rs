use expect_test::expect;

use super::{assert_with_editor_parity, assert_with_editor_signal_parity};

#[test]
fn with_editor_macro_scopes_editor_to_sleeping_fallback_and_restores_environment() {
    let elisp_form = r##"(let ((process-environment
                    (cons "EDITOR=original"
                          (copy-sequence process-environment)))
                   (with-editor-emacsclient-executable nil))
               (let ((inside
                      (with-editor
                        (list
                         (getenv "EDITOR")
                         (getenv "ALTERNATE_EDITOR")
                         with-editor--envvar))))
                 (list inside
                       (getenv "EDITOR")
                       (getenv "ALTERNATE_EDITOR")
                       with-editor--envvar)))"##;
    let expect = expect![[
        r#"OK (("sh -c 'printf \"\\nWITH-EDITOR: $$ OPEN $0\\037$1\\037 IN $(pwd)\\n\"; sleep 604800 & sleep=$!; trap \"kill $sleep; exit 0\" USR1; trap \"kill $sleep; exit 1\" USR2; wait $sleep'" nil "EDITOR") "original" nil nil)"#
    ]];

    assert_with_editor_parity(elisp_form, expect);
}

#[test]
fn with_editor_literal_and_dynamic_macros_set_only_requested_environment_variable() {
    let elisp_form = r##"(let ((process-environment
                    (copy-sequence process-environment))
                   (with-editor-emacsclient-executable nil)
                   (name "HG_EDITOR"))
               (setenv "EDITOR" "outer-editor")
               (setenv "GIT_EDITOR" "outer-git")
               (setenv "HG_EDITOR" "outer-hg")
               (list
                (with-editor "GIT_EDITOR"
                  (list (getenv "EDITOR")
                        (getenv "GIT_EDITOR")
                        with-editor--envvar))
                (with-editor* name
                  (list (getenv "EDITOR")
                        (getenv "HG_EDITOR")
                        with-editor--envvar))
                (list (getenv "EDITOR")
                      (getenv "GIT_EDITOR")
                      (getenv "HG_EDITOR"))))"##;
    let expect = expect![[
        r#"OK (("outer-editor" "sh -c 'printf \"\\nWITH-EDITOR: $$ OPEN $0\\037$1\\037 IN $(pwd)\\n\"; sleep 604800 & sleep=$!; trap \"kill $sleep; exit 0\" USR1; trap \"kill $sleep; exit 1\" USR2; wait $sleep'" "GIT_EDITOR") ("outer-editor" "sh -c 'printf \"\\nWITH-EDITOR: $$ OPEN $0\\037$1\\037 IN $(pwd)\\n\"; sleep 604800 & sleep=$!; trap \"kill $sleep; exit 0\" USR1; trap \"kill $sleep; exit 1\" USR2; wait $sleep'" "HG_EDITOR") ("outer-editor" "outer-git" "outer-hg"))"#
    ]];

    assert_with_editor_parity(elisp_form, expect);
}

#[test]
fn with_editor_server_window_uses_first_matching_rule_then_fallback() {
    let elisp_form = r##"(let ((with-editor-server-window-alist
                    '(("\\.git/" . git-window)
                      ("COMMIT_EDITMSG\\'" . commit-window)))
                   (server-window 'fallback-window))
               (with-temp-buffer
                 (setq buffer-file-name
                       "/repo/.git/COMMIT_EDITMSG")
                 (let ((first (with-editor-server-window)))
                   (setq buffer-file-name "/repo/notes.txt")
                   (list first
                         (with-editor-server-window)))))"##;
    let expect = expect![[r#"OK (git-window fallback-window)"#]];

    assert_with_editor_parity(elisp_form, expect);
}

#[test]
fn with_editor_export_editor_rejects_unsupported_major_mode() {
    let elisp_form = r##"(with-temp-buffer
               (fundamental-mode)
               (with-editor-export-editor "EDITOR"))"##;
    let expect = expect![[r#"ERR (error "Cannot export environment variables in this buffer")"#]];

    assert_with_editor_signal_parity(elisp_form, expect);
}
