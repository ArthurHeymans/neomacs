use expect_test::expect;

use super::assert_alert_parity;

#[test]
fn alert_basic_dispatch_ports_upstream_message_severity_title_category_and_data_cases() {
    let elisp_form = r##"(let (captured)
         (alert-define-style
          'test-capture
          :title "Capture"
          :notifier (lambda (info) (push info captured)))
         (let ((alert-default-style 'test-capture)
               (alert-user-configuration nil)
               (alert-internal-configuration nil)
               (alert-active-alerts nil)
               (alert-log-messages nil)
               (alert-hide-all-notifications nil))
           (with-temp-buffer
             (rename-buffer "alert-origin" t)
             (text-mode)
             (alert "basic")
             (alert "severe" :severity 'high)
             (alert "titled" :title "My Title")
             (alert "categorized" :category 'debug)
             (alert "data" :data '(custom payload) :id 'job)
             (mapcar
              (lambda (info)
                (list
                 (plist-get info :message)
                 (plist-get info :title)
                 (plist-get info :severity)
                 (plist-get info :category)
                 (plist-get info :mode)
                 (buffer-name (plist-get info :buffer))
                 (plist-get info :data)
                 (plist-get info :id)))
              (nreverse captured)))))"##;
    let expect = expect![[
        r#"OK (("basic" "alert-origin" normal nil text-mode "alert-origin" nil nil) ("severe" "alert-origin" high nil text-mode "alert-origin" nil nil) ("titled" "My Title" normal nil text-mode "alert-origin" nil nil) ("categorized" "alert-origin" normal debug text-mode "alert-origin" nil nil) ("data" "alert-origin" normal nil text-mode "alert-origin" (custom payload) job))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_hide_all_suppresses_rule_and_default_notifications_but_keeps_logging() {
    let elisp_form = r##"(let (calls)
         (alert-define-style
          'test-hidden
          :notifier (lambda (_) (push 'notified calls)))
         (let ((alert-default-style 'test-hidden)
               (alert-user-configuration nil)
               (alert-internal-configuration nil)
               (alert-active-alerts nil)
               (alert-log-messages t)
               (alert-hide-all-notifications t))
           (cl-letf (((symbol-function 'alert-log-notify)
                      (lambda (info)
                        (push
                         (list 'logged
                               (plist-get info :message))
                         calls))))
             (alert-add-rule
              :severity 'high :style 'test-hidden)
             (alert "Should only log" :severity 'high)
             (list (nreverse calls)
                   alert-active-alerts))))"##;
    let expect = expect![[
        r#"OK (((logged "Should only log") notified) (((:buffer #1="*scratch*") (:message "Should only log" :title "*scratch*" :icon nil :severity high :category nil :buffer (:buffer #1#) :persistent nil :mode lisp-interaction-mode :id nil :data nil :persistent nil) nil)))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_forced_style_bypasses_nonmatching_rules_and_default() {
    let elisp_form = r##"(let (calls)
         (dolist (style '(forced fallback configured))
           (alert-define-style
            style
            :title (symbol-name style)
            :notifier #'ignore))
         (let ((alert-default-style 'fallback)
               (alert-user-configuration
                '((((:severity urgent)) configured nil)))
               (alert-internal-configuration nil)
               (alert-log-messages nil))
           (cl-letf (((symbol-function 'alert-send-notification)
                      (lambda (_buffer info style-def &rest _)
                        (push
                         (list (plist-get style-def :title)
                               (plist-get info :severity))
                         calls))))
             (alert "Debug style"
                    :severity 'low
                    :style 'forced)
             (nreverse calls))))"##;
    let expect = expect![[r#"OK (("forced" low))"#]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_buffer_status_covers_selected_visible_buried_and_idle_states() {
    let elisp_form = r##"(let ((alert-reveal-idle-time 15))
         (cl-labels
             ((probe (window selected idle)
                (cl-letf
                    (((symbol-function 'get-buffer-window)
                      (lambda (&rest _) window))
                     ((symbol-function 'selected-window)
                      (lambda () selected))
                     ((symbol-function 'current-idle-time)
                      (lambda () idle)))
                  (alert-buffer-status))))
           (list
            (probe nil 'selected nil)
            (probe 'other 'selected nil)
            (probe 'selected 'selected nil)
            (probe 'selected 'selected '(20 0 0 0))
            (probe 'selected 'selected '(5 0 0 0)))))"##;
    let expect = expect!["OK (buried visible selected idle idle)"];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_message_style_treats_percent_signs_as_literal_user_content() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push
                       (list format-string args
                             (apply #'format format-string args))
                       messages)
                      'shown)))
           (list
            (alert-message-notify
             '(:message "Build 100%: %s remains literal"))
            (alert-message-remove nil)
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (shown shown (("%s" ("Build 100%: %s remains literal") "Build 100%: %s remains literal") ("" nil "")))"#
    ]];
    assert_alert_parity(elisp_form, expect);
}

#[test]
fn alert_momentary_style_formats_complete_info_at_origin_line_start() {
    let elisp_form = r##"(let (captured)
         (with-temp-buffer
           (insert "first line\nsecond line")
           (goto-char (point-max))
           (cl-letf
               (((symbol-function 'momentary-string-display)
                 (lambda (text position)
                   (setq captured
                         (list text position
                               (line-number-at-pos position)))
                   'displayed)))
             (list
              (alert-momentary-notify
               (list :buffer (current-buffer)
                     :title "Compile"
                     :message "Succeeded"
                     :severity 'normal
                     :category 'ci
                     :mode 'text-mode))
              captured))))"##;
    let expect = expect![[r#"OK (displayed ("Compile: Succeeded (normal/ci/text-mode)" 12 2))"#]];
    assert_alert_parity(elisp_form, expect);
}
