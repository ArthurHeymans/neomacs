use expect_test::expect;

use super::assert_activity_watch_mode_parity;

#[test]
fn activity_watch_mode_start_timer_creates_each_timer_once_with_exact_schedules() {
    let elisp_form = r##"(let ((activity-watch-timer
                nil)
               (activity-watch-idle-timer
                nil)
               calls)
         (cl-letf
             (((symbol-function
                'run-at-time)
               (lambda
                 (&rest arguments)
                 (push
                  (cons
                   'run-at-time
                   arguments)
                  calls)
                 'heartbeat-timer))
              ((symbol-function
                'run-with-idle-timer)
               (lambda
                 (&rest arguments)
                 (push
                  (cons
                   'run-with-idle-timer
                   arguments)
                  calls)
                 'idle-timer)))
           (list
            (activity-watch--start-timer)
            activity-watch-timer
            activity-watch-idle-timer
            (activity-watch--start-timer)
            activity-watch-timer
            activity-watch-idle-timer
            (nreverse calls))))"##;
    let expect = expect![
        "OK (idle-timer heartbeat-timer idle-timer nil heartbeat-timer idle-timer ((run-at-time t 2 activity-watch--save) (run-with-idle-timer 30 t activity-watch--stop-timer)))"
    ];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_stop_timer_functions_cancel_present_timers_reset_and_noop() {
    let elisp_form = r##"(let ((activity-watch-timer
                'heartbeat-timer)
               (activity-watch-idle-timer
                'idle-timer)
               calls)
         (cl-letf
             (((symbol-function
                'cancel-timer)
               (lambda (timer)
                 (push timer calls)
                 'cancelled)))
           (list
            (activity-watch--stop-timer)
            activity-watch-timer
            (activity-watch--stop-timer)
            (activity-watch--stop-idle-timer)
            activity-watch-idle-timer
            (activity-watch--stop-idle-timer)
            (nreverse calls))))"##;
    let expect = expect!["OK (nil nil nil nil nil nil (heartbeat-timer idle-timer))"];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_bind_and_unbind_hooks_are_buffer_local_exact_and_idempotent() {
    let elisp_form = r##"(with-temp-buffer
         (let ((global-before
                (mapcar
                 (lambda (hook)
                   (cons
                    hook
                    (default-value hook)))
                 '(pre-command-hook
                   after-save-hook
                   auto-save-hook
                   first-change-hook))))
           (activity-watch--bind-hooks)
           (activity-watch--bind-hooks)
           (let ((bound
                  (mapcar
                   (lambda (hook)
                     (list
                      hook
                      (local-variable-p hook)
                      (symbol-value hook)
                      (default-value hook)))
                   '(pre-command-hook
                     after-save-hook
                     auto-save-hook
                     first-change-hook))))
             (activity-watch--unbind-hooks)
             (let ((unbound
                    (mapcar
                     (lambda (hook)
                       (list
                        hook
                        (local-variable-p hook)
                        (symbol-value hook)
                        (default-value hook)))
                     '(pre-command-hook
                       after-save-hook
                       auto-save-hook
                       first-change-hook))))
               (list
                global-before
                bound
                unbound)))))"##;
    let expect = expect![
        "OK (((pre-command-hook . #1=(tooltip-hide)) (after-save-hook) (auto-save-hook) (first-change-hook)) ((pre-command-hook t (activity-watch--start-timer t) #1#) (after-save-hook t (activity-watch--save t) nil) (auto-save-hook t (activity-watch--save t) nil) (first-change-hook t (activity-watch--save t) nil)) ((pre-command-hook nil #1# #1#) (after-save-hook nil nil nil) (auto-save-hook nil nil nil) (first-change-hook nil nil nil)))"
    ];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_turn_on_covers_deferred_ready_and_retry_initialization_paths() {
    let elisp_form = r##"(let (calls
               init-finished)
         (cl-letf
             (((symbol-function
                'run-at-time)
               (lambda
                 (&rest arguments)
                 (push
                  (cons
                   'run-at-time
                   arguments)
                  calls)
                 'scheduled))
              ((symbol-function
                'activity-watch--init)
               (lambda ()
                 (push
                  'init
                  calls)
                 (setq activity-watch-init-finished
                       init-finished)
                 'initialized))
              ((symbol-function
                'activity-watch--bind-hooks)
               (lambda ()
                 (push
                  'bind
                  calls)
                 'bound))
              ((symbol-function
                'activity-watch--start-timer)
               (lambda ()
                 (push
                  'start
                  calls)
                 'started)))
           (let ((deferred
                  (activity-watch-turn-on
                   t)))
             (setq init-finished
                   t)
             (let ((ready
                    (activity-watch-turn-on
                     nil)))
               (setq init-finished
                     nil)
               (let ((retry
                      (activity-watch-turn-on
                       nil)))
                 (list
                  deferred
                  ready
                  retry
                  (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (scheduled started scheduled ((run-at-time "1 sec" nil activity-watch-turn-on nil) init bind start init (run-at-time "1 sec" nil activity-watch-turn-on nil)))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_turn_off_unbinds_and_stops_both_timers_in_order() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'activity-watch--unbind-hooks)
               (lambda ()
                 (push
                  'unbind
                  calls)
                 'unbound))
              ((symbol-function
                'activity-watch--stop-timer)
               (lambda ()
                 (push
                  'stop-timer
                  calls)
                 'timer-stopped))
              ((symbol-function
                'activity-watch--stop-idle-timer)
               (lambda ()
                 (push
                  'stop-idle
                  calls)
                 'idle-stopped)))
           (list
            (activity-watch-turn-off)
            (nreverse calls))))"##;
    let expect = expect!["OK (idle-stopped (unbind stop-timer stop-idle))"];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_refresh_project_command_forces_resolution_and_returns_value() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'activity-watch--get-project)
               (lambda
                 (&optional refresh)
                 (push refresh calls)
                 "refreshed-project")))
           (list
            (commandp
             'activity-watch-refresh-project-name)
            (interactive-form
             'activity-watch-refresh-project-name)
            (activity-watch-refresh-project-name)
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (t (interactive nil) "refreshed-project" (t))"#]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_local_minor_mode_interactive_and_batch_transitions_match() {
    let elisp_form = r##"(let ((activity-watch-mode-hook
                (copy-sequence
                 activity-watch-mode-hook))
               calls)
         (add-hook
          'activity-watch-mode-hook
          (lambda ()
            (push
             (list
              'local-hook
              activity-watch-mode)
             calls)))
         (cl-letf
             (((symbol-function
                'activity-watch-turn-on)
               (lambda (defer)
                 (push
                  (list
                   'turn-on
                   defer)
                  calls)
                 'turned-on))
              ((symbol-function
                'activity-watch-turn-off)
               (lambda ()
                 (push
                  'turn-off
                  calls)
                 'turned-off)))
           (with-temp-buffer
             (let ((noninteractive
                    nil))
               (let ((enabled
                      (activity-watch-mode
                       1)))
                 (let ((disabled
                        (activity-watch-mode
                         0)))
                   (let ((noninteractive
                          t))
                     (let ((batch-enable
                            (activity-watch-mode
                             1)))
                       (list
                        enabled
                        disabled
                        batch-enable
                        activity-watch-mode
                        activity-watch-mode--set-explicitly
                        (nreverse calls))))))))))"##;
    let expect = expect![
        "OK (t nil nil nil t ((turn-on t) (local-hook t) turn-off (local-hook nil) (local-hook nil)))"
    ];
    assert_activity_watch_mode_parity(elisp_form, expect);
}

#[test]
fn activity_watch_mode_globalized_mode_invokes_local_mode_and_manages_global_hooks() {
    let elisp_form = r##"(let ((first
                (generate-new-buffer
                 " *activity-watch-first*"))
               (second
                (generate-new-buffer
                 " *activity-watch-second*"))
               (third
                (generate-new-buffer
                 " *activity-watch-third*"))
               (listed-buffers
                nil)
               (activity-watch-mode-hook
                (copy-sequence
                 activity-watch-mode-hook))
               (global-activity-watch-mode-hook
                nil)
               calls)
         (unwind-protect
             (progn
               (setq listed-buffers
                     (list
                      first
                      second))
               (add-hook
                'activity-watch-mode-hook
                (lambda ()
                  (push
                   (list
                    'local-hook
                    (buffer-name)
                    activity-watch-mode
                    activity-watch-mode--set-explicitly
                    activity-watch-mode--suppress-set-explicitly)
                   calls)))
               (add-hook
                'global-activity-watch-mode-hook
                (lambda ()
                  (push
                   (list
                    'global-hook
                    global-activity-watch-mode)
                   calls)))
               (cl-letf
                   (((symbol-function
                     'buffer-list)
                     (lambda ()
                       listed-buffers))
                    ((symbol-function
                      'activity-watch-turn-on)
                     (lambda (defer)
                       (push
                        (list
                         'turn-on
                         (buffer-name)
                         defer
                         activity-watch-mode--suppress-set-explicitly)
                        calls)))
                    ((symbol-function
                      'activity-watch-turn-off)
                     (lambda ()
                       (push
                        (list
                         'turn-off
                         (buffer-name)
                         activity-watch-mode--suppress-set-explicitly)
                        calls))))
                 (let ((noninteractive
                        nil))
                   (let ((enabled
                          (global-activity-watch-mode
                           1)))
                     (let ((after-enable
                            (list
                             global-activity-watch-mode
                             (and
                              (memq
                               'global-activity-watch-mode-enable-in-buffer
                               after-change-major-mode-hook)
                              t)
                             (mapcar
                              (lambda (buffer)
                                (with-current-buffer buffer
                                  (list
                                   (buffer-name)
                                   activity-watch-mode
                                   activity-watch-mode--set-explicitly
                                   activity-watch-mode--suppress-set-explicitly)))
                              (list
                               first
                               second))
                             (nreverse calls))))
                       (setq calls
                             nil)
                       (with-current-buffer third
                         (global-activity-watch-mode-enable-in-buffer))
                       (let ((after-fresh-buffer
                              (list
                               (with-current-buffer third
                                 (list
                                  activity-watch-mode
                                  activity-watch-mode--set-explicitly
                                  activity-watch-mode--suppress-set-explicitly))
                               (nreverse calls))))
                         (setq calls
                               nil)
                         (with-current-buffer first
                           (activity-watch-mode
                            0)
                           (global-activity-watch-mode-enable-in-buffer))
                         (let ((after-explicit-disable
                                (list
                                 (with-current-buffer first
                                   (list
                                    activity-watch-mode
                                    activity-watch-mode--set-explicitly
                                    activity-watch-mode--suppress-set-explicitly))
                                 (nreverse calls))))
                           (setq calls
                                 nil
                                 listed-buffers
                                 (list
                                  first
                                  second
                                  third))
                           (let ((disabled
                                  (global-activity-watch-mode
                                   0)))
                             (list
                              (fboundp
                               'global-activity-watch-mode-enable-in-buffer)
                              (help-function-arglist
                               'global-activity-watch-mode-enable-in-buffer
                               t)
                              enabled
                              after-enable
                              after-fresh-buffer
                              after-explicit-disable
                              disabled
                              global-activity-watch-mode
                              (and
                               (memq
                                'global-activity-watch-mode-enable-in-buffer
                                after-change-major-mode-hook)
                               t)
                              (mapcar
                               (lambda (buffer)
                                 (with-current-buffer buffer
                                   (list
                                    (buffer-name)
                                    activity-watch-mode
                                    activity-watch-mode--set-explicitly
                                    activity-watch-mode--suppress-set-explicitly)))
                               (list
                                first
                                second
                                third))
                              (nreverse calls))))))))))
           (when
               (buffer-live-p first)
             (kill-buffer first))
           (when
               (buffer-live-p second)
             (kill-buffer second))
           (when
               (buffer-live-p third)
             (kill-buffer third))))"##;
    let expect = expect![[
        r#"OK (t nil t (t t ((" *activity-watch-first*" t t nil) (" *activity-watch-second*" t t nil)) ((turn-on " *activity-watch-first*" t nil) (local-hook " *activity-watch-first*" t nil nil) (turn-on " *activity-watch-second*" t nil) (local-hook " *activity-watch-second*" t nil nil) (global-hook t))) ((t nil nil) ((turn-on " *activity-watch-third*" t t) (local-hook " *activity-watch-third*" t nil t))) ((nil t nil) ((turn-off " *activity-watch-first*" nil) (local-hook " *activity-watch-first*" nil t nil))) nil nil nil ((" *activity-watch-first*" nil t nil) (" *activity-watch-second*" nil t nil) (" *activity-watch-third*" nil t nil)) ((turn-off " *activity-watch-second*" nil) (local-hook " *activity-watch-second*" nil t nil) (turn-off " *activity-watch-third*" nil) (local-hook " *activity-watch-third*" nil nil nil) (global-hook nil)))"#
    ]];
    assert_activity_watch_mode_parity(elisp_form, expect);
}
