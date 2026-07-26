use expect_test::expect;

use super::assert_ac_sly_parity;

#[test]
fn ac_sly_fuzzy_candidates_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-source-sly-fuzzy-candidates
                t)
               (interactive-form
                'ac-source-sly-fuzzy-candidates)
               (documentation
                'ac-source-sly-fuzzy-candidates
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-source-sly-fuzzy-candidates
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil nil "Return a possibly-empty list of fuzzy completions for the symbol at point." "ac-sly.el")"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_simple_candidates_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-source-sly-simple-candidates
                t)
               (interactive-form
                'ac-source-sly-simple-candidates)
               (documentation
                'ac-source-sly-simple-candidates
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-source-sly-simple-candidates
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil nil "Return a possibly-empty list of completions for the symbol at point." "ac-sly.el")"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_case_correcting_completions_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-source-sly-case-correcting-completions
                t)
               (interactive-form
                'ac-source-sly-case-correcting-completions)
               (documentation
                'ac-source-sly-case-correcting-completions
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-source-sly-case-correcting-completions
                 'defun)))"##;
    let expect = expect![[r#"OK ((name collection) nil nil "ac-sly.el")"#]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_documentation_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-sly-documentation
                t)
               (interactive-form
                'ac-sly-documentation)
               (documentation
                'ac-sly-documentation
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-sly-documentation
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((symbol-name) nil "Return a documentation string for SYMBOL-NAME." "ac-sly.el")"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_init_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-sly-init
                t)
               (interactive-form
                'ac-sly-init)
               (documentation
                'ac-sly-init
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-sly-init
                 'defun)))"##;
    let expect =
        expect![[r#"OK (nil nil "Called when completion source is initialized." "ac-sly.el")"#]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_setup_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'set-up-sly-ac
                t)
               (interactive-form
                'set-up-sly-ac)
               (documentation
                'set-up-sly-ac
                t)
               (file-name-nondirectory
                (symbol-file
                 'set-up-sly-ac
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((&optional fuzzy) (interactive nil) "Add an optionally-fuzzy slime completion source to `ac-sources'." "ac-sly.el")"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}
