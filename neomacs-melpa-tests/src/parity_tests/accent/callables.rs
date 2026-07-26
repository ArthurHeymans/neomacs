use expect_test::expect;

use super::assert_accent_parity;

#[test]
fn accent_list_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'accent-lst
                t)
               (interactive-form
                'accent-lst)
               (documentation
                'accent-lst
                t)
               (file-name-nondirectory
                (symbol-file
                 'accent-lst
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil nil "Merge `accent-custom` with default accenter characters." "accent.el")"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_menu_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'accent-menu
                t)
               (interactive-form
                'accent-menu)
               (documentation
                'accent-menu
                t)
               (file-name-nondirectory
                (symbol-file
                 'accent-menu
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Display a popup completion menu with accents, if current character is matching." "accent.el")"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_company_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'accent-company
                t)
               (interactive-form
                'accent-company)
               (documentation
                'accent-company
                t)
               (file-name-nondirectory
                (symbol-file
                 'accent-company
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((command &rest _ignored) (interactive (list 'interactive)) "Use `company' to display a completion menu for accented characters at point.\n\nSee `company-backends' for the description of COMMAND." "accent.el")"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

#[test]
fn accent_corfu_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'accent-corfu
                t)
               (interactive-form
                'accent-corfu)
               (documentation
                'accent-corfu
                t)
               (file-name-nondirectory
                (symbol-file
                 'accent-corfu
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Use `corfu' to display a completion menu for accented characters at point." "accent.el")"#
    ]];

    assert_accent_parity(elisp_form, expect);
}
