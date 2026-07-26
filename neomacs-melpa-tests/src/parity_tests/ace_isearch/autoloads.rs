use expect_test::expect;

use super::assert_ace_isearch_autoload_parity;

#[test]
fn ace_isearch_jump_during_autoload_object_documentation_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function 'ace-isearch-jump-during-isearch)
               (symbol-file 'ace-isearch-jump-during-isearch 'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-isearch" "Jump to one of the current isearch candidates." t nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el")"#
    ]];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_minor_mode_autoload_object_documentation_variable_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function 'ace-isearch-mode)
               (boundp 'ace-isearch-mode)
               (symbol-file 'ace-isearch-mode 'defun))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-isearch" "Minor-mode that combines isearch, ace-jump-mode, avy, helm-swoop and swiper seamlessly.\n\nThis is a minor mode.  If called interactively, toggle the `Ace-Isearch\nmode' mode.  If the prefix argument is positive, enable the mode, and if\nit is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `ace-isearch-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" t nil) nil "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el")"#
    ]];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_global_mode_autoload_object_variable_properties_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function 'global-ace-isearch-mode)
               global-ace-isearch-mode
               (get 'global-ace-isearch-mode 'globalized-minor-mode)
               (get 'global-ace-isearch-mode 'custom-autoload)
               (get 'global-ace-isearch-mode 'variable-documentation)
               (symbol-file 'global-ace-isearch-mode 'defun)
               (symbol-file 'global-ace-isearch-mode 'defvar))"##;
    let expect = expect![[
        r#"OK ((autoload "ace-isearch" "Toggle Ace-Isearch mode in many buffers.\nSpecifically, Ace-Isearch mode is enabled in all buffers where\n`ace-isearch--turn-on' would do it.\n\nWith prefix ARG, enable Global Ace-Isearch mode if ARG is positive;\notherwise, disable it.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.\nEnable the mode if ARG is nil, omitted, or is a positive number.\nDisable the mode if ARG is a negative number.\n\nSee `ace-isearch-mode' for more information on Ace-Isearch mode.\n\n(fn &optional ARG)" t nil) nil t t "Non-nil if Global Ace-Isearch mode is enabled.\nSee the `global-ace-isearch-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `global-ace-isearch-mode'." "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch-autoloads.el")"#
    ]];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_autoload_feature_and_prefix_registry_match() {
    let elisp_form = r##"(list
               (featurep 'ace-isearch-autoloads)
               (featurep 'ace-isearch)
               (hash-table-p definition-prefixes)
               (if (hash-table-p definition-prefixes)
                   (gethash "ace-isearch-" definition-prefixes)
                 (cdr (assq 'ace-isearch definition-prefixes))))"##;
    let expect = expect![[r#"OK (t nil t ("ace-isearch" "ace-isearch"))"#]];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch--jumper-function)
               (symbol-file 'ace-isearch--jumper-function 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_pop_mark_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-pop-mark)
               (symbol-file 'ace-isearch-pop-mark 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_function_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-function)
               (symbol-file 'ace-isearch-function 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_group_metadata_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (get 'ace-isearch 'group-documentation)
               (get 'ace-isearch 'custom-group))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}
