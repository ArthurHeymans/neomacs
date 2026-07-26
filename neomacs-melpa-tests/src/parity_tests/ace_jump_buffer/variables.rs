use expect_test::expect;

use super::assert_ace_jump_buffer_parity;

#[test]
fn ace_jump_buffer_showing_flag_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ajb/showing
               (get 'ajb/showing 'variable-documentation)
               (special-variable-p 'ajb/showing)
               (file-name-nondirectory
                (symbol-file 'ajb/showing 'defvar)))"##;
    let expect = expect![[r#"OK (nil nil t "ace-jump-buffer.el")"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_other_window_flag_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ajb/other-window
               (get 'ajb/other-window 'variable-documentation)
               (special-variable-p 'ajb/other-window)
               (file-name-nondirectory
                (symbol-file 'ajb/other-window 'defvar)))"##;
    let expect = expect![[r#"OK (nil nil t "ace-jump-buffer.el")"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_one_window_flag_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ajb/in-one-window
               (get 'ajb/in-one-window 'variable-documentation)
               (special-variable-p 'ajb/in-one-window)
               (file-name-nondirectory
                (symbol-file 'ajb/in-one-window 'defvar)))"##;
    let expect = expect![[r#"OK (nil nil t "ace-jump-buffer.el")"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_history_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ajb/configuration-history
               (get 'ajb/configuration-history 'variable-documentation)
               (special-variable-p 'ajb/configuration-history)
               (file-name-nondirectory
                (symbol-file 'ajb/configuration-history 'defvar)))"##;
    let expect = expect![[r#"OK (nil nil t "ace-jump-buffer.el")"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_bs_attributes_exact_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ajb/bs-attributes-list
               (get 'ajb/bs-attributes-list 'variable-documentation)
               (special-variable-p 'ajb/bs-attributes-list)
               (file-name-nondirectory
                (symbol-file 'ajb/bs-attributes-list 'defvar)))"##;
    let expect = expect![[
        r#"OK ((("" 2 2 left " ") ("" 1 1 left bs--get-marked-string) ("" 1 1 left " ") ("Buffer" bs--get-name-length 10 left bs--get-name)) nil t "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_dynamic_flags_nest_and_restore_independently() {
    let elisp_form = r##"(let ((ajb/showing 'outer-showing)
                   (ajb/other-window 'outer-other)
                   (ajb/in-one-window 'outer-one)
                   inside)
               (let ((ajb/showing t)
                     (ajb/other-window nil)
                     (ajb/in-one-window t))
                 (setq inside
                       (list
                        ajb/showing
                        ajb/other-window
                        ajb/in-one-window)))
               (list
                inside
                ajb/showing
                ajb/other-window
                ajb/in-one-window))"##;
    let expect = expect!["OK ((t nil t) outer-showing outer-other outer-one)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}
