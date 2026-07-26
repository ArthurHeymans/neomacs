use super::assert_ace_window_parity;
use expect_test::expect;

#[test]
fn ace_window_update_disables_all_filters_builds_tree_and_labels_every_window() {
    let elisp_form = r##"(progn
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function 'aw-window-list)
               (lambda ()
                 (push
                  (list
                   'windows
                   aw-ignore-on
                   aw-ignore-current
                   ignore-window-parameters)
                  ace-window--test-events)
                 '(w1 w2)))
              ((symbol-function 'avy-tree)
               (lambda (windows keys)
                 (push
                  (list 'tree windows keys)
                  ace-window--test-events)
                 'fixture-tree))
              ((symbol-function 'avy-traverse)
               (lambda (tree callback)
                 (push
                  (list 'traverse tree)
                  ace-window--test-events)
                 (funcall callback
                          '(49 97)
                          'w1)
                 (funcall callback
                          '(50)
                          'w2)
                 'traverse-result))
              ((symbol-function
                'set-window-parameter)
               (lambda
                   (window parameter value)
                 (push
                  (list
                   'set
                   window
                   parameter
                   (substring-no-properties
                    value)
                   (get-text-property
                    0
                    'face
                    value))
                  ace-window--test-events)
                 value)))
           (list
            (aw-update)
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect![[
        r#"OK (traverse-result ((windows nil nil t) (tree (w1 w2) (49 50 51 52 53 54 55 56 57)) (traverse fixture-tree) (set w1 ace-window-path "a1" aw-mode-line-face) (set w2 ace-window-path "2" aw-mode-line-face)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_after_make_frame_updates_labels_before_making_frame_visible() {
    let elisp_form = r##"(progn
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function 'aw-update)
               (lambda ()
                 (push 'update
                       ace-window--test-events)
                 'update-result))
              ((symbol-function
                'make-frame-visible)
               (lambda (frame)
                 (push
                  (list 'visible frame)
                  ace-window--test-events)
                 'visible-result)))
           (list
            (aw--after-make-frame
             'fixture-frame)
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect!["OK (visible-result (update (visible fixture-frame)))"];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_display_mode_lifecycle_rewrites_mode_line_and_manages_exact_hooks() {
    let elisp_form = r##"(let ((original-mode-line
              (default-value
               'mode-line-format)))
         (unwind-protect
             (progn
               (ace-window-display-mode -1)
               (set-default
                'mode-line-format
                '((fixture-before value)
                  (ace-window-display-mode
                   stale)
                  "tail"))
               (setq
                ace-window--test-events
                nil)
               (cl-letf
                   (((symbol-function 'aw-update)
                     (lambda ()
                       (push 'update
                             ace-window--test-events)))
                    ((symbol-function
                      'force-mode-line-update)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'force
                         arguments)
                        ace-window--test-events)))
                    ((symbol-function 'add-hook)
                     (lambda
                         (hook function
                          &optional depth local)
                       (push
                        (list
                         'add
                         hook
                         function
                         depth
                         local)
                        ace-window--test-events)))
                    ((symbol-function
                      'remove-hook)
                     (lambda
                         (hook function
                          &optional local)
                       (push
                        (list
                         'remove
                         hook
                         function
                         local)
                        ace-window--test-events))))
                 (let ((enabled
                        (progn
                          (ace-window-display-mode
                           1)
                          (list
                           ace-window-display-mode
                           (default-value
                            'mode-line-format)
                           (nreverse
                            (prog1
                                ace-window--test-events
                              (setq
                               ace-window--test-events
                               nil)))))))
                   (let ((disabled
                          (progn
                            (ace-window-display-mode
                             -1)
                            (list
                             ace-window-display-mode
                             (default-value
                              'mode-line-format)
                             (nreverse
                              ace-window--test-events)))))
                     (list
                      enabled
                      disabled)))))
           (ace-window-display-mode -1)
           (set-default
            'mode-line-format
            original-mode-line)))"##;
    let expect = expect![[
        r#"OK ((t ((ace-window-display-mode (:eval (window-parameter (selected-window) 'ace-window-path))) . #1=((fixture-before value) "tail")) (update (force t) (add window-configuration-change-hook aw-update nil nil) (add after-make-frame-functions aw--after-make-frame t nil) (force))) (nil #1# ((remove window-configuration-change-hook aw-update nil) (remove after-make-frame-functions aw--after-make-frame nil) (force))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_display_mode_repeated_enable_keeps_one_mode_line_entry() {
    let elisp_form = r##"(let ((original-mode-line
              (default-value
               'mode-line-format)))
         (unwind-protect
             (progn
               (ace-window-display-mode -1)
               (set-default
                'mode-line-format
                '((ace-window-display-mode
                   old-one)
                  "middle"
                  (ace-window-display-mode
                   old-two)))
               (cl-letf
                   (((symbol-function 'aw-update)
                     (lambda ()))
                    ((symbol-function
                      'force-mode-line-update)
                     (lambda (&rest _arguments)))
                    ((symbol-function 'add-hook)
                     (lambda (&rest _arguments)))
                    ((symbol-function
                      'remove-hook)
                     (lambda (&rest _arguments))))
                 (ace-window-display-mode 1)
                 (ace-window-display-mode 1)
                 (list
                  ace-window-display-mode
                  (default-value
                   'mode-line-format)
                  (length
                   (seq-filter
                    (lambda (entry)
                      (and
                       (consp entry)
                       (eq
                        (car entry)
                        'ace-window-display-mode)))
                    (default-value
                     'mode-line-format))))))
           (ace-window-display-mode -1)
           (set-default
            'mode-line-format
            original-mode-line)))"##;
    let expect = expect![[
        r#"OK (t ((ace-window-display-mode (:eval (window-parameter (selected-window) 'ace-window-path))) "middle") 1)"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}
