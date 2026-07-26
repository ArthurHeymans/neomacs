use super::assert_ace_jump_helm_line_autoload_parity;
use expect_test::expect;

#[test]
fn ace_jump_helm_line_fresh_autoload_provides_only_its_autoload_feature() {
    let elisp_form = r##"(list
               (featurep 'ace-jump-helm-line-autoloads)
               (featurep 'ace-jump-helm-line)
               (featurep 'avy)
               (featurep 'helm)
               (featurep 'linum))"##;
    let expect = expect!["OK (t nil nil nil nil)"];
    assert_ace_jump_helm_line_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_primary_commands_have_exact_fresh_autoload_objects() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (let ((definition
                        (symbol-function symbol)))
                   (list
                    symbol
                    (autoloadp definition)
                    (nth 1 definition)
                    (nth 2 definition)
                    (nth 3 definition)
                    (nth 4 definition)
                    (commandp symbol)
                    (symbol-file symbol 'defun))))
               '(ace-jump-helm-line
                 ace-jump-helm-line-and-select))"##;
    let expect = expect![[
        r#"OK ((ace-jump-helm-line t "ace-jump-helm-line" "Jump to a candidate and execute the default action." t nil t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-helm-line/20160918.1836/home/.emacs.d/elpa/ace-jump-helm-line-20160918.1836/ace-jump-helm-line.el") (ace-jump-helm-line-and-select t "ace-jump-helm-line" "Jump to and select the candidate in helm window." t nil t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-helm-line/20160918.1836/home/.emacs.d/elpa/ace-jump-helm-line-20160918.1836/ace-jump-helm-line.el"))"#
    ]];
    assert_ace_jump_helm_line_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_idle_functions_have_noninteractive_fresh_autoload_objects() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (let ((definition
                        (symbol-function symbol)))
                   (list
                    symbol
                    (autoloadp definition)
                    (nth 1 definition)
                    (nth 2 definition)
                    (nth 3 definition)
                    (nth 4 definition)
                    (commandp symbol)
                    (symbol-file symbol 'defun))))
               '(ace-jump-helm-line-idle-exec-add
                 ace-jump-helm-line-idle-exec-remove))"##;
    let expect = expect![[
        r#"OK ((ace-jump-helm-line-idle-exec-add t "ace-jump-helm-line" "\n\n(fn FUNC)" nil nil nil "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-helm-line/20160918.1836/home/.emacs.d/elpa/ace-jump-helm-line-20160918.1836/ace-jump-helm-line.el") (ace-jump-helm-line-idle-exec-remove t "ace-jump-helm-line" "\n\n(fn FUNC)" nil nil nil "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-helm-line/20160918.1836/home/.emacs.d/elpa/ace-jump-helm-line-20160918.1836/ace-jump-helm-line.el"))"#
    ]];
    assert_ace_jump_helm_line_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_execute_action_is_a_fresh_alias_to_select_command() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-jump-helm-line-execute-action)
               (indirect-function
                'ace-jump-helm-line-execute-action)
               (commandp
                'ace-jump-helm-line-execute-action)
               (symbol-file
                'ace-jump-helm-line-execute-action
                'defun))"##;
    let expect = expect![[
        r#"OK (ace-jump-helm-line-and-select (autoload "ace-jump-helm-line" "Jump to and select the candidate in helm window." t nil) t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-helm-line/20160918.1836/home/.emacs.d/elpa/ace-jump-helm-line-20160918.1836/ace-jump-helm-line-autoloads.el")"#
    ]];
    assert_ace_jump_helm_line_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_autoshow_mode_fresh_autoload_variable_and_custom_metadata_match() {
    let elisp_form = r##"(let ((definition
                    (symbol-function
                     'ace-jump-helm-line-autoshow-mode)))
               (list
                (boundp
                 'ace-jump-helm-line-autoshow-mode)
                ace-jump-helm-line-autoshow-mode
                (documentation-property
                 'ace-jump-helm-line-autoshow-mode
                 'variable-documentation
                 t)
                (get
                 'ace-jump-helm-line-autoshow-mode
                 'custom-autoload)
                (autoloadp definition)
                (nth 1 definition)
                (nth 2 definition)
                (nth 3 definition)
                (nth 4 definition)
                (commandp
                 'ace-jump-helm-line-autoshow-mode)
                (help-function-arglist
                 'ace-jump-helm-line-autoshow-mode
                 t)
                (documentation
                 'ace-jump-helm-line-autoshow-mode)
                (symbol-file
                 'ace-jump-helm-line-autoshow-mode
                 'defun)))"##;
    let expect = expect![[
        r#"OK (t nil "Non-nil if Ace-Jump-Helm-Line-Autoshow mode is enabled.\nSee the `ace-jump-helm-line-autoshow-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `ace-jump-helm-line-autoshow-mode'." t t "ace-jump-helm-line" "Automatically show line labels in `helm'.\n\nThis is a global minor mode.  If called interactively, toggle the\n`Ace-Jump-Helm-Line-Autoshow mode' mode.  If the prefix argument is\npositive, enable the mode, and if it is zero or negative, disable the\nmode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate `(default-value \\='ace-jump-helm-line-autoshow-mode)'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" t nil t "[Arg list not available until function definition is loaded.]" "Automatically show line labels in ‘helm’.\n\nThis is a global minor mode.  If called interactively, toggle the\n‘Ace-Jump-Helm-Line-Autoshow mode’ mode.  If the prefix argument is\npositive, enable the mode, and if it is zero or negative, disable the\nmode.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate ‘(default-value 'ace-jump-helm-line-autoshow-mode)’.\n\nThe mode’s hook is called both when the mode is enabled and when it is\ndisabled.\n\n(fn &optional ARG)" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-jump-helm-line/20160918.1836/home/.emacs.d/elpa/ace-jump-helm-line-20160918.1836/ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_fresh_autoload_leaves_internal_surface_undefined() {
    let elisp_form = r##"(list
               (mapcar
                #'fboundp
                '(ace-jump-helm-line-action-persistent
                  ace-jump-helm-line-action-select
                  ace-jump-helm-line-action-move-only
                  ace-jump-helm-line--move-selection
                  ace-jump-helm-line--get-dispatch-alist
                  ace-jump-helm-line--collect-lines
                  ace-jump-helm-line--scroll-function
                  ace-jump-helm-line--add-scroll-function
                  ace-jump-helm-line--do
                  ace-jump-helm-line--exec-default-action
                  ace-jump-helm-line--with-helm-minibuffer-setup-hook
                  ace-jump-helm-line--do-if-empty
                  ace-jump-helm-line--maybe
                  ace-jump-helm-line--update-line-overlays-maybe
                  ace-jump-helm-line--cleanup-overlays
                  ace-jump-helm-line--linum
                  turn-on-ace-jump-helm-line--linum))
               (mapcar
                #'boundp
                '(ace-jump-helm-line-keys
                  ace-jump-helm-line-style
                  ace-jump-helm-line-background
                  ace-jump-helm-line-use-avy-style
                  ace-jump-helm-line-persistent-key
                  ace-jump-helm-line-select-key
                  ace-jump-helm-line-move-only-key
                  ace-jump-helm-line-default-action
                  ace-jump-helm-line-idle-delay
                  ace-jump-helm-line-autoshow-use-linum
                  ace-jump-helm-line--tree-leafs
                  ace-jump-helm-line--original-linum-format
                  ace-jump-helm-line--action-type
                  ace-jump-helm-line--last-win-start)))"##;
    let expect = expect![
        "OK ((nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil) (nil nil nil nil nil nil nil nil nil nil nil nil nil nil))"
    ];
    assert_ace_jump_helm_line_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_fresh_autoload_registers_exact_definition_prefixes() {
    let elisp_form = r##"(list
               (gethash
                "ace-jump-helm-line-"
                definition-prefixes)
               (gethash
                "turn-on-ace-jump-helm-line--linum"
                definition-prefixes)
               (gethash
                "ace-jump-helm-line"
                definition-prefixes))"##;
    let expect = expect![[
        r#"OK (("ace-jump-helm-line" "ace-jump-helm-line") ("ace-jump-helm-line" "ace-jump-helm-line") nil)"#
    ]];
    assert_ace_jump_helm_line_autoload_parity(elisp_form, expect);
}
