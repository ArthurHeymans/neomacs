use expect_test::expect;

use super::assert_ac_rtags_parity;

#[test]
fn ac_rtags_trim_whitespace_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-trim-leading-trailing-whitespace
                t)
               (interactive-form
                'ac-rtags-trim-leading-trailing-whitespace)
               (documentation
                'ac-rtags-trim-leading-trailing-whitespace)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-trim-leading-trailing-whitespace
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((argstr) nil "Remove leading trailing whitespaces from ARGSTR." "ac-rtags.el")"#
    ]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_candidates_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-candidates
                t)
               (interactive-form
                'ac-rtags-candidates)
               (documentation
                'ac-rtags-candidates)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-candidates
                 'defun)))"##;
    let expect = expect![[r#"OK (nil nil "Get candidates." "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_document_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-document
                t)
               (interactive-form
                'ac-rtags-document)
               (documentation
                'ac-rtags-document)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-document
                 'defun)))"##;
    let expect = expect![[r#"OK ((item) nil "Get property text from ITEM." "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_action_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-action
                t)
               (interactive-form
                'ac-rtags-action)
               (documentation
                'ac-rtags-action)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-action
                 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_action_function_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-action-function
                t)
               (interactive-form
                'ac-rtags-action-function)
               (documentation
                'ac-rtags-action-function)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-action-function
                 'defun)))"##;
    let expect = expect![[r#"OK ((origtag) nil nil "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_action_namespace_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-action-namespace
                t)
               (interactive-form
                'ac-rtags-action-namespace)
               (documentation
                'ac-rtags-action-namespace)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-action-namespace
                 'defun)))"##;
    let expect = expect![[r#"OK ((_origtag) nil nil "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_prefix_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-prefix
                t)
               (interactive-form
                'ac-rtags-prefix)
               (documentation
                'ac-rtags-prefix)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-prefix
                 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_init_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-init
                t)
               (interactive-form
                'ac-rtags-init)
               (documentation
                'ac-rtags-init)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-init
                 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}

#[test]
fn ac_rtags_completions_hook_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-rtags-completions-hook
                t)
               (interactive-form
                'ac-rtags-completions-hook)
               (documentation
                'ac-rtags-completions-hook)
               (file-name-nondirectory
                (symbol-file
                 'ac-rtags-completions-hook
                 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-rtags.el")"#]];

    assert_ac_rtags_parity(elisp_form, expect);
}
