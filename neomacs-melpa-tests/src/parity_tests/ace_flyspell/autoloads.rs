use expect_test::expect;

use super::assert_ace_flyspell_autoload_parity;

#[test]
fn ace_flyspell_jump_word_autoload_object_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-flyspell-jump-word)
               (interactive-form
                'ace-flyspell-jump-word)
               (symbol-file
                'ace-flyspell-jump-word
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-flyspell" nil t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-flyspell/20170309.509/home/.emacs.d/elpa/ace-flyspell-20170309.509/ace-flyspell.el")"#
    ]];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_dwim_autoload_object_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-flyspell-dwim)
               (interactive-form
                'ace-flyspell-dwim)
               (symbol-file
                'ace-flyspell-dwim
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-flyspell" nil t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-flyspell/20170309.509/home/.emacs.d/elpa/ace-flyspell-20170309.509/ace-flyspell.el")"#
    ]];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_setup_autoload_object_documentation_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-flyspell-setup)
               (interactive-form
                'ace-flyspell-setup)
               (documentation
                'ace-flyspell-setup
                t)
               (symbol-file
                'ace-flyspell-setup
                'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-flyspell" "Set up default keybindings." t nil) (interactive nil) "Set up default keybindings." "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-flyspell/20170309.509/home/.emacs.d/elpa/ace-flyspell-20170309.509/ace-flyspell.el")"#
    ]];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_autoload_prefix_registry_representation_and_values_match_gnu() {
    let elisp_form = r##"(list
               (featurep
                'ace-flyspell-autoloads)
               (featurep
                'ace-flyspell)
               (hash-table-p
                definition-prefixes)
               (if
                   (hash-table-p
                    definition-prefixes)
                   (gethash
                    "ace-flyspell-"
                    definition-prefixes)
                 (cdr
                  (assq
                   'ace-flyspell
                   definition-prefixes))))"##;
    let expect = expect![[r#"OK (t nil t ("ace-flyspell" "ace-flyspell"))"#]];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_correct_word_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell-correct-word)
               (symbol-file
                'ace-flyspell-correct-word
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_default_handler_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell-default-handler)
               (symbol-file
                'ace-flyspell-default-handler
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_collect_candidates_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell--collect-candidates)
               (symbol-file
                'ace-flyspell--collect-candidates
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_has_overlay_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell--has-flyspell-overlay-p)
               (symbol-file
                'ace-flyspell--has-flyspell-overlay-p
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_help_default_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell-help-default)
               (symbol-file
                'ace-flyspell-help-default
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_auto_correct_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell--auto-correct-word)
               (symbol-file
                'ace-flyspell--auto-correct-word
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_insert_word_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell--insert-word)
               (symbol-file
                'ace-flyspell--insert-word
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_reset_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell--reset)
               (symbol-file
                'ace-flyspell--reset
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_avy_word_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp
                'ace-flyspell--avy-word)
               (symbol-file
                'ace-flyspell--avy-word
                'defun))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_background_face_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (facep
                'ace-flyspell--background)
               (get
                'ace-flyspell--background
                'face-defface-spec)
               (symbol-file
                'ace-flyspell--background
                'defface))"##;
    let expect = expect!["OK (nil nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_group_metadata_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (get
                'ace-flyspell
                'group-documentation)
               (get
                'ace-flyspell
                'custom-group))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_new_word_no_query_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp
                'ace-flyspell-new-word-no-query)
               (symbol-file
                'ace-flyspell-new-word-no-query
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_handler_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp
                'ace-flyspell-handler)
               (symbol-file
                'ace-flyspell-handler
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_current_word_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp
                'ace-flyspell--current-word)
               (symbol-file
                'ace-flyspell--current-word
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_overlay_constant_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp
                'ace-flyspell--ov)
               (symbol-file
                'ace-flyspell--ov
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];

    assert_ace_flyspell_autoload_parity(elisp_form, expect);
}
