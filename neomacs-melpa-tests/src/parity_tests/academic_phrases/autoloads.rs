use expect_test::expect;

use super::assert_academic_phrases_autoload_parity;

#[test]
fn academic_phrases_fresh_autoload_defines_only_public_commands_and_prefixes() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (featurep
                'academic-phrases-autoloads)
               (boundp
                'academic-phrases--all-phrases)
               (fboundp
                'academic-phrases)
               (autoloadp
                (symbol-function
                 'academic-phrases))
               (fboundp
                'academic-phrases-by-section)
               (autoloadp
                (symbol-function
                 'academic-phrases-by-section))
               (gethash
                "academic-phrases--"
                definition-prefixes))"##;
    let expect = expect![[r#"OK (nil t nil t t t t ("academic-phrases" "academic-phrases"))"#]];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_topic_command_has_exact_interactive_object() {
    let elisp_form = r##"(list
               (copy-tree
                (symbol-function
                 'academic-phrases))
               (interactive-form
                'academic-phrases)
               (symbol-file
                'academic-phrases
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "academic-phrases" "Insert a phrase from a list of academic phrases by topic." t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/academic-phrases/20180723.1021/home/.emacs.d/elpa/academic-phrases-20180723.1021/academic-phrases.el")"#
    ]];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_section_command_has_exact_interactive_object() {
    let elisp_form = r##"(list
               (copy-tree
                (symbol-function
                 'academic-phrases-by-section))
               (interactive-form
                'academic-phrases-by-section)
               (symbol-file
                'academic-phrases-by-section
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "academic-phrases" "Insert a phrase from a list of academic phrases by the paper section." t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/academic-phrases/20180723.1021/home/.emacs.d/elpa/academic-phrases-20180723.1021/academic-phrases.el")"#
    ]];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_ht_get_star() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--ht-get*)
               (symbol-function
                'academic-phrases--ht-get*)
               (symbol-file
                'academic-phrases--ht-get*
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_ht_select_keys() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--ht-select-keys)
               (symbol-function
                'academic-phrases--ht-select-keys)
               (symbol-file
                'academic-phrases--ht-select-keys
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_replace_placeholders() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--replace-placeholders)
               (symbol-function
                'academic-phrases--replace-placeholders)
               (symbol-file
                'academic-phrases--replace-placeholders
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_prompt_categories() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--prompt-categories)
               (symbol-function
                'academic-phrases--prompt-categories)
               (symbol-file
                'academic-phrases--prompt-categories
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_prompt_items() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--prompt-items)
               (symbol-function
                'academic-phrases--prompt-items)
               (symbol-file
                'academic-phrases--prompt-items
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_filter_item() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--filter-item)
               (symbol-function
                'academic-phrases--filter-item)
               (symbol-file
                'academic-phrases--filter-item
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_get_cat() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--get-cat)
               (symbol-function
                'academic-phrases--get-cat)
               (symbol-file
                'academic-phrases--get-cat
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_get_items() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--get-items)
               (symbol-function
                'academic-phrases--get-items)
               (symbol-file
                'academic-phrases--get-items
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_gen_cats_keywords() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--gen-cats-keywords)
               (symbol-function
                'academic-phrases--gen-cats-keywords)
               (symbol-file
                'academic-phrases--gen-cats-keywords
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_insert() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--insert)
               (symbol-function
                'academic-phrases--insert)
               (symbol-file
                'academic-phrases--insert
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_fresh_autoload_does_not_define_insert_by_section() {
    let elisp_form = r##"(list
               (featurep
                'academic-phrases)
               (fboundp
                'academic-phrases--insert-by-section)
               (symbol-function
                'academic-phrases--insert-by-section)
               (symbol-file
                'academic-phrases--insert-by-section
                'defun))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_academic_phrases_autoload_parity(elisp_form, expect);
}
