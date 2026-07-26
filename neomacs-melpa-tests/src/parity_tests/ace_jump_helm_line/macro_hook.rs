use super::assert_ace_jump_helm_line_parity;
use expect_test::expect;

#[test]
fn ace_jump_helm_line_setup_hook_macro_metadata_matches() {
    let elisp_form = r##"(list
               (macrop
                'ace-jump-helm-line--with-helm-minibuffer-setup-hook)
               (get
                'ace-jump-helm-line--with-helm-minibuffer-setup-hook
                'lisp-indent-function)
               (get
                'ace-jump-helm-line--with-helm-minibuffer-setup-hook
                'edebug-form-spec)
               (help-function-arglist
                'ace-jump-helm-line--with-helm-minibuffer-setup-hook
                t)
               (documentation
                'ace-jump-helm-line--with-helm-minibuffer-setup-hook)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-helm-line--with-helm-minibuffer-setup-hook
                 'defun)))"##;
    let expect = expect![[
        r#"OK (t 1 t (fun &rest body) "Temporarily add FUN to ‘helm-minibuffer-set-up-hook’ while executing BODY." "ace-jump-helm-line.el")"#
    ]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_setup_hook_macro_expansion_uses_one_uninterned_self_removing_hook() {
    let elisp_form = r##"(let* ((expansion
                     (macroexpand
                      '(ace-jump-helm-line--with-helm-minibuffer-setup-hook
                           #'target
                         (first-form)
                         (second-form))))
                    (hook-symbol
                     (car
                      (cadr expansion)))
                    (assignment
                     (nth 2 expansion))
                    (unwind
                     (nth 3 expansion))
                    (protected
                     (nth 1 unwind))
                    (cleanup
                     (nth 2 unwind))
                    (callback
                     (nth 2 assignment))
                    (self-removal
                     (nth 2 callback))
                    (registration
                     (nth 1 protected)))
               (list
                (symbol-name hook-symbol)
                (eq hook-symbol
                    (intern-soft
                     (symbol-name hook-symbol)))
                (eq hook-symbol
                    (nth 1 assignment))
                (eq hook-symbol
                    (nth 2 self-removal))
                (eq hook-symbol
                    (nth 2 registration))
                (eq hook-symbol
                    (nth 2 cleanup))
                (copy-tree
                 (cddr protected))))"##;
    let expect = expect![[r#"OK ("setup-hook" nil t t t t ((first-form) (second-form)))"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_setup_hook_runs_once_removes_itself_and_returns_last_body_value() {
    let elisp_form = r##"(let (events)
               (let ((helm-minibuffer-set-up-hook
                      (list
                       (lambda ()
                         (push 'outer events)))))
                 (list
                  (ace-jump-helm-line--with-helm-minibuffer-setup-hook
                      (lambda ()
                        (push 'target events))
                    (push
                     (length helm-minibuffer-set-up-hook)
                     events)
                    (run-hooks
                     'helm-minibuffer-set-up-hook)
                    (push
                     (length helm-minibuffer-set-up-hook)
                     events)
                    'body-result)
                  (nreverse events)
                  (length helm-minibuffer-set-up-hook))))"##;
    let expect = expect!["OK (body-result (2 target outer 1) 1)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_setup_hook_removes_unused_temporary_hook_after_body() {
    let elisp_form = r##"(let ((helm-minibuffer-set-up-hook nil))
               (list
                (ace-jump-helm-line--with-helm-minibuffer-setup-hook
                    #'ignore
                  (length
                   helm-minibuffer-set-up-hook))
                helm-minibuffer-set-up-hook))"##;
    let expect = expect!["OK (1 nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_setup_hook_restores_hook_after_body_error() {
    let elisp_form = r##"(let ((helm-minibuffer-set-up-hook '(outer)))
               (list
                (condition-case err
                    (ace-jump-helm-line--with-helm-minibuffer-setup-hook
                        #'ignore
                      (error "body failed"))
                  (error err))
                helm-minibuffer-set-up-hook))"##;
    let expect = expect![[r#"OK ((error "body failed") (outer))"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_setup_hook_removes_itself_before_target_error() {
    let elisp_form = r##"(let ((helm-minibuffer-set-up-hook nil)
                   observed)
               (list
                (condition-case err
                    (ace-jump-helm-line--with-helm-minibuffer-setup-hook
                        (lambda ()
                          (setq observed
                                (copy-sequence
                                 helm-minibuffer-set-up-hook))
                          (error "target failed"))
                      (run-hooks
                       'helm-minibuffer-set-up-hook))
                  (error err))
                observed
                helm-minibuffer-set-up-hook))"##;
    let expect = expect![[r#"OK ((error "target failed") nil nil)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
