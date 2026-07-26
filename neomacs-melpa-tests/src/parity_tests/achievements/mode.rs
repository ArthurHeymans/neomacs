use super::assert_achievements_functions_parity;
use expect_test::expect;

#[test]
fn achievements_list_mode_initializes_tabulated_state_hooks_and_keymap() {
    let elisp_form = r##"(progn
         (defvar achievements-test-hook-runs)
         (let ((achievements-test-hook-runs
                0)
               (achievements-list-mode-hook
                (list
                 (lambda ()
                   (setq achievements-test-hook-runs
                         (1+
                          achievements-test-hook-runs))))))
           (with-temp-buffer
             (achievements-list-mode)
             (list
              achievements-test-hook-runs
              major-mode
              mode-name
              (append
               tabulated-list-format
               nil)
              tabulated-list-entries
              tabulated-list-padding
              show-trailing-whitespace
              (local-variable-p
               'show-trailing-whitespace)
              (memq
               #'achievements-update-score
               tabulated-list-revert-hook)
              (copy-tree
               (current-local-map))
              (keymap-parent
               (current-local-map))
              (and
               (boundp
                'tabulated-list-header-string)
               tabulated-list-header-string
               t)))))"##;
    let expect = expect![[
        r#"OK (1 achievements-list-mode "Achievements" (("E" 3 t :pad-right 0) ("Pts" 3 t :pad-right 1 :right-align t) ("Name" 30 t :pad-right 1) ("Description" 20 t :pad-right 1)) achievements-tabulated-list-entries 1 nil t (achievements-update-score) (keymap keymap (mouse-2 . mouse-select-window) (follow-link . mouse-face) (123 . tabulated-list-narrow-current-column) (125 . tabulated-list-widen-current-column) (83 . tabulated-list-sort) (M-right . tabulated-list-next-column) (M-left . tabulated-list-previous-column) (112 . previous-line) (110 . next-line) keymap (keymap (backtab . backward-button) (27 keymap (9 . backward-button)) (9 . forward-button)) keymap (103 . revert-buffer) (60 . beginning-of-buffer) (62 . end-of-buffer) (104 . describe-mode) (63 . describe-mode) (127 . scroll-down-command) (33554464 . scroll-down-command) (32 . scroll-up-command) (113 . quit-window) (57 . digit-argument) (56 . digit-argument) (55 . digit-argument) (54 . digit-argument) (53 . digit-argument) (52 . digit-argument) (51 . digit-argument) (50 . digit-argument) (49 . digit-argument) (48 . digit-argument) (45 . negative-argument) (remap keymap (self-insert-command . undefined))) (keymap (mouse-2 . mouse-select-window) (follow-link . mouse-face) (123 . tabulated-list-narrow-current-column) (125 . tabulated-list-widen-current-column) (83 . tabulated-list-sort) (M-right . tabulated-list-next-column) (M-left . tabulated-list-previous-column) (112 . previous-line) (110 . next-line) keymap (keymap (backtab . backward-button) (27 keymap (9 . backward-button)) (9 . forward-button)) keymap (103 . revert-buffer) (60 . beginning-of-buffer) (62 . end-of-buffer) (104 . describe-mode) (63 . describe-mode) (127 . scroll-down-command) (33554464 . scroll-down-command) (32 . scroll-up-command) (113 . quit-window) (57 . digit-argument) (56 . digit-argument) (55 . digit-argument) (54 . digit-argument) (53 . digit-argument) (52 . digit-argument) (51 . digit-argument) (50 . digit-argument) (49 . digit-argument) (48 . digit-argument) (45 . negative-argument) (remap keymap (self-insert-command . undefined))) nil)"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_setup_post_command_hook_selects_only_unearned_callable_records() {
    let elisp_form = r##"(let* ((eligible
                  (make-achievement
                   "Eligible"
                   "Fixture"
                   :post-command
                   (lambda () t)))
                 (earned
                  (make-achievement
                   "Earned"
                   "Fixture"
                   :post-command
                   (lambda () t)))
                 (no-post
                  (make-achievement
                   "No post"
                   "Fixture"))
                 (noncallable
                  (make-achievement
                   "Noncallable"
                   "Fixture"
                   :post-command
                   'fixture-symbol))
                 (achievements-list
                  (list
                   eligible
                   earned
                   no-post
                   noncallable))
                 (achievements-post-command-list
                  '(stale)))
         (setf
          (emacs-achievement-predicate
           eligible)
          (lambda () nil)
          (emacs-achievement-predicate
           earned)
          t
          (emacs-achievement-predicate
           no-post)
          (lambda () nil)
          (emacs-achievement-predicate
           noncallable)
          nil)
         (list
          (achievements-setup-post-command-hook)
          (mapcar
           #'emacs-achievement-name
           achievements-post-command-list)))"##;
    let expect = expect![[r#"OK (nil ("Noncallable" "Eligible"))"#]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_post_command_function_earns_removes_and_retains_exact_records() {
    let elisp_form = r##"(let* ((earned
                  (make-achievement
                   "Earned now"
                   "Fixture"
                   :post-command
                   (lambda () t)))
                 (waiting
                  (make-achievement
                   "Waiting"
                   "Fixture"
                   :post-command
                   (lambda () nil)))
                 (invalid
                  (make-achievement
                   "Invalid"
                   "Fixture"
                   :post-command
                   'not-callable))
                 (achievements-post-command-list
                  (list
                   earned
                   waiting
                   invalid)))
         (setq
          achievements--test-events
          nil)
         (cl-letf
             (((symbol-function
                'achievements-earned-message)
               (lambda (achievement)
                 (push
                  (list
                   'message
                   (emacs-achievement-name
                    achievement))
                  achievements--test-events))))
           (list
            (achievements-post-command-function)
            (mapcar
             #'emacs-achievement-name
             achievements-post-command-list)
            (mapcar
             (lambda (achievement)
               (list
                (emacs-achievement-name
                 achievement)
                (eq
                 (emacs-achievement-predicate
                  achievement)
                 t)))
             (list earned
                   waiting
                   invalid))
            (nreverse
             achievements--test-events))))"##;
    let expect = expect![[
        r#"OK (nil ("Waiting") (("Earned now" t) ("Waiting" nil) ("Invalid" nil)) ((message "Earned now")))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_mode_lifecycle_covers_new_existing_and_cancelled_timers() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            achievements--test-events
            nil)
           (let ((achievements-mode
                  (nth 0 fixture))
                 (achievements-timer
                  (nth 1 fixture))
                 (achievements-idle-time
                  17))
             (cl-letf
                 (((symbol-function
                    'run-with-idle-timer)
                   (lambda
                       (seconds repeat
                        function
                        &rest arguments)
                     (push
                      (list
                       'run-timer
                       seconds
                       repeat
                       function
                       arguments)
                      achievements--test-events)
                     'new-timer))
                  ((symbol-function
                    'cancel-timer)
                   (lambda (timer)
                     (push
                      (list
                       'cancel
                       timer)
                      achievements--test-events)
                     'cancel-result))
                  ((symbol-function
                    'achievements-setup-post-command-hook)
                   (lambda ()
                     (push
                      '(setup-post-command)
                      achievements--test-events)))
                  ((symbol-function 'add-hook)
                   (lambda
                       (hook function
                        &optional append local)
                     (push
                      (list
                       'add-hook
                       hook
                       function
                       append
                       local)
                      achievements--test-events)))
                  ((symbol-function
                    'remove-hook)
                   (lambda
                       (hook function
                        &optional local)
                     (push
                      (list
                       'remove-hook
                       hook
                       function
                       local)
                      achievements--test-events))))
               (list
                fixture
                (condition-case error
                    (list
                     'ok
                     (achievements-mode
                      (nth 2 fixture)))
                  (error
                   (list 'error error)))
                achievements-mode
                achievements-timer
                (nreverse
                 achievements--test-events)))))
         '((nil nil 1)
           (nil existing-timer 1)
           (t existing-timer -1)
           (t nil -1)))"##;
    let expect = expect![
        "OK (((nil nil 1) (ok t) t new-timer ((run-timer 17 t achievements-update-score nil) #1=(setup-post-command) (add-hook post-command-hook achievements-post-command-function nil nil))) ((nil existing-timer 1) (ok t) t existing-timer (#1# (add-hook post-command-hook achievements-post-command-function nil nil))) ((t existing-timer -1) (ok nil) nil cancel-result ((cancel existing-timer) (remove-hook post-command-hook achievements-post-command-function nil))) ((t nil -1) (ok nil) nil cancel-result ((cancel nil) (remove-hook post-command-hook achievements-post-command-function nil))))"
    ];
    assert_achievements_functions_parity(elisp_form, expect);
}
