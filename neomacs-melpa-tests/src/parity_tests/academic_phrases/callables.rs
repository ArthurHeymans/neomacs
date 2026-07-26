use expect_test::expect;

use super::assert_academic_phrases_parity;

#[test]
fn academic_phrases_ht_get_star_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--ht-get*
                t)
               (interactive-form
                'academic-phrases--ht-get*)
               (documentation
                'academic-phrases--ht-get*
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--ht-get*
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((table &rest keys) nil "Starting with TABLE, Look up KEYS in nested hash tables.\nThe lookup for each key should return another hash table, except\nfor the final key, which may return any value." "academic-phrases.el")"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_ht_select_keys_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--ht-select-keys
                t)
               (interactive-form
                'academic-phrases--ht-select-keys)
               (documentation
                'academic-phrases--ht-select-keys
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--ht-select-keys
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((table keys) nil "Return a copy of TABLE with only the specified KEYS." "academic-phrases.el")"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_replace_placeholders_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--replace-placeholders
                t)
               (interactive-form
                'academic-phrases--replace-placeholders)
               (documentation
                'academic-phrases--replace-placeholders
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--replace-placeholders
                 'defun)))"##;
    let expect = expect![[r#"OK ((tmp choices) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_prompt_categories_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--prompt-categories
                t)
               (interactive-form
                'academic-phrases--prompt-categories)
               (documentation
                'academic-phrases--prompt-categories
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--prompt-categories
                 'defun)))"##;
    let expect = expect![[r#"OK ((phrases) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_prompt_items_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--prompt-items
                t)
               (interactive-form
                'academic-phrases--prompt-items)
               (documentation
                'academic-phrases--prompt-items
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--prompt-items
                 'defun)))"##;
    let expect = expect![[r#"OK ((cat &optional phrases) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_filter_item_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--filter-item
                t)
               (interactive-form
                'academic-phrases--filter-item)
               (documentation
                'academic-phrases--filter-item
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--filter-item
                 'defun)))"##;
    let expect = expect![[r#"OK ((cat id &optional phrases) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_get_cat_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--get-cat
                t)
               (interactive-form
                'academic-phrases--get-cat)
               (documentation
                'academic-phrases--get-cat
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--get-cat
                 'defun)))"##;
    let expect = expect![[r#"OK ((res &optional phrases) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_get_items_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--get-items
                t)
               (interactive-form
                'academic-phrases--get-items)
               (documentation
                'academic-phrases--get-items
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--get-items
                 'defun)))"##;
    let expect = expect![[r#"OK ((cat &optional phrases) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_gen_cats_keywords_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--gen-cats-keywords
                t)
               (interactive-form
                'academic-phrases--gen-cats-keywords)
               (documentation
                'academic-phrases--gen-cats-keywords
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--gen-cats-keywords
                 'defun)))"##;
    let expect = expect![[r#"OK ((s e) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_insert_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--insert
                t)
               (interactive-form
                'academic-phrases--insert)
               (documentation
                'academic-phrases--insert
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--insert
                 'defun)))"##;
    let expect = expect![[r#"OK ((phrases) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_insert_by_section_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases--insert-by-section
                t)
               (interactive-form
                'academic-phrases--insert-by-section)
               (documentation
                'academic-phrases--insert-by-section
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases--insert-by-section
                 'defun)))"##;
    let expect = expect![[r#"OK ((section &optional phrases) nil nil "academic-phrases.el")"#]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_topic_command_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases
                t)
               (interactive-form
                'academic-phrases)
               (documentation
                'academic-phrases
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Insert a phrase from a list of academic phrases by topic." "academic-phrases.el")"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_section_command_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist
                'academic-phrases-by-section
                t)
               (interactive-form
                'academic-phrases-by-section)
               (documentation
                'academic-phrases-by-section
                t)
               (file-name-nondirectory
                (symbol-file
                 'academic-phrases-by-section
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Insert a phrase from a list of academic phrases by the paper section." "academic-phrases.el")"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}
