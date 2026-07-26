use super::{assert_ace_jump_helm_line_parity, assert_ace_jump_helm_line_with_prelude_parity};
use expect_test::expect;

#[test]
fn ace_jump_helm_line_keys_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-keys
               (documentation-property
                'ace-jump-helm-line-keys
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-keys)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-keys
                 'defvar)))"##;
    let expect =
        expect![[r#"OK (nil "Keys used for `ace-jump-helm-line'." t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_style_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-style
               (documentation-property
                'ace-jump-helm-line-style
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-style)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-style
                 'defvar)))"##;
    let expect =
        expect![[r#"OK (nil "Style used for `ace-jump-helm-line'." t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_background_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-background
               (documentation-property
                'ace-jump-helm-line-background
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-background)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-background
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil "Use background or not in `ace-jump-helm-line'." t "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_obsolete_style_flag_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-use-avy-style
               (documentation-property
                'ace-jump-helm-line-use-avy-style
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-use-avy-style)
               (copy-tree
                (get
                 'ace-jump-helm-line-use-avy-style
                 'byte-obsolete-variable))
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-use-avy-style
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (t "Useless variable since v0.4.\nPlease set `ace-jump-helm-line-keys', `ace-jump-helm-line-style'\nand `ace-jump-helm-line-background' instead." t (nil nil "0.4") "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_persistent_key_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-persistent-key
               (documentation-property
                'ace-jump-helm-line-persistent-key
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-persistent-key)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-persistent-key
                 'defvar)))"##;
    let expect =
        expect![[r#"OK (nil "The key to perform persistent action." t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_select_key_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-select-key
               (documentation-property
                'ace-jump-helm-line-select-key
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-select-key)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-select-key
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil "The key to select.\nUsed for `ace-jump-helm-line'." t "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_move_only_key_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-move-only-key
               (documentation-property
                'ace-jump-helm-line-move-only-key
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-move-only-key)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-move-only-key
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil "The key to only move the selection.\n Used for `ace-jump-helm-line-and-select'." t "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_default_action_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-default-action
               (documentation-property
                'ace-jump-helm-line-default-action
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-default-action)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-default-action
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil "The default action when jumping to a candidate." t "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_idle_delay_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-idle-delay
               (documentation-property
                'ace-jump-helm-line-idle-delay
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-idle-delay)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-idle-delay
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (1 "The delay to trigger automatic `ace-jump-helm-line'." t "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_autoshow_linum_default_documentation_specialness_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line-autoshow-use-linum
               (documentation-property
                'ace-jump-helm-line-autoshow-use-linum
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line-autoshow-use-linum)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-autoshow-use-linum
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil "Whether showing the line hints using `linum-mode' or not." t "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_tree_leafs_internal_default_documentation_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line--tree-leafs
               (documentation-property
                'ace-jump-helm-line--tree-leafs
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line--tree-leafs)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line--tree-leafs
                 'defvar)))"##;
    let expect = expect![[r#"OK (nil nil t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_original_linum_format_captures_the_loaded_linum_default() {
    let elisp_form = r##"(list
               ace-jump-helm-line--original-linum-format
               linum-format
               (equal
                ace-jump-helm-line--original-linum-format
                linum-format)
               (documentation-property
                'ace-jump-helm-line--original-linum-format
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line--original-linum-format)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line--original-linum-format
                 'defvar)))"##;
    let expect = expect![[r#"OK (dynamic dynamic t nil t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_action_type_internal_default_documentation_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line--action-type
               (documentation-property
                'ace-jump-helm-line--action-type
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line--action-type)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line--action-type
                 'defvar)))"##;
    let expect = expect![[r#"OK (nil nil t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_last_window_start_internal_default_documentation_and_source_match() {
    let elisp_form = r##"(list
               ace-jump-helm-line--last-win-start
               (documentation-property
                'ace-jump-helm-line--last-win-start
                'variable-documentation
                t)
               (special-variable-p
                'ace-jump-helm-line--last-win-start)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line--last-win-start
                 'defvar)))"##;
    let expect = expect![[r#"OK (-1 nil t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_load_preserves_a_non_nil_prebound_original_linum_format() {
    let prelude = r##"(setq
               ace-jump-helm-line--original-linum-format
               'prebound-format)"##;
    let elisp_form = r##"(list
               ace-jump-helm-line--original-linum-format
               linum-format)"##;
    let expect = expect!["OK (prebound-format dynamic)"];
    assert_ace_jump_helm_line_with_prelude_parity(prelude, elisp_form, expect);
}
