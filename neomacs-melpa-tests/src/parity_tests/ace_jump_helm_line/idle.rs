use super::assert_ace_jump_helm_line_parity;
use expect_test::expect;

#[test]
fn ace_jump_helm_line_do_if_empty_skips_nonempty_minibuffer_contents() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'minibuffer-contents)
                     (lambda () "query"))
                    ((symbol-function
                      'ace-jump-helm-line)
                     (lambda ()
                       (push 'jump events))))
                 (list
                  (ace-jump-helm-line--do-if-empty)
                  events)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_if_empty_invokes_jump_and_returns_its_value() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'minibuffer-contents)
                     (lambda () ""))
                    ((symbol-function
                      'ace-jump-helm-line)
                     (lambda ()
                       (push 'jump events)
                       'jump-result)))
                 (list
                  (ace-jump-helm-line--do-if-empty)
                  events)))"##;
    let expect = expect!["OK (jump-result (jump))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_if_empty_reports_jump_errors_as_messages() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'minibuffer-contents)
                     (lambda () ""))
                    ((symbol-function
                      'ace-jump-helm-line)
                     (lambda ()
                       (error "jump failed")))
                    ((symbol-function
                      'message)
                     (lambda (format-string &rest args)
                       (let ((text
                              (apply #'format
                                     format-string
                                     args)))
                         (push text events)
                         text))))
                 (list
                  (ace-jump-helm-line--do-if-empty)
                  events)))"##;
    let expect = expect![[r#"OK ("jump failed" ("jump failed"))"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_maybe_installs_one_shot_timer_hook_and_forwards_arguments() {
    let elisp_form = r##"(let ((helm-minibuffer-set-up-hook nil)
                   (ace-jump-helm-line-idle-delay 2.5)
                   events)
               (cl-letf
                   (((symbol-function
                      'run-at-time)
                     (lambda (delay repeat function &rest args)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'timer
                                delay
                                repeat
                                function
                                args))))
                       'timer-object)))
                 (list
                  (ace-jump-helm-line--maybe
                   (lambda (&rest args)
                     (setq events
                           (append
                            events
                            (list
                             (list
                              'orig
                              args
                              (length
                               helm-minibuffer-set-up-hook)))))
                     (run-hooks
                      'helm-minibuffer-set-up-hook)
                     'orig-result)
                   'alpha
                   7)
                  events
                  helm-minibuffer-set-up-hook)))"##;
    let expect = expect![
        "OK (orig-result ((orig (alpha 7) 1) (timer 2.5 nil ace-jump-helm-line--do-if-empty nil)) nil)"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_maybe_removes_setup_hook_when_original_errors_before_setup() {
    let elisp_form = r##"(let ((helm-minibuffer-set-up-hook '(outer)))
               (list
                (condition-case err
                    (ace-jump-helm-line--maybe
                     (lambda (&rest _)
                       (error "original failed"))
                     1 2)
                  (error err))
                helm-minibuffer-set-up-hook))"##;
    let expect = expect![[r#"OK ((error "original failed") (outer))"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_idle_advice_add_and_remove_are_idempotent() {
    let elisp_form = r##"(let (count)
               (fset
                'ace-jump-helm-line-test-command
                (lambda (&rest args)
                  args))
               (unwind-protect
                   (list
                    (ace-jump-helm-line-idle-exec-add
                     'ace-jump-helm-line-test-command)
                    (not
                     (null
                      (advice-member-p
                       #'ace-jump-helm-line--maybe
                       'ace-jump-helm-line-test-command)))
                    (ace-jump-helm-line-idle-exec-add
                     'ace-jump-helm-line-test-command)
                    (progn
                      (setq count 0)
                      (advice-mapc
                       (lambda (&rest _)
                         (setq count
                               (1+ count)))
                       'ace-jump-helm-line-test-command)
                      count)
                    (ace-jump-helm-line-idle-exec-remove
                     'ace-jump-helm-line-test-command)
                    (not
                     (null
                      (advice-member-p
                       #'ace-jump-helm-line--maybe
                       'ace-jump-helm-line-test-command)))
                    (ace-jump-helm-line-idle-exec-remove
                     'ace-jump-helm-line-test-command))
                 (fmakunbound
                  'ace-jump-helm-line-test-command)))"##;
    let expect = expect!["OK (nil t nil 1 nil nil nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_idle_advice_dispatches_once_then_removal_restores_direct_calls() {
    let elisp_form = r##"(let (events)
               (let ((helm-minibuffer-set-up-hook nil)
                     (ace-jump-helm-line-idle-delay 4))
                 (fset
                  'ace-jump-helm-line-test-dispatch
                  (lambda (&rest args)
                    (setq events
                          (append
                           events
                           (list
                            (list
                             'target
                             args
                             (length
                              helm-minibuffer-set-up-hook)))))
                    (run-hooks
                     'helm-minibuffer-set-up-hook)
                    'target-result))
                 (unwind-protect
                     (cl-letf
                         (((symbol-function
                            'run-at-time)
                           (lambda (delay repeat function &rest args)
                             (setq events
                                   (append
                                    events
                                    (list
                                     (list
                                      'timer
                                      delay
                                      repeat
                                      function
                                      args))))
                             'timer-object)))
                       (ace-jump-helm-line-idle-exec-add
                        'ace-jump-helm-line-test-dispatch)
                       (let ((advised
                              (ace-jump-helm-line-test-dispatch
                               'first 1)))
                         (ace-jump-helm-line-idle-exec-remove
                          'ace-jump-helm-line-test-dispatch)
                         (list
                          advised
                          (ace-jump-helm-line-test-dispatch
                           'second 2)
                          events
                          helm-minibuffer-set-up-hook)))
                   (fmakunbound
                    'ace-jump-helm-line-test-dispatch))))"##;
    let expect = expect![
        "OK (target-result target-result ((target (first 1) 1) (timer 4 nil ace-jump-helm-line--do-if-empty nil) (target (second 2) 0)) nil)"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
