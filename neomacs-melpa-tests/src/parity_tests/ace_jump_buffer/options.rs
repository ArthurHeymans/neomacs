use expect_test::expect;

use super::assert_ace_jump_buffer_parity;

#[test]
fn ace_jump_buffer_max_window_height_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ajb-max-window-height
               (get 'ajb-max-window-height 'custom-type)
               (get 'ajb-max-window-height 'variable-documentation)
               (get 'ajb-max-window-height 'standard-value)
               (assq
                'ajb-max-window-height
                (get 'ace-jump-buffer 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ajb-max-window-height 'defvar)))"##;
    let expect = expect![[
        r#"OK (20 integer "Maximal window height of Ace Jump Buffer Selection Menu." ((funcall #'#[nil (20) (t)])) (ajb-max-window-height custom-variable) "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_sort_function_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ajb-sort-function
               (get 'ajb-sort-function 'custom-type)
               (get 'ajb-sort-function 'variable-documentation)
               (get 'ajb-sort-function 'standard-value)
               (assq
                'ajb-sort-function
                (get 'ace-jump-buffer 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ajb-sort-function 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil (radio (const :tag "No custom sorting" nil) (function-item bs--sort-by-recentf) (function-item bs--sort-by-name) (function-item bs--sort-by-size) (function-item bs--sort-by-filename) (function-item bs--sort-by-mode) (function :tag "Other function")) "The `bs-sort-function' function used when displaying `ace-jump-buffer'." ((funcall #'#[nil (nil) (t)])) (ajb-sort-function custom-variable) "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_bs_configuration_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ajb-bs-configuration
               (get 'ajb-bs-configuration 'custom-type)
               (get 'ajb-bs-configuration 'variable-documentation)
               (get 'ajb-bs-configuration 'standard-value)
               (assq
                'ajb-bs-configuration
                (get 'ace-jump-buffer 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ajb-bs-configuration 'defvar)))"##;
    let expect = expect![[
        r#"OK ("all" nil "The `bs-configuration' used when displaying `ace-jump-buffer'." ((funcall #'#[nil ("all") (t)])) (ajb-bs-configuration custom-variable) "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_style_default_type_documentation_missing_group_and_source_match() {
    let elisp_form = r##"(list
               ajb-style
               (get 'ajb-style 'custom-type)
               (get 'ajb-style 'variable-documentation)
               (get 'ajb-style 'standard-value)
               (assq
                'ajb-style
                (get 'ace-jump-buffer 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ajb-style 'defvar)))"##;
    let expect = expect![[
        r#"OK (at-full (choice (const :tag "Pre" pre) (const :tag "At" at) (const :tag "At Full" at-full) (const :tag "Post" post) (const :tag "De Bruijn" de-bruijn) (const :tag "Words" words)) "The default method of displaying the overlays for `ace-jump-buffer'." ((funcall #'#[nil ('at-full) (t)])) (ajb-style custom-variable) "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_face_specification_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               (get 'ajb-face 'face-defface-spec)
               (get 'ajb-face 'face-documentation)
               (assq
                'ajb-face
                (get 'ace-jump-buffer 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ajb-face 'defface)))"##;
    let expect = expect![[
        r#"OK (((t :background unspecified :foreground unspecified)) "Customizable face to use within the `ace-jump-buffer' menu. The default is unspecified." (ajb-face custom-face) "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}
