use super::assert_ace_window_parity;
use expect_test::expect;

#[test]
fn ace_window_select_with_zero_or_one_candidate_returns_start_or_candidate_and_applies_action() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-window-list
            (nth 0 fixture))
           (let ((aw-dispatch-always nil))
             (cl-letf
                 (((symbol-function
                    'selected-window)
                   (lambda ()
                     'start-window))
                  ((symbol-function
                    'aw-window-list)
                   (lambda ()
                     ace-window--test-window-list))
                  ((symbol-function
                    'ace-window--test-action)
                   (lambda (window)
                     (push
                      (list 'action window)
                      ace-window--test-events)
                     (list
                      'action-result
                      window))))
               (list
                fixture
                (aw-select
                 " Fixture"
                 (and
                  (nth 1 fixture)
                  #'ace-window--test-action))
                (nreverse
                 ace-window--test-events)))))
         '((nil nil)
           ((only-window) nil)
           (nil t)
           ((only-window) t)))"##;
    let expect = expect![
        "OK (((nil nil) start-window nil) (((only-window) nil) only-window nil) ((nil t) (action-result start-window) ((action start-window))) (((only-window) t) (action-result only-window) ((action only-window))))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_select_single_candidate_dispatch_always_handles_exit_and_replacement_action() {
    let elisp_form = r##"(mapcar
         (lambda (dispatch-result)
           (setq ace-window--test-events nil)
           (let ((aw-dispatch-always t)
                 (aw-dispatch-function
                  'ace-window--test-dispatch))
             (cl-letf
                 (((symbol-function
                    'selected-window)
                   (lambda ()
                     'start-window))
                  ((symbol-function
                    'aw-window-list)
                   (lambda ()
                     '(only-window)))
                  ((symbol-function
                    'read-char)
                   (lambda ()
                     (push '(read)
                           ace-window--test-events)
                     113))
                  ((symbol-function
                    'ace-window--test-dispatch)
                   (lambda (character)
                     (push
                      (list
                       'dispatch
                       character)
                      ace-window--test-events)
                     ace-window--test-dispatch-result))
                  ((symbol-function 'aw--done)
                   (lambda ()
                     (push '(done)
                           ace-window--test-events)))
                  ((symbol-function
                    'ace-window--test-action)
                   (lambda (window)
                     (push
                      (list 'action window)
                      ace-window--test-events)
                     'action-result)))
               (setq
                ace-window--test-dispatch-result
                dispatch-result)
               (list
                dispatch-result
                (aw-select " Fixture")
                aw-action
                (nreverse
                 ace-window--test-events)))))
         '(exit
           ace-window--test-action))"##;
    let expect = expect![
        "OK ((exit only-window nil (#1=(read) (dispatch 113) #2=(done))) (ace-window--test-action action-result ace-window--test-action (#1# (dispatch 113) #2# (action only-window))))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_select_fast_path_uses_scope_and_skips_unavailable_next_windows() {
    let elisp_form = r##"(mapcar
         (lambda (scope)
           (setq
            ace-window--test-events
            nil
            ace-window--test-next
            '(outside target-window))
           (let ((aw-scope scope)
                 (aw-dispatch-always nil)
                 (aw-dispatch-when-more-than
                  2)
                 (aw-ignore-current nil))
             (cl-letf
                 (((symbol-function
                    'selected-window)
                   (lambda ()
                     'start-window))
                  ((symbol-function
                    'aw-window-list)
                   (lambda ()
                     '(start-window
                       target-window)))
                  ((symbol-function
                    'aw-ignored-p)
                   (lambda (window)
                     (eq window 'outside)))
                  ((symbol-function
                    'next-window)
                   (lambda
                       (&optional window
                        _minibuffer all-frames)
                     (let ((value
                            (pop
                             ace-window--test-next)))
                       (push
                        (list
                         'next
                         window
                         all-frames
                         value)
                        ace-window--test-events)
                       value))))
               (list
                scope
                (aw-select " Fixture")
                (nreverse
                 ace-window--test-events)))))
         '(visible global frame))"##;
    let expect = expect![
        "OK ((visible target-window ((next nil visible outside) (next outside visible target-window))) (global target-window ((next nil visible outside) (next outside visible target-window))) (frame target-window ((next nil frame outside) (next outside frame target-window))))"
    ];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_select_ignored_start_and_ignore_current_guards_force_full_path() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil)
           (let ((aw-dispatch-always
                  nil)
                 (aw-dispatch-when-more-than
                  2)
                 (aw-ignore-current
                  (nth 1 fixture))
                 (aw-keys '(49 50)))
             (cl-letf
                 (((symbol-function
                    'selected-window)
                   (lambda ()
                     'start-window))
                  ((symbol-function
                    'aw-window-list)
                   (lambda ()
                     '(w1 w2)))
                  ((symbol-function
                    'aw-ignored-p)
                   (lambda (window)
                     (and
                      (eq
                       (car fixture)
                       'ignored-start)
                      (eq
                       window
                       'start-window))))
                  ((symbol-function
                    'next-window)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'next-window
                       arguments)
                      ace-window--test-events)
                     'unexpected-fast-path))
                  ((symbol-function
                    'aw-offset)
                   (lambda (window)
                     (cdr
                      (assq
                       window
                       '((w1 . 11)
                         (w2 . 22))))))
                  ((symbol-function
                    'aw--make-backgrounds)
                   (lambda (windows)
                     (push
                      (list
                       'backgrounds
                       windows)
                      ace-window--test-events)))
                  ((symbol-function
                    'aw-set-mode-line)
                   (lambda (value)
                     (push
                      (list
                       'mode-line
                       value)
                      ace-window--test-events)))
                  ((symbol-function
                    'remove-hook)
                   (lambda (hook function)
                     (push
                      (list
                       'remove-hook
                       hook
                       function)
                      ace-window--test-events)))
                  ((symbol-function
                    'avy-tree)
                   (lambda
                       (candidates keys)
                     (push
                      (list
                       'tree
                       candidates
                       keys)
                      ace-window--test-events)
                     'fixture-tree))
                  ((symbol-function
                    'avy-read)
                   (lambda
                       (tree _lead
                        _remove)
                     (push
                      (list
                       'read
                       tree)
                      ace-window--test-events)
                     '(22 . w2)))
                  ((symbol-function 'aw--done)
                   (lambda ()
                     (push '(done)
                           ace-window--test-events))))
               (list
                fixture
                (aw-select " Fixture")
                (nreverse
                 ace-window--test-events)))))
         '((ignored-start nil)
           (ignore-current t)))"##;
    let expect = expect![[
        r#"OK (((ignored-start nil) w2 ((backgrounds #1=(w1 w2)) (mode-line " Fixture") (remove-hook post-command-hook helm--maybe-update-keymap) (tree ((11 . w1) (22 . w2)) #2=(49 50)) (read fixture-tree) #3=(done))) ((ignore-current t) w2 ((backgrounds #1#) (mode-line " Fixture") (remove-hook post-command-hook helm--maybe-update-keymap) (tree ((11 . w1) (22 . w2)) #2#) (read fixture-tree) #3#)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_select_full_path_builds_candidates_binds_avy_and_cleans_up_all_results() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil
            ace-window--test-result
            (nth 0 fixture))
           (let ((aw-scope 'frame)
                 (aw-dispatch-always nil)
                 (aw-dispatch-when-more-than
                  2)
                 (aw-ignore-current nil)
                 (aw-keys '(49 50 51))
                 (aw-dispatch-function
                  'fixture-dispatch)
                 (aw-translate-char-function
                  'fixture-translate)
                 (aw--lead-overlay-fn
                  'fixture-lead)
                 (aw--remove-leading-chars-fn
                  'fixture-remove)
                 (ace-window-display-mode
                  (nth 1 fixture))
                 (aw-display-mode-overlay
                  (nth 2 fixture)))
             (cl-letf
                 (((symbol-function
                    'selected-window)
                   (lambda ()
                     'start-window))
                  ((symbol-function
                    'aw-window-list)
                   (lambda ()
                     '(w1 w2 w3)))
                  ((symbol-function
                    'aw-ignored-p)
                   (lambda (_window)
                     nil))
                  ((symbol-function 'aw-offset)
                   (lambda (window)
                     (cdr
                      (assq
                       window
                       '((w1 . 11)
                         (w2 . 22)
                         (w3 . 33))))))
                  ((symbol-function
                    'aw--make-backgrounds)
                   (lambda (windows)
                     (push
                      (list
                       'backgrounds
                       windows)
                      ace-window--test-events)))
                  ((symbol-function
                    'aw-set-mode-line)
                   (lambda (value)
                     (push
                      (list
                       'mode-line
                       value)
                      ace-window--test-events)))
                  ((symbol-function 'remove-hook)
                   (lambda (hook function)
                     (push
                      (list
                       'remove-hook
                       hook
                       function)
                      ace-window--test-events)))
                  ((symbol-function 'avy-tree)
                   (lambda (candidates keys)
                     (push
                      (list
                       'tree
                       candidates
                       keys)
                      ace-window--test-events)
                     'fixture-tree))
                  ((symbol-function 'avy-read)
                   (lambda
                       (tree lead remove)
                     (push
                      (list
                       'read
                       tree
                       (if (eq
                            lead
                            aw--lead-overlay-fn)
                           'lead
                         'no-overlay)
                       (eq
                        remove
                        aw--remove-leading-chars-fn)
                       avy-handler-function
                       avy-translate-char-function
                       transient-mark-mode)
                      ace-window--test-events)
                     ace-window--test-result))
                  ((symbol-function 'aw--done)
                   (lambda ()
                     (push '(done)
                           ace-window--test-events)))
                  ((symbol-function
                    'ace-window--test-action)
                   (lambda (window)
                     (push
                      (list 'action window)
                      ace-window--test-events)
                     (list
                      'action-result
                      window))))
               (list
                fixture
                (aw-select
                 " Fixture"
                 (and
                  (nth 3 fixture)
                  #'ace-window--test-action))
                aw-action
                (nreverse
                 ace-window--test-events)))))
         '(((22 . w2) nil t nil)
           (exit nil t t)
           (nil t nil nil)
           ((33 . w3) t t t)))"##;
    let expect = expect![[
        r#"OK ((((22 . w2) nil t nil) w2 nil ((backgrounds #1=(w1 w2 w3)) (mode-line " Fixture") (remove-hook post-command-hook helm--maybe-update-keymap) (tree ((11 . w1) (22 . w2) (33 . w3)) #2=(49 50 51)) (read fixture-tree lead t fixture-dispatch fixture-translate nil) #3=(done))) ((exit nil t t) nil nil ((backgrounds #1#) (mode-line " Fixture") (remove-hook post-command-hook helm--maybe-update-keymap) (tree ((11 . w1) (22 . w2) (33 . w3)) #2#) (read fixture-tree lead t fixture-dispatch fixture-translate nil) #3#)) ((nil t nil nil) start-window nil ((backgrounds #1#) (mode-line " Fixture") (remove-hook post-command-hook helm--maybe-update-keymap) (tree ((11 . w1) (22 . w2) (33 . w3)) #2#) (read fixture-tree no-overlay t fixture-dispatch fixture-translate nil) #3#)) (((33 . w3) t t t) (action-result w3) ace-window--test-action ((backgrounds #1#) (mode-line " Fixture") (remove-hook post-command-hook helm--maybe-update-keymap) (tree ((11 . w1) (22 . w2) (33 . w3)) #2#) (read fixture-tree lead t fixture-dispatch fixture-translate nil) #3# (action w3))))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}

#[test]
fn ace_window_select_preserves_dispatch_and_avy_signals_while_always_cleaning_up() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-window--test-events
            nil)
           (let ((aw-dispatch-always
                  (eq fixture 'dispatch))
                 (aw-dispatch-function
                  'ace-window--test-dispatch)
                 (aw-dispatch-when-more-than
                  2)
                 (aw-ignore-current nil))
             (cl-letf
                 (((symbol-function
                    'selected-window)
                   (lambda ()
                     'start-window))
                  ((symbol-function
                    'aw-window-list)
                   (lambda ()
                     (if
                         (eq fixture
                             'dispatch)
                         '(only-window)
                       '(w1 w2 w3))))
                  ((symbol-function
                    'read-char)
                   (lambda ()
                     (push '(read)
                           ace-window--test-events)
                     113))
                  ((symbol-function
                    'ace-window--test-dispatch)
                   (lambda (character)
                     (push
                      (list
                       'dispatch
                       character)
                      ace-window--test-events)
                     (error
                      "dispatch boom")))
                  ((symbol-function
                    'aw-ignored-p)
                   (lambda (_window)
                     nil))
                  ((symbol-function
                    'aw-offset)
                   (lambda (window)
                     (cdr
                      (assq
                       window
                       '((w1 . 11)
                         (w2 . 22)
                         (w3 . 33))))))
                  ((symbol-function
                    'aw--make-backgrounds)
                   (lambda (windows)
                     (push
                      (list
                       'backgrounds
                       windows)
                      ace-window--test-events)))
                  ((symbol-function
                    'aw-set-mode-line)
                   (lambda (value)
                     (push
                      (list
                       'mode-line
                       value)
                      ace-window--test-events)))
                  ((symbol-function
                    'remove-hook)
                   (lambda (hook function)
                     (push
                      (list
                       'remove-hook
                       hook
                       function)
                      ace-window--test-events)))
                  ((symbol-function
                    'avy-tree)
                   (lambda
                       (candidates keys)
                     (push
                      (list
                       'tree
                       candidates
                       keys)
                      ace-window--test-events)
                     'fixture-tree))
                  ((symbol-function
                    'avy-read)
                   (lambda
                       (_tree _lead
                        _remove)
                     (push '(read-avy)
                           ace-window--test-events)
                     (error "avy boom")))
                  ((symbol-function 'aw--done)
                   (lambda ()
                     (push '(done)
                           ace-window--test-events))))
               (condition-case error
                   (list
                    'ok
                    (aw-select
                     " Fixture"
                     #'ace-window--test-action))
                 (error
                  (list
                   'error
                   error
                   aw-action
                   (nreverse
                    ace-window--test-events)))))))
         '(dispatch avy))"##;
    let expect = expect![[
        r#"OK ((error (error "dispatch boom") ace-window--test-action ((read) (dispatch 113) #1=(done))) (error (error "avy boom") ace-window--test-action ((backgrounds (w1 w2 w3)) (mode-line " Fixture") (remove-hook post-command-hook helm--maybe-update-keymap) (tree ((11 . w1) (22 . w2) (33 . w3)) (49 50 51 52 53 54 55 56 57)) (read-avy) #1#)))"#
    ]];
    assert_ace_window_parity(elisp_form, expect);
}
