use super::assert_ace_jump_helm_line_parity;
use expect_test::expect;

#[test]
fn ace_jump_helm_line_public_entrypoint_callable_metadata_matches() {
    let elisp_form = r##"(list
               (commandp 'ace-jump-helm-line)
               (help-function-arglist
                'ace-jump-helm-line
                t)
               (interactive-form
                'ace-jump-helm-line)
               (documentation
                'ace-jump-helm-line)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line
                 'defun)))"##;
    let expect = expect![[
        r#"OK (t nil (interactive nil) "Jump to a candidate and execute the default action." "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_action_callable_metadata_matches() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (list
                  symbol
                  (help-function-arglist symbol t)
                  (commandp symbol)
                  (interactive-form symbol)
                  (documentation symbol)
                  (file-name-nondirectory
                   (symbol-file symbol 'defun))))
               '(ace-jump-helm-line-action-persistent
                 ace-jump-helm-line-action-select
                 ace-jump-helm-line-action-move-only
                 ace-jump-helm-line--move-selection))"##;
    let expect = expect![[
        r#"OK ((ace-jump-helm-line-action-persistent (pt) nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line-action-select (pt) nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line-action-move-only (pt) nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--move-selection nil nil nil nil "ace-jump-helm-line.el"))"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_collection_execution_and_idle_callable_metadata_matches() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (list
                  symbol
                  (help-function-arglist symbol t)
                  (commandp symbol)
                  (interactive-form symbol)
                  (documentation symbol)
                  (file-name-nondirectory
                   (symbol-file symbol 'defun))))
               '(ace-jump-helm-line--get-dispatch-alist
                 ace-jump-helm-line--collect-lines
                 ace-jump-helm-line--scroll-function
                 ace-jump-helm-line--add-scroll-function
                 ace-jump-helm-line--do
                 ace-jump-helm-line--exec-default-action
                 ace-jump-helm-line--do-if-empty
                 ace-jump-helm-line--maybe))"##;
    let expect = expect![[
        r#"OK ((ace-jump-helm-line--get-dispatch-alist nil nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--collect-lines (win-start &optional win-end) nil nil "Collect lines in helm window." "ace-jump-helm-line.el") (ace-jump-helm-line--scroll-function (win start-pos) nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--add-scroll-function nil nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--do nil nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--exec-default-action nil nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--do-if-empty nil nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--maybe (orig-func &rest args) nil nil nil "ace-jump-helm-line.el"))"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_preview_callable_metadata_matches() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (list
                  symbol
                  (help-function-arglist symbol t)
                  (commandp symbol)
                  (interactive-form symbol)
                  (documentation symbol)
                  (file-name-nondirectory
                   (symbol-file symbol 'defun))))
               '(ace-jump-helm-line--update-line-overlays-maybe
                 ace-jump-helm-line--cleanup-overlays
                 ace-jump-helm-line--linum
                 turn-on-ace-jump-helm-line--linum))"##;
    let expect = expect![[
        r#"OK ((ace-jump-helm-line--update-line-overlays-maybe (&optional win-start) t (interactive nil) nil "ace-jump-helm-line.el") (ace-jump-helm-line--cleanup-overlays nil nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line--linum (line-number) nil nil nil "ace-jump-helm-line.el") (turn-on-ace-jump-helm-line--linum nil nil nil nil "ace-jump-helm-line.el"))"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_public_compatibility_and_idle_callable_metadata_matches() {
    let elisp_form = r##"(mapcar
               (lambda (symbol)
                 (list
                  symbol
                  (help-function-arglist symbol t)
                  (commandp symbol)
                  (interactive-form symbol)
                  (documentation symbol)
                  (file-name-nondirectory
                   (symbol-file symbol 'defun))))
               '(ace-jump-helm-line-and-select
                 ace-jump-helm-line-execute-action
                 ace-jump-helm-line-idle-exec-add
                 ace-jump-helm-line-idle-exec-remove))"##;
    let expect = expect![[
        r#"OK ((ace-jump-helm-line-and-select nil t (interactive nil) "Jump to and select the candidate in helm window." "ace-jump-helm-line.el") (ace-jump-helm-line-execute-action nil t (interactive nil) "Jump to and select the candidate in helm window." "ace-jump-helm-line.el") (ace-jump-helm-line-idle-exec-add (func) nil nil nil "ace-jump-helm-line.el") (ace-jump-helm-line-idle-exec-remove (func) nil nil nil "ace-jump-helm-line.el"))"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_execute_action_alias_identity_and_source_match() {
    let elisp_form = r##"(list
               (symbol-function
                'ace-jump-helm-line-execute-action)
               (eq
                (indirect-function
                 'ace-jump-helm-line-execute-action)
                (indirect-function
                 'ace-jump-helm-line-and-select))
               (commandp
                'ace-jump-helm-line-execute-action)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-execute-action
                 'defun)))"##;
    let expect = expect![[r#"OK (ace-jump-helm-line-and-select t t "ace-jump-helm-line.el")"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_autoshow_global_mode_metadata_matches() {
    let elisp_form = r##"(list
               ace-jump-helm-line-autoshow-mode
               (special-variable-p
                'ace-jump-helm-line-autoshow-mode)
               (documentation-property
                'ace-jump-helm-line-autoshow-mode
                'variable-documentation
                t)
               (get
                'ace-jump-helm-line-autoshow-mode
                'custom-type)
               (get
                'ace-jump-helm-line-autoshow-mode
                'standard-value)
               (get
                'ace-jump-helm-line-autoshow-mode
                'variable-documentation)
               (commandp
                'ace-jump-helm-line-autoshow-mode)
               (help-function-arglist
                'ace-jump-helm-line-autoshow-mode
                t)
               (interactive-form
                'ace-jump-helm-line-autoshow-mode)
               (documentation
                'ace-jump-helm-line-autoshow-mode)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line-autoshow-mode
                 'defun)))"##;
    let expect = expect![[
        r#"OK (nil t "Non-nil if Ace-Jump-Helm-Line-Autoshow mode is enabled.\nSee the `ace-jump-helm-line-autoshow-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `ace-jump-helm-line-autoshow-mode'." boolean (nil) "Non-nil if Ace-Jump-Helm-Line-Autoshow mode is enabled.\nSee the `ace-jump-helm-line-autoshow-mode' command\nfor a description of this minor mode.\nSetting this variable directly does not take effect;\neither customize it (see the info node `Easy Customization')\nor call the function `ace-jump-helm-line-autoshow-mode'." t (&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Automatically show line labels in ‘helm’.\n\nThis is a global minor mode.  If called interactively, toggle the\n‘Ace-Jump-Helm-Line-Autoshow mode’ mode.  If the prefix argument is\npositive, enable the mode, and if it is zero or negative, disable the\nmode.\n\nIf called from Lisp, toggle the mode if ARG is ‘toggle’.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate ‘(default-value 'ace-jump-helm-line-autoshow-mode)’.\n\nThe mode’s hook is called both when the mode is enabled and when it is\ndisabled." "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_packaged_source_descriptor_autoload_and_readme_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-jump-helm-line
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor)))
               (mapcar
                (lambda (name)
                  (let ((path
                         (expand-file-name
                          name
                          directory)))
                    (with-temp-buffer
                      (set-buffer-multibyte nil)
                      (insert-file-contents-literally path)
                      (list
                       name
                       (buffer-size)
                       (secure-hash
                        'sha256
                        (current-buffer))))))
                '("ace-jump-helm-line.el"
                  "ace-jump-helm-line-pkg.el"
                  "ace-jump-helm-line-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-jump-helm-line.el" 23650 "7d2c79cc68a689604b5d4de58b427445330d1ba26fd366758c32413af785ad3c") ("ace-jump-helm-line-pkg.el" 467 "2cc12e0f2d8441adab1652204936d293273736661c9462c0475b0cc278641051") ("ace-jump-helm-line-autoloads.el" 2412 "5bb6cb3deb625069aa17ab6d0e69758ead8ec828eeae7d6352e2600ea4edfd6c") ("README-elpa" 9637 "734320c34e2aaff009f05e8006da0b291e1a880814eb1251b67e050f0408964d"))"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_installation_produces_a_local_byte_compilation_artifact() {
    let elisp_form = r##"(let* ((descriptor
                      (cadr
                       (assq
                        'ace-jump-helm-line
                        package-alist)))
                     (directory
                      (package-desc-dir descriptor))
                     (path
                      (expand-file-name
                       "ace-jump-helm-line.elc"
                       directory)))
               (list
                (file-exists-p path)
                (file-regular-p path)
                (> (file-attribute-size
                    (file-attributes path))
                   0)))"##;
    let expect = expect!["OK (t t t)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
