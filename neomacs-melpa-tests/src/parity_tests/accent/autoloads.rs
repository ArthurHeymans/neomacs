use expect_test::expect;

use super::assert_accent_autoload_parity;

#[test]
fn accent_fresh_autoload_feature_and_prefix_registration_match() {
    let elisp_form = r##"(list
               (featurep
                'accent)
               (featurep
                'accent-autoloads)
               (gethash
                "accent-"
                definition-prefixes))"##;
    let expect = expect![[r#"OK (nil t ("accent" "accent"))"#]];

    assert_accent_autoload_parity(elisp_form, expect);
}

#[test]
fn accent_fresh_autoload_menu_has_exact_interactive_object() {
    let elisp_form = r##"(list
               (copy-tree
                (symbol-function
                 'accent-menu))
               (interactive-form
                'accent-menu)
               (symbol-file
                'accent-menu
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "accent" "Display a popup completion menu with accents, if current character is matching." t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/accent/20250210.906/home/.emacs.d/elpa/accent-20250210.906/accent.el")"#
    ]];

    assert_accent_autoload_parity(elisp_form, expect);
}

#[test]
fn accent_fresh_autoload_company_has_exact_interactive_object() {
    let elisp_form = r##"(list
               (copy-tree
                (symbol-function
                 'accent-company))
               (interactive-form
                'accent-company)
               (symbol-file
                'accent-company
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "accent" "Use `company' to display a completion menu for accented characters at point.\n\nSee `company-backends' for the description of COMMAND.\n\n(fn COMMAND &rest IGNORED)" t nil) (interactive (list 'interactive)) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/accent/20250210.906/home/.emacs.d/elpa/accent-20250210.906/accent.el")"#
    ]];

    assert_accent_autoload_parity(elisp_form, expect);
}

#[test]
fn accent_fresh_autoload_corfu_has_exact_interactive_object() {
    let elisp_form = r##"(list
               (copy-tree
                (symbol-function
                 'accent-corfu))
               (interactive-form
                'accent-corfu)
               (symbol-file
                'accent-corfu
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "accent" "Use `corfu' to display a completion menu for accented characters at point." t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/accent/20250210.906/home/.emacs.d/elpa/accent-20250210.906/accent.el")"#
    ]];

    assert_accent_autoload_parity(elisp_form, expect);
}

#[test]
fn accent_fresh_autoload_does_not_define_internal_list_function() {
    let elisp_form = r##"(list
               (featurep
                'accent)
               (fboundp
                'accent-lst)
               (symbol-function
                'accent-lst)
               (symbol-file
                'accent-lst
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_accent_autoload_parity(elisp_form, expect);
}

#[test]
fn accent_fresh_autoload_leaves_version_options_data_and_custom_group_undefined() {
    let elisp_form = r##"(list
               (boundp
                'accent-version)
               (boundp
                'accent-position)
               (boundp
                'accent-custom)
               (boundp
                'accent-diacritics)
               (get
                'accent
                'group-documentation)
               (get
                'accent
                'custom-loads))"##;
    let expect = expect!["OK (nil nil nil nil nil nil)"];

    assert_accent_autoload_parity(elisp_form, expect);
}
