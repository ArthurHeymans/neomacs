use expect_test::expect;

use super::assert_ace_jump_buffer_parity;

#[test]
fn ace_jump_buffer_header_advice_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ajb/bs--show-header--around
                t)
               (interactive-form
                'ajb/bs--show-header--around)
               (documentation
                'ajb/bs--show-header--around
                t)
               (file-name-nondirectory
                (symbol-file
                 'ajb/bs--show-header--around
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((oldfun) nil "Don't show the `bs' header when doing `ace-jump-buffer'." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_advice_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ajb/bs-set-configuration--after
                t)
               (interactive-form
                'ajb/bs-set-configuration--after)
               (documentation
                'ajb/bs-set-configuration--after
                t)
               (file-name-nondirectory
                (symbol-file
                 'ajb/bs-set-configuration--after
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((name) nil "Set `bs-buffer-sort-function' to the value of `ajb-sort-function'." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_recentf_sort_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'bs--sort-by-recentf t)
               (interactive-form 'bs--sort-by-recentf)
               (documentation 'bs--sort-by-recentf t)
               (file-name-nondirectory
                (symbol-file 'bs--sort-by-recentf 'defun)))"##;
    let expect = expect![[
        r#"OK ((b1 b2) nil "Sort function for comparing buffers `B1' and `B2' by recentf order." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_select_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ajb/select-buffer t)
               (interactive-form 'ajb/select-buffer)
               (documentation 'ajb/select-buffer t)
               (file-name-nondirectory
                (symbol-file 'ajb/select-buffer 'defun)))"##;
    let expect = expect![[
        r#"OK (nil nil "On the end of ace jump, select the buffer at the current line." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_kill_menu_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ajb/kill-bs-menu t)
               (interactive-form 'ajb/kill-bs-menu)
               (documentation 'ajb/kill-bs-menu t)
               (file-name-nondirectory
                (symbol-file 'ajb/kill-bs-menu 'defun)))"##;
    let expect = expect![[
        r#"OK (nil nil "Exit and kill the `bs' window on an invalid character." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_exit_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ajb/exit t)
               (interactive-form 'ajb/exit)
               (documentation 'ajb/exit t)
               (file-name-nondirectory
                (symbol-file 'ajb/exit 'defun)))"##;
    let expect = expect![[
        r#"OK ((_char) nil "Exit and kill the `bs' window on an invalid character, throw done message." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_goto_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ajb/goto-line-and-buffer
                t)
               (interactive-form
                'ajb/goto-line-and-buffer)
               (documentation
                'ajb/goto-line-and-buffer
                t)
               (file-name-nondirectory
                (symbol-file
                 'ajb/goto-line-and-buffer
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Goto visible line below the cursor and visit the associated buffer." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_primary_command_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-jump-buffer t)
               (interactive-form 'ace-jump-buffer)
               (documentation 'ace-jump-buffer t)
               (file-name-nondirectory
                (symbol-file 'ace-jump-buffer 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Quickly hop to buffer with `avy'." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_other_window_command_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-jump-buffer-other-window
                t)
               (interactive-form
                'ace-jump-buffer-other-window)
               (documentation
                'ace-jump-buffer-other-window
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-buffer-other-window
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Quickly hop to buffer with `avy' in other window." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_one_window_command_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-jump-buffer-in-one-window
                t)
               (interactive-form
                'ace-jump-buffer-in-one-window)
               (documentation
                'ace-jump-buffer-in-one-window
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-buffer-in-one-window
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Quickly hop to buffer with `avy' in one window." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_command_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-jump-buffer-with-configuration
                t)
               (interactive-form
                'ace-jump-buffer-with-configuration)
               (documentation
                'ace-jump-buffer-with-configuration
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-buffer-with-configuration
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Quickly hop to buffer with `avy' with selected configuration." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generator_macro_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (macrop 'make-ace-jump-buffer-function)
               (help-function-arglist
                'make-ace-jump-buffer-function
                t)
               (interactive-form
                'make-ace-jump-buffer-function)
               (get
                'make-ace-jump-buffer-function
                'lisp-indent-function)
               (documentation
                'make-ace-jump-buffer-function
                t)
               (file-name-nondirectory
                (symbol-file
                 'make-ace-jump-buffer-function
                 'defun)))"##;
    let expect = expect![[
        r#"OK (t (name &rest buffer-list-reject-filter) nil 1 "Create a `bs-configuration' and interactive defun using `NAME'.\n\nIt will displays buffers that don't get rejected by the body of\n`BUFFER-LIST-REJECT-FILTER'." "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generated_same_mode_filter_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ajb/filter-same-mode-buffers
                t)
               (interactive-form
                'ajb/filter-same-mode-buffers)
               (documentation
                'ajb/filter-same-mode-buffers
                t)
               (file-name-nondirectory
                (symbol-file
                 'ajb/filter-same-mode-buffers
                 'defun)))"##;
    let expect = expect![[r#"OK ((buffer) nil nil "ace-jump-buffer.el")"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generated_same_mode_command_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-jump-same-mode-buffers
                t)
               (interactive-form
                'ace-jump-same-mode-buffers)
               (documentation
                'ace-jump-same-mode-buffers
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-same-mode-buffers
                 'defun)))"##;
    let expect = expect![[r#"OK (nil (interactive nil) nil "ace-jump-buffer.el")"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_load_time_advice_membership_matches() {
    let elisp_form = r##"(list
               (not
                (null
                 (advice-member-p
                  'ajb/bs--show-header--around
                  'bs--show-header)))
               (not
                (null
                 (advice-member-p
                  'ajb/bs-set-configuration--after
                  'bs-set-configuration))))"##;
    let expect = expect!["OK (t t)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((source
                     (symbol-file
                      'ace-jump-buffer
                      'defun))
                    (directory
                     (file-name-directory source))
                    (files
                     '("ace-jump-buffer.el"
                       "ace-jump-buffer-pkg.el"
                       "ace-jump-buffer-autoloads.el"
                       "README-elpa")))
               (mapcar
                (lambda (file)
                  (let ((path
                         (expand-file-name
                          file
                          directory)))
                    (list
                     file
                     (file-attribute-size
                      (file-attributes path))
                     (with-temp-buffer
                       (set-buffer-multibyte nil)
                       (insert-file-contents-literally path)
                       (secure-hash 'sha256
                                    (current-buffer))))))
                files))"##;
    let expect = expect![[
        r#"OK (("ace-jump-buffer.el" 7421 "041e7dfba0341c878a0b446240ed7a7af2485b5104401e58841646795507c1fd") ("ace-jump-buffer-pkg.el" 438 "86e237c2b3ec5869daf3e9c59099b1a2c8b8c5925d0ba6a77f1ed983b8e31088") ("ace-jump-buffer-autoloads.el" 1443 "b24783a010fe1a515ec03f9c87c4a9e81d6aee2ca68a5aea66fd4cb4e74c4ace") ("README-elpa" 80 "b2f4c8fd261f1b7c116fd5417823cb55c6aa8dde6b6d237b8ec97477c32256b9"))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}
