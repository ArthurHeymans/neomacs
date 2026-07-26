use super::{assert_ace_jump_helm_line_parity, assert_ace_jump_helm_line_signal_parity};
use expect_test::expect;

#[test]
fn ace_jump_helm_line_do_signals_when_no_helm_session_is_running() {
    let elisp_form = r##"(let ((helm-alive-p nil))
               (ace-jump-helm-line--do))"##;
    let expect = expect![[r#"ERR (error "No helm session is running")"#]];
    assert_ace_jump_helm_line_signal_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_default_action_is_a_noop_without_a_live_helm_session() {
    let elisp_form = r##"(let ((helm-alive-p nil)
                   (ace-jump-helm-line-default-action 'select)
                   (ace-jump-helm-line--action-type 'select)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (push 'move events)))
                    ((symbol-function
                      'helm-exit-minibuffer)
                     (lambda ()
                       (push 'exit events))))
                 (list
                  (ace-jump-helm-line--exec-default-action)
                  events)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_default_action_is_a_noop_when_dispatched_action_differs() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-default-action 'select)
                   (ace-jump-helm-line--action-type 'persistent)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (push 'move events)))
                    ((symbol-function
                      'helm-exit-minibuffer)
                     (lambda ()
                       (push 'exit events))))
                 (list
                  (ace-jump-helm-line--exec-default-action)
                  events)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_nil_and_move_only_defaults_only_move_selection() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (push 'move events)
                       'move-result))
                    ((symbol-function
                      'helm-exit-minibuffer)
                     (lambda ()
                       (push 'exit events)))
                    ((symbol-function
                      'helm-execute-persistent-action)
                     (lambda ()
                       (push 'persistent events))))
                 (list
                  (let ((ace-jump-helm-line-default-action nil)
                        (ace-jump-helm-line--action-type nil))
                    (ace-jump-helm-line--exec-default-action))
                  (let ((ace-jump-helm-line-default-action 'move-only)
                        (ace-jump-helm-line--action-type 'move-only))
                    (ace-jump-helm-line--exec-default-action))
                  events)))"##;
    let expect = expect!["OK (nil nil (move move))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_select_default_moves_then_exits() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-default-action 'select)
                   (ace-jump-helm-line--action-type 'select)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (setq events
                             (append events '(move)))))
                    ((symbol-function
                      'helm-exit-minibuffer)
                     (lambda ()
                       (setq events
                             (append events '(exit)))
                       'exit-result)))
                 (list
                  (ace-jump-helm-line--exec-default-action)
                  events)))"##;
    let expect = expect!["OK (exit-result (move exit))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_persistent_default_moves_then_executes_action() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-default-action 'persistent)
                   (ace-jump-helm-line--action-type 'persistent)
                   events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (setq events
                             (append events '(move)))))
                    ((symbol-function
                      'helm-execute-persistent-action)
                     (lambda ()
                       (setq events
                             (append events '(persistent)))
                       'persistent-result)))
                 (list
                  (ace-jump-helm-line--exec-default-action)
                  events)))"##;
    let expect = expect!["OK (persistent-result (move persistent))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_public_command_binds_action_type_to_default_for_the_call() {
    let elisp_form = r##"(let ((ace-jump-helm-line-default-action 'persistent)
                   (ace-jump-helm-line--action-type 'outer)
                   observed)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--do)
                     (lambda ()
                       (setq observed
                             ace-jump-helm-line--action-type)
                       'do-result)))
                 (list
                  (ace-jump-helm-line)
                  observed
                  ace-jump-helm-line--action-type)))"##;
    let expect = expect!["OK (do-result persistent outer)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_public_command_restores_action_type_after_error() {
    let elisp_form = r##"(let ((ace-jump-helm-line-default-action 'select)
                   (ace-jump-helm-line--action-type 'outer)
                   observed)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--do)
                     (lambda ()
                       (setq observed
                             ace-jump-helm-line--action-type)
                       (error "do failed"))))
                 (list
                  (condition-case err
                      (ace-jump-helm-line)
                    (error err))
                  observed
                  ace-jump-helm-line--action-type)))"##;
    let expect = expect![[r#"OK ((error "do failed") select outer)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_and_select_overrides_default_only_around_primary_command() {
    let elisp_form = r##"(let ((ace-jump-helm-line-default-action 'outer)
                   observed)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line)
                     (lambda ()
                       (setq observed
                             ace-jump-helm-line-default-action)
                       'jump-result)))
                 (list
                  (ace-jump-helm-line-and-select)
                  observed
                  ace-jump-helm-line-default-action)))"##;
    let expect = expect!["OK (jump-result select outer)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_passes_dynamic_avy_configuration_and_executes_default() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-background 'custom-background)
                   (ace-jump-helm-line-keys '(?x ?y))
                   (ace-jump-helm-line-style 'post)
                   (ace-jump-helm-line-autoshow-mode nil)
                   (avy-keys '(?a))
                   (avy-style 'pre)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'ace-jump-helm-line--get-dispatch-alist)
                     (lambda ()
                       '((?d . dispatch-action))))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (start end)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'collect
                                start
                                end))))
                       '((11 . window))))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (style)
                       (setq events
                             (append
                              events
                              (list
                               (list 'style style))))
                       'style-function))
                    ((symbol-function
                      'avy--process)
                     (lambda (candidates style-function)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'process
                                candidates
                                style-function
                                avy-background
                                avy-keys
                                avy-dispatch-alist
                                avy-style
                                avy-action
                                avy-all-windows))))
                       11))
                    ((symbol-function
                      'ace-jump-helm-line--exec-default-action)
                     (lambda ()
                       (setq events
                             (append
                              events
                              '(default)))
                       'default-result)))
                 (list
                  (ace-jump-helm-line--do)
                  events
                  avy-keys
                  avy-style)))"##;
    let expect = expect![
        "OK (default-result ((collect 1 1) (style post) (process ((11 . window)) style-function custom-background (120 121) ((100 . dispatch-action)) post nil nil) default) (97) pre)"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_skips_default_when_avy_returns_a_non_number() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-autoshow-mode nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (_)
                       #'ignore))
                    ((symbol-function
                      'avy--process)
                     (lambda (&rest _)
                       'cancelled))
                    ((symbol-function
                      'ace-jump-helm-line--exec-default-action)
                     (lambda ()
                       (push 'default events))))
                 (list
                  (ace-jump-helm-line--do)
                  events)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_uses_dispatched_avy_action_instead_of_default() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-autoshow-mode nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       '((1 . window))))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (_)
                       #'ignore))
                    ((symbol-function
                      'avy--process)
                     (lambda (&rest _)
                       (setq avy-action 'dispatched-action)
                       1))
                    ((symbol-function
                      'ace-jump-helm-line--exec-default-action)
                     (lambda ()
                       (push 'default events))))
                 (list
                  (ace-jump-helm-line--do)
                  events)))"##;
    let expect = expect!["OK (dispatched-action nil)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_disables_and_restores_linum_preview_around_processing() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-autoshow-mode t)
                   (ace-jump-helm-line-autoshow-use-linum t)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'linum-mode)
                     (lambda (arg)
                       (setq events
                             (append
                              events
                              (list
                               (list 'linum arg))))))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       (setq events
                             (append events
                                     '(collect)))
                       nil))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (_)
                       #'ignore))
                    ((symbol-function
                      'avy--process)
                     (lambda (&rest _)
                       (setq events
                             (append events
                                     '(process)))
                       'cancelled))
                    ((symbol-function
                      'turn-on-ace-jump-helm-line--linum)
                     (lambda ()
                       (setq events
                             (append events
                                     '(turn-on)))))
                    ((symbol-function
                      'ace-jump-helm-line--update-line-overlays-maybe)
                     (lambda ()
                       (setq events
                             (append events
                                     '(update))))))
                 (list
                  (ace-jump-helm-line--do)
                  events)))"##;
    let expect = expect!["OK (nil ((linum -1) collect process turn-on update))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_restores_preview_after_avy_error() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-autoshow-mode t)
                   (ace-jump-helm-line-autoshow-use-linum t)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'linum-mode)
                     (lambda (arg)
                       (setq events
                             (append
                              events
                              (list
                               (list 'linum arg))))))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (_)
                       #'ignore))
                    ((symbol-function
                      'avy--process)
                     (lambda (&rest _)
                       (setq events
                             (append events
                                     '(process)))
                       (error "avy failed")))
                    ((symbol-function
                      'turn-on-ace-jump-helm-line--linum)
                     (lambda ()
                       (setq events
                             (append events
                                     '(turn-on)))))
                    ((symbol-function
                      'ace-jump-helm-line--update-line-overlays-maybe)
                     (lambda ()
                       (setq events
                             (append events
                                     '(update))))))
                 (list
                  (condition-case err
                      (ace-jump-helm-line--do)
                    (error err))
                  events)))"##;
    let expect = expect![[r#"OK ((error "avy failed") ((linum -1) process turn-on update))"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_processes_in_helm_window_then_restores_original_window() {
    let elisp_form = r##"(let* ((original-window
                      (selected-window))
                     (target-window
                      (split-window
                       original-window))
                     (helm-alive-p t)
                     (ace-jump-helm-line-autoshow-mode nil)
                     events)
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'helm-window)
                         (lambda ()
                           target-window))
                        ((symbol-function
                          'ace-jump-helm-line--collect-lines)
                         (lambda (&rest _)
                           (push
                            (list
                             'collect
                             (eq
                              (selected-window)
                              target-window))
                            events)
                           nil))
                        ((symbol-function
                          'avy--style-fn)
                         (lambda (_)
                           #'ignore))
                        ((symbol-function
                          'avy--process)
                         (lambda (&rest _)
                           (push
                            (list
                             'process
                             (eq
                              (selected-window)
                              target-window))
                            events)
                           'cancelled)))
                     (list
                      (ace-jump-helm-line--do)
                      (nreverse events)
                      (eq
                       (selected-window)
                       original-window)))
                 (delete-window target-window)))"##;
    let expect = expect!["OK (nil ((collect t) (process t)) t)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_restores_original_window_after_avy_error() {
    let elisp_form = r##"(let* ((original-window
                      (selected-window))
                     (target-window
                      (split-window
                       original-window))
                     (helm-alive-p t)
                     (ace-jump-helm-line-autoshow-mode nil)
                     events)
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'helm-window)
                         (lambda ()
                           target-window))
                        ((symbol-function
                          'ace-jump-helm-line--collect-lines)
                         (lambda (&rest _)
                           nil))
                        ((symbol-function
                          'avy--style-fn)
                         (lambda (_)
                           #'ignore))
                        ((symbol-function
                          'avy--process)
                         (lambda (&rest _)
                           (push
                            (eq
                             (selected-window)
                             target-window)
                            events)
                           (error "avy window failure"))))
                     (list
                      (condition-case err
                          (ace-jump-helm-line--do)
                        (error err))
                      events
                      (eq
                       (selected-window)
                       original-window)))
                 (delete-window target-window)))"##;
    let expect = expect![[r#"OK ((error "avy window failure") (t) t)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_do_falls_back_to_avy_keys_and_style_regardless_of_obsolete_flag() {
    let elisp_form = r##"(let ((helm-alive-p t)
                   (ace-jump-helm-line-keys nil)
                   (ace-jump-helm-line-style nil)
                   (ace-jump-helm-line-autoshow-mode nil)
                   (avy-keys '(?f ?g))
                   (avy-style 'at-full)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-window)
                     (lambda ()
                       (selected-window)))
                    ((symbol-function
                      'ace-jump-helm-line--collect-lines)
                     (lambda (&rest _)
                       nil))
                    ((symbol-function
                      'avy--style-fn)
                     (lambda (style)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'style
                                style
                                ace-jump-helm-line-use-avy-style))))
                       #'ignore))
                    ((symbol-function
                      'avy--process)
                     (lambda (&rest _)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'process
                                avy-keys
                                avy-style
                                ace-jump-helm-line-use-avy-style))))
                       'cancelled)))
                 (list
                  (let ((ace-jump-helm-line-use-avy-style nil))
                    (ace-jump-helm-line--do))
                  (let ((ace-jump-helm-line-use-avy-style t))
                    (ace-jump-helm-line--do))
                  events)))"##;
    let expect = expect![
        "OK (nil nil ((style at-full nil) (process #1=(102 103) at-full nil) (style at-full t) (process #1# at-full t)))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
