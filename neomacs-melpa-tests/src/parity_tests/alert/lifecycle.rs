use expect_test::expect;

use super::assert_alert_parity;

#[test]
fn alert_send_notification_runs_notifier_tracks_active_alert_and_schedules_removal() {
    let elisp_form = r##"(let ((alert-active-alerts nil)
                (alert-fade-time 7)
                calls)
         (with-temp-buffer
           (let ((origin (current-buffer))
                 (info '(:message "Lifecycle")))
             (cl-letf
                 (((symbol-function 'run-with-timer)
                   (lambda (seconds repeat function &rest args)
                     (push
                      (list 'timer seconds repeat
                            (eq function
                                #'alert-remove-when-active)
                            (length args))
                      calls)
                     'timer)))
               (alert-send-notification
                origin info
                (list
                 :notifier
                 (lambda (value)
                   (push
                    (list 'notify
                          (plist-get value :message))
                    calls))
                 :remover
                 (lambda (value)
                   (push
                    (list 'remove
                          (plist-get value :message))
                    calls)))
                nil nil)
               (list
                (mapcar
                 (lambda (entry)
                   (list
                    (eq (nth 0 entry) origin)
                    (nth 1 entry)
                    (functionp (nth 2 entry))))
                 alert-active-alerts)
                (memq #'alert-remove-on-command
                      post-command-hook)
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (((t (:message "Lifecycle") t)) (alert-remove-on-command t) ((notify "Lifecycle") (timer 7 nil t 2)))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_send_notification_persistent_alert_skips_timer_unless_never_persist() {
    let elisp_form = r##"(let ((alert-active-alerts nil)
                calls)
         (with-temp-buffer
           (cl-letf (((symbol-function 'run-with-timer)
                      (lambda (&rest args)
                        (push args calls)
                        'timer)))
             (alert-send-notification
              (current-buffer) '(:message "sticky")
              '(:notifier ignore :remover ignore)
              t nil)
             (alert-send-notification
              (current-buffer) '(:message "forced fade")
              '(:notifier ignore :remover ignore)
              t t)
             (list (length alert-active-alerts)
                   (length calls)
                   (mapcar
                    (lambda (args)
                      (list (nth 0 args)
                            (nth 1 args)
                            (eq (nth 2 args)
                                #'alert-remove-when-active)))
                    (nreverse calls))))))"##;
    let expect = expect!["OK (2 1 ((5 nil t)))"];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_remove_on_command_removes_only_current_buffer_alerts_and_calls_removers() {
    let elisp_form = r##"(let ((other (generate-new-buffer " *alert other*"))
                calls)
         (unwind-protect
             (with-temp-buffer
               (let* ((current (current-buffer))
                      (remove-a
                       (lambda (info)
                         (push
                          (list 'remove-a
                                (plist-get info :message))
                          calls)))
                      (remove-b
                       (lambda (info)
                         (push
                          (list 'remove-b
                                (plist-get info :message))
                          calls)))
                      (alert-active-alerts
                       (list
                        (list current '(:message "one") remove-a)
                        (list other '(:message "other") remove-b)
                        (list current '(:message "no remover") nil))))
                 (alert-remove-on-command)
                 (list
                  (mapcar
                   (lambda (entry)
                     (list
                      (eq (car entry) other)
                      (plist-get (nth 1 entry) :message)))
                   alert-active-alerts)
                  (nreverse calls))))
           (kill-buffer other)))"##;
    let expect = expect![[r#"OK (((t "other")) ((remove-a "one")))"#]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_remove_when_active_immediate_delayed_and_persistent_idle_paths_match() {
    let elisp_form = r##"(let ((alert-reveal-idle-time 15)
                (alert-persist-idle-time 900)
                (alert-fade-time 5)
                calls)
         (cl-labels
             ((probe (idle)
                (cl-letf
                    (((symbol-function 'current-idle-time)
                      (lambda () idle))
                     ((symbol-function 'run-with-timer)
                      (lambda (seconds repeat function
                                       remover info)
                        (push
                         (list 'timer seconds repeat
                               (eq function
                                   #'alert-remove-when-active)
                               (functionp remover)
                               info)
                         calls)
                        'timer)))
                  (alert-remove-when-active
                   (lambda (info)
                     (push
                      (list 'remove
                            (plist-get info :message))
                      calls)
                     'removed)
                   '(:message "idle")))))
           (list
            (probe nil)
            (probe '(20 0 0 0))
            (probe '(1000 0 0 0))
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (removed t t ((remove "idle")))"#]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_legacy_log_writes_timestamp_message_face_and_clear_behavior() {
    let elisp_form = r##"(let ((alert-severity-faces
                '((high . alert-high-face))))
         (unwind-protect
             (cl-letf (((symbol-function 'format-time-string)
                        (lambda (_) "09:45 AM")))
               (alert-legacy-log-notify
                "Build complete" 'high 14)
               (with-current-buffer "*Alerts*"
                 (list
                  (buffer-string)
                  (get-text-property
                   (save-excursion
                     (goto-char (point-min))
                     (search-forward "Build")
                     (match-beginning 0))
                   'face)
                  (alert-log-clear
                   '(:message "Build complete"))
                  (buffer-string))))
           (when (get-buffer "*Alerts*")
             (kill-buffer "*Alerts*"))))"##;
    let expect = expect![[
        r#"OK (#("09:45 AMBuild complete\n" 8 22 (face alert-high-face)) alert-high-face nil #("09:45 AMBuild complete\n" 8 22 (face alert-high-face)))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}
