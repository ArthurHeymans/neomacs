use super::assert_ace_jump_helm_line_parity;
use expect_test::expect;

#[test]
fn ace_jump_helm_line_move_only_action_moves_point_sets_type_then_updates_selection() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'move
                                (point)
                                ace-jump-helm-line--action-type)))))))
                 (with-temp-buffer
                   (insert "abcdef")
                   (goto-char 2)
                   (setq ace-jump-helm-line--action-type 'outer)
                   (list
                    (ace-jump-helm-line-action-move-only 5)
                    (point)
                    ace-jump-helm-line--action-type
                    events))))"##;
    let expect = expect!["OK (#1=((move 5 move-only)) 5 move-only #1#)"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_persistent_action_orders_move_selection_before_persistent_action() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'move
                                (point)
                                ace-jump-helm-line--action-type))))))
                    ((symbol-function
                      'helm-execute-persistent-action)
                     (lambda ()
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'persistent
                                (point)
                                ace-jump-helm-line--action-type))))
                       'persistent-result)))
                 (with-temp-buffer
                   (insert "abcdef")
                   (goto-char 2)
                   (setq ace-jump-helm-line--action-type 'outer)
                   (list
                    (ace-jump-helm-line-action-persistent 6)
                    (point)
                    ace-jump-helm-line--action-type
                    events))))"##;
    let expect = expect![
        "OK (persistent-result 6 persistent ((move 6 persistent) (persistent 6 persistent)))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_select_action_orders_move_selection_before_helm_exit() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function
                      'ace-jump-helm-line--move-selection)
                     (lambda ()
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'move
                                (point)
                                ace-jump-helm-line--action-type))))))
                    ((symbol-function
                      'helm-exit-minibuffer)
                     (lambda ()
                       (setq events
                             (append
                              events
                              (list
                               (list
                                'exit
                                (point)
                                ace-jump-helm-line--action-type))))
                       'exit-result)))
                 (with-temp-buffer
                   (insert "abcdef")
                   (goto-char 4)
                   (setq ace-jump-helm-line--action-type 'outer)
                   (list
                    (ace-jump-helm-line-action-select 1)
                    (point)
                    ace-jump-helm-line--action-type
                    events))))"##;
    let expect = expect!["OK (exit-result 1 select ((move 1 select) (exit 1 select)))"];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_move_selection_stops_when_previous_keeps_point_fixed() {
    let elisp_form = r##"(let ((helm-after-preselection-hook '(outer-pre))
                   (helm-move-selection-after-hook '(outer-move))
                   (helm-after-update-hook '(outer-update))
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-move-selection-common)
                     (lambda (&rest args)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                args
                                helm-after-preselection-hook
                                helm-move-selection-after-hook
                                helm-after-update-hook
                                (point))))))))
                 (with-temp-buffer
                   (insert "a\nb\n")
                   (goto-char 3)
                   (list
                    (ace-jump-helm-line--move-selection)
                    (point)
                    events
                    helm-after-preselection-hook
                    helm-move-selection-after-hook
                    helm-after-update-hook))))"##;
    let expect = expect![
        "OK (nil 3 (((:where line :direction previous) nil nil nil 3)) (outer-pre) (outer-move) (outer-update))"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_move_selection_moves_next_after_previous_changes_point() {
    let elisp_form = r##"(let ((helm-after-preselection-hook 'outer-pre)
                   (helm-move-selection-after-hook 'outer-move)
                   (helm-after-update-hook 'outer-update)
                   events)
               (cl-letf
                   (((symbol-function
                      'helm-move-selection-common)
                     (lambda (&rest args)
                       (setq events
                             (append
                              events
                              (list
                               (list
                                args
                                helm-after-preselection-hook
                                helm-move-selection-after-hook
                                helm-after-update-hook
                                (point)))))
                       (if
                           (eq
                            (plist-get args :direction)
                            'previous)
                           (goto-char 1)
                         (goto-char 5)))))
                 (with-temp-buffer
                   (insert "a\nb\nc\n")
                   (goto-char 3)
                   (list
                    (ace-jump-helm-line--move-selection)
                    (point)
                    events
                    helm-after-preselection-hook
                    helm-move-selection-after-hook
                    helm-after-update-hook))))"##;
    let expect = expect![
        "OK (5 5 (((:where line :direction previous) nil nil nil 3) ((:where line :direction next) nil nil nil 1)) outer-pre outer-move outer-update)"
    ];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}

#[test]
fn ace_jump_helm_line_move_selection_propagates_errors_with_outer_hooks_intact() {
    let elisp_form = r##"(let ((helm-after-preselection-hook 'outer-pre)
                   (helm-move-selection-after-hook 'outer-move)
                   (helm-after-update-hook 'outer-update)
                   observed)
               (cl-letf
                   (((symbol-function
                      'helm-move-selection-common)
                     (lambda (&rest _)
                       (setq observed
                             (list
                              helm-after-preselection-hook
                              helm-move-selection-after-hook
                              helm-after-update-hook))
                       (error "move failed"))))
                 (condition-case err
                     (ace-jump-helm-line--move-selection)
                   (error
                    (list
                     err
                     observed
                     helm-after-preselection-hook
                     helm-move-selection-after-hook
                     helm-after-update-hook)))))"##;
    let expect =
        expect![[r#"OK ((error "move failed") (nil nil nil) outer-pre outer-move outer-update)"#]];
    assert_ace_jump_helm_line_parity(elisp_form, expect);
}
