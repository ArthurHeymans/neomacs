use expect_test::expect;

use super::{
    assert_ace_jump_buffer_autoload_parity, assert_ace_jump_buffer_autoload_with_prelude_parity,
};

#[test]
fn ace_jump_buffer_primary_command_autoload_object_documentation_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function 'ace-jump-buffer)
               (symbol-file 'ace-jump-buffer 'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-jump-buffer" "Quickly hop to buffer with `avy'." t nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-buffer/20171031.1550/home/.emacs.d/elpa/ace-jump-buffer-20171031.1550/ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_other_window_command_autoload_object_documentation_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-jump-buffer-other-window)
               (symbol-file
                'ace-jump-buffer-other-window
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-jump-buffer" "Quickly hop to buffer with `avy' in other window." t nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-buffer/20171031.1550/home/.emacs.d/elpa/ace-jump-buffer-20171031.1550/ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_one_window_command_autoload_object_documentation_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-jump-buffer-in-one-window)
               (symbol-file
                'ace-jump-buffer-in-one-window
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-jump-buffer" "Quickly hop to buffer with `avy' in one window." t nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-buffer/20171031.1550/home/.emacs.d/elpa/ace-jump-buffer-20171031.1550/ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_command_autoload_object_documentation_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-jump-buffer-with-configuration)
               (symbol-file
                'ace-jump-buffer-with-configuration
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-jump-buffer" "Quickly hop to buffer with `avy' with selected configuration." t nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-buffer/20171031.1550/home/.emacs.d/elpa/ace-jump-buffer-20171031.1550/ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generator_macro_autoload_object_documentation_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'make-ace-jump-buffer-function)
               (macrop
                'make-ace-jump-buffer-function)
               (symbol-file
                'make-ace-jump-buffer-function
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-jump-buffer" "Create a `bs-configuration' and interactive defun using `NAME'.\n\nIt will displays buffers that don't get rejected by the body of\n`BUFFER-LIST-REJECT-FILTER'.\n\n(fn NAME &rest BUFFER-LIST-REJECT-FILTER)" nil t) (t) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-buffer/20171031.1550/home/.emacs.d/elpa/ace-jump-buffer-20171031.1550/ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_autoload_feature_and_prefix_registry_match() {
    let elisp_form = r##"(list
               (featurep 'ace-jump-buffer-autoloads)
               (featurep 'ace-jump-buffer)
               (hash-table-p definition-prefixes)
               (if (hash-table-p definition-prefixes)
                   (gethash
                    "ajb"
                    definition-prefixes)
                 (cdr
                  (assq
                   'ajb
                   definition-prefixes)))
               (if (hash-table-p definition-prefixes)
                   (gethash
                    "bs--sort-by-recentf"
                    definition-prefixes)
                 (cdr
                  (assq
                   'bs--sort-by-recentf
                   definition-prefixes))))"##;
    let expect = expect![[
        r#"OK (t nil t ("ace-jump-buffer" "ace-jump-buffer") ("ace-jump-buffer" "ace-jump-buffer"))"#
    ]];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_header_advice_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/bs--show-header--around)
               (symbol-file
                'ajb/bs--show-header--around
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_advice_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/bs-set-configuration--after)
               (symbol-file
                'ajb/bs-set-configuration--after
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_recentf_sort_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'bs--sort-by-recentf)
               (symbol-file 'bs--sort-by-recentf 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_select_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/select-buffer)
               (symbol-file 'ajb/select-buffer 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_kill_menu_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/kill-bs-menu)
               (symbol-file 'ajb/kill-bs-menu 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_exit_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/exit)
               (symbol-file 'ajb/exit 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_goto_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/goto-line-and-buffer)
               (symbol-file
                'ajb/goto-line-and-buffer
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_same_mode_filter_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/filter-same-mode-buffers)
               (symbol-file
                'ajb/filter-same-mode-buffers
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_same_mode_command_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-jump-same-mode-buffers)
               (symbol-file
                'ace-jump-same-mode-buffers
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_perspective_filter_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/filter-persp-buffers)
               (symbol-file
                'ajb/filter-persp-buffers
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_perspective_command_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-jump-persp-buffers)
               (symbol-file
                'ace-jump-persp-buffers
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_projectile_filter_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ajb/filter-projectile-buffers)
               (symbol-file
                'ajb/filter-projectile-buffers
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_projectile_command_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-jump-projectile-buffers)
               (symbol-file
                'ace-jump-projectile-buffers
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_autoloads_do_not_attach_advice_or_register_generated_configurations() {
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
                  'bs-set-configuration)))
               (assoc "same-mode" bs-configurations))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ace_jump_buffer_autoload_with_prelude_parity("(require 'bs)", elisp_form, expect);
}

#[test]
fn ace_jump_buffer_max_window_height_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb-max-window-height)
               (symbol-file
                'ajb-max-window-height
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_sort_function_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb-sort-function)
               (symbol-file 'ajb-sort-function 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_bs_configuration_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb-bs-configuration)
               (symbol-file
                'ajb-bs-configuration
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_style_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb-style)
               (symbol-file 'ajb-style 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_showing_flag_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb/showing)
               (symbol-file 'ajb/showing 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_other_window_flag_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb/other-window)
               (symbol-file 'ajb/other-window 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_one_window_flag_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb/in-one-window)
               (symbol-file 'ajb/in-one-window 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_configuration_history_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb/configuration-history)
               (symbol-file
                'ajb/configuration-history
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_bs_attributes_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ajb/bs-attributes-list)
               (symbol-file
                'ajb/bs-attributes-list
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_face_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (facep 'ajb-face)
               (get 'ajb-face 'face-defface-spec)
               (symbol-file 'ajb-face 'defface))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_group_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (get
                'ace-jump-buffer
                'group-documentation)
               (get
                'ace-jump-buffer
                'custom-group))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_buffer_autoload_parity(elisp_form, expect);
}
