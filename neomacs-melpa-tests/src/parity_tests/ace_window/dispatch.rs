use super::assert_ace_window_parity;
use expect_test::expect;

#[test]
fn ace_window_dispatch_action_returns_first_matching_entry_or_nil() {
    let elisp_form = r##"(let ((aw-dispatch-alist
              '((120 first "First")
                (120 second "Second")
                (109 third))))
         (list
          (aw--dispatch-action 120)
          (aw--dispatch-action 109)
          (aw--dispatch-action 113)))"##;
    let expect = expect![[r#"OK ((120 first "First") (109 third) nil)"#]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_make_frame_builds_exact_default_explicit_and_zero_size_parameters() {
    let elisp_form = r##"(mapcar
         (lambda (size)
           (let ((aw-frame-size size)
                 (aw-frame-offset
                  '(13 . 23)))
             (setq ace-window--test-events
                   nil)
             (cl-letf
                 (((symbol-function
                    'frame-width)
                   (lambda
                       (&optional _frame)
                     120))
                  ((symbol-function
                    'frame-height)
                   (lambda
                       (&optional _frame)
                     60))
                  ((symbol-function
                    'frame-position)
                   (lambda
                       (&optional _frame)
                     '(100 . 200)))
                  ((symbol-function
                    'make-frame)
                   (lambda (parameters)
                     (push parameters
                           ace-window--test-events)
                     'new-frame)))
               (list
                size
                (aw-make-frame)
                (nreverse
                 ace-window--test-events)))))
         '(nil
           (80 . 40)
           (0 . 0)
           (0 . 40)))"##;
    let expect = expect![
        "OK ((nil new-frame ((#1=(no-focus-on-map . t) (left . 113) (top . 223)))) ((80 . 40) new-frame ((#1# (width . 80) (height . 80) (left . 113) (top . 223)))) ((0 . 0) new-frame ((#1# (width . 120) (height . 60) (left . 113) (top . 223)))) ((0 . 40) new-frame ((#1# (width . 120) (height . 0) (left . 113) (top . 223)))))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_use_frame_switches_to_source_window_before_creating_frame() {
    let elisp_form = r##"(progn
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'aw-switch-to-window)
               (lambda (window)
                 (push
                  (list 'switch window)
                  ace-window--test-events)
                 'switch-result))
              ((symbol-function 'aw-make-frame)
               (lambda ()
                 (push '(make-frame)
                       ace-window--test-events)
                 'new-frame)))
           (list
            (aw-use-frame
             'source-window)
            (nreverse
             ace-window--test-events))))"##;
    let expect = expect!["OK (new-frame ((switch source-window) (make-frame)))"];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_path_cleanup_removes_only_a_leading_dispatch_character() {
    let elisp_form = r##"(let ((aw-dispatch-alist
              '((120 fixture "Fixture")
                (109 other "Other"))))
         (mapcar
          (lambda (path)
            (setq avy-current-path path)
            (list
             path
             (aw-clean-up-avy-current-path)
             avy-current-path))
          '("" "x12" "m" "12x"
            "q12")))"##;
    let expect = expect![[
        r#"OK (("" nil "") ("x12" "12" "12") ("m" "" "") ("12x" nil "12x") ("q12" nil "q12"))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_default_dispatch_handles_mouse_cancel_and_avy_fallback_branches() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-branch
            (car fixture)
            ace-window--test-events
            nil
            avy-current-path
            "x12")
           (let ((aw-dispatch-alist
                  '((120 fixture
                         "Fixture"))))
             (cl-letf
                 (((symbol-function
                    'avy-mouse-event-window)
                   (lambda (character)
                     (push
                      (list
                       'mouse
                       character)
                      ace-window--test-events)
                     (and
                      (eq
                       ace-window--test-branch
                       'mouse)
                      'mouse-window)))
                  ((symbol-function
                    'avy-handler-default)
                   (lambda (character)
                     (push
                      (list
                       'avy
                       character
                       avy-dispatch-alist
                       avy-current-path)
                      ace-window--test-events)
                     'avy-result)))
               (list
                ace-window--test-branch
                (catch 'done
                  (aw-dispatch-default
                   (nth 1 fixture)))
                avy-current-path
                (nreverse
                 ace-window--test-events)))))
         (list
          (list 'mouse 'mouse-event)
          (list
           'cancel
           (aref (kbd "C-g") 0))
          (list 'fallback 113)))"##;
    let expect = expect![[
        r#"OK ((mouse mouse-window "x12" ((mouse mouse-event))) (cancel exit "x12" ((mouse 7))) (fallback avy-result "12" ((mouse 113) (avy 113 nil "12"))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_described_dispatch_action_sets_pending_action_and_mode_line() {
    let elisp_form = r##"(let ((aw-dispatch-alist
              '((120 fixture-action
                     "Delete Window")))
             (aw-action nil))
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'avy-mouse-event-window)
               (lambda (_character)
                 nil))
              ((symbol-function
                'aw-set-mode-line)
               (lambda (value)
                 (push
                  (list 'mode-line value)
                  ace-window--test-events)
                 'mode-result)))
           (list
            (aw-dispatch-default 120)
            aw-action
            (nreverse
             ace-window--test-events))))"##;
    let expect =
        expect![[r#"OK (fixture-action fixture-action ((mode-line " Ace - Delete Window")))"#]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_undescribed_dispatch_actions_call_commands_and_functions_then_exit() {
    let elisp_form = r##"(progn
         (setq ace-window--test-events nil)
         (cl-letf
             (((symbol-function
                'avy-mouse-event-window)
               (lambda (_character)
                 nil))
              ((symbol-function
                'ace-window--test-command)
               (lambda ()
                 (interactive)
                 (push 'command
                       ace-window--test-events)
                 'command-result))
              ((symbol-function
                'ace-window--test-function)
               (lambda ()
                 (push 'function
                       ace-window--test-events)
                 'function-result)))
           (mapcar
            (lambda (fixture)
              (let ((aw-dispatch-alist
                     (list fixture))
                    (aw-action nil))
                (list
                 fixture
                 (catch 'done
                   (aw-dispatch-default
                    (car fixture)))
                 aw-action
                 (nreverse
                  (prog1
                      ace-window--test-events
                    (setq
                     ace-window--test-events
                     nil))))))
            '((99
               ace-window--test-command)
              (102
               ace-window--test-function)))))"##;
    let expect = expect![
        "OK (((99 ace-window--test-command) exit nil (command)) ((102 ace-window--test-function) exit nil (function)))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_frame_dispatch_switches_or_applies_pending_action_from_start_window() {
    let elisp_form = r##"(mapcar
         (lambda (with-action)
           (setq ace-window--test-events nil)
           (let ((aw-make-frame-char 122)
                 (aw-action
                  (and with-action
                       'ace-window--test-action)))
             (cl-letf
                 (((symbol-function
                    'avy-mouse-event-window)
                   (lambda (_character)
                     nil))
                  ((symbol-function
                    'selected-window)
                   (lambda ()
                     'start-window))
                  ((symbol-function
                    'window-frame)
                   (lambda (window)
                     (if (eq window
                             'start-window)
                         'start-frame
                       'new-frame)))
                  ((symbol-function
                    'aw-make-frame)
                   (lambda ()
                     (push '(make-frame)
                           ace-window--test-events)
                     'new-frame))
                  ((symbol-function
                    'frame-selected-window)
                   (lambda (frame)
                     (push
                      (list
                       'frame-window
                       frame)
                      ace-window--test-events)
                     'new-window))
                  ((symbol-function
                    'select-frame-set-input-focus)
                   (lambda (frame)
                     (push
                      (list 'focus frame)
                      ace-window--test-events)))
                  ((symbol-function
                    'aw-switch-to-window)
                   (lambda (window)
                     (push
                      (list 'switch window)
                      ace-window--test-events)))
                  ((symbol-function
                    'ace-window--test-action)
                   (lambda (window)
                     (push
                      (list 'action window)
                      ace-window--test-events)
                     'action-result)))
               (list
                with-action
                (catch 'done
                  (aw-dispatch-default
                   122))
                aw-action
                (nreverse
                 ace-window--test-events)))))
         '(nil t))"##;
    let expect = expect![
        "OK ((nil exit nil (#1=(make-frame) (frame-window new-frame) (switch new-window))) (t exit ace-window--test-action (#1# (frame-window new-frame) (focus start-frame) (action new-window))))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}
