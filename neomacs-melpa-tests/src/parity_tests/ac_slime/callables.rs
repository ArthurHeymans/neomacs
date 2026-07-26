use expect_test::expect;

use super::assert_ac_slime_parity;

#[test]
fn ac_slime_fuzzy_candidates_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-source-slime-fuzzy-candidates
                t)
               (interactive-form
                'ac-source-slime-fuzzy-candidates)
               (documentation
                'ac-source-slime-fuzzy-candidates
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-source-slime-fuzzy-candidates
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil nil "Return a possibly-empty list of fuzzy completions for the symbol at point." "ac-slime.el")"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_simple_candidates_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-source-slime-simple-candidates
                t)
               (interactive-form
                'ac-source-slime-simple-candidates)
               (documentation
                'ac-source-slime-simple-candidates
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-source-slime-simple-candidates
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil nil "Return a possibly-empty list of completions for the symbol at point." "ac-slime.el")"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_case_correcting_completions_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-source-slime-case-correcting-completions
                t)
               (interactive-form
                'ac-source-slime-case-correcting-completions)
               (documentation
                'ac-source-slime-case-correcting-completions
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-source-slime-case-correcting-completions
                 'defun)))"##;
    let expect = expect![[r#"OK ((name collection) nil nil "ac-slime.el")"#]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_documentation_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-slime-documentation
                t)
               (interactive-form
                'ac-slime-documentation)
               (documentation
                'ac-slime-documentation
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-slime-documentation
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((symbol-name) nil "Return a documentation string for SYMBOL-NAME." "ac-slime.el")"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_init_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ac-slime-init
                t)
               (interactive-form
                'ac-slime-init)
               (documentation
                'ac-slime-init
                t)
               (file-name-nondirectory
                (symbol-file
                 'ac-slime-init
                 'defun)))"##;
    let expect =
        expect![[r#"OK (nil nil "Called when completion source is initialized." "ac-slime.el")"#]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_setup_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'set-up-slime-ac
                t)
               (interactive-form
                'set-up-slime-ac)
               (documentation
                'set-up-slime-ac
                t)
               (file-name-nondirectory
                (symbol-file
                 'set-up-slime-ac
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((&optional fuzzy) (interactive nil) "Add an optionally FUZZY slime completion source to `ac-sources'." "ac-slime.el")"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}
