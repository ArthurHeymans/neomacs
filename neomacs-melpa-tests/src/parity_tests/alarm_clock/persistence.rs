use expect_test::expect;

use super::assert_alarm_clock_parity;

#[test]
fn alarm_clock_formatted_cache_serializes_only_unexpired_time_and_message_fields() {
    let elisp_form = r##"(let ((alarms
                (list
                 (list :time (encode-time 5 4 3 2 1 2030)
                       :message "Wake"
                       :timer 'private-a)
                 (list :time (encode-time 9 8 7 6 5 2031)
                       :message "Ship"
                       :timer 'private-b))))
         (cl-letf (((symbol-function 'alarm-clock--unexpired-alarms)
                    (lambda () alarms)))
           (alarm-clock--formatted-cache)))"##;
    let expect = expect![[
        r#"OK ((:time "2030-01-02T03:04:05+0000" :message "Wake") (:time "2031-05-06T07:08:09+0000" :message "Ship"))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_save_writes_reloadable_cache_below_the_oracle_sandbox() {
    let elisp_form = r##"(let* ((root (expand-file-name "save"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (alarm-clock-cache-file
                 (expand-file-name "alarms.el" root))
                (payload
                 '((:time "2030-01-02T03:04:05+0000"
                    :message "Wake")
                   (:time "2031-05-06T07:08:09+0000"
                    :message "Ship"))))
         (make-directory root t)
         (cl-letf (((symbol-function 'alarm-clock--formatted-cache)
                    (lambda () payload)))
           (alarm-clock-save)
           (with-temp-buffer
             (insert-file-contents alarm-clock-cache-file)
             (let ((text (buffer-string)))
               (goto-char (point-min))
               (forward-line 1)
               (list
                (file-exists-p alarm-clock-cache-file)
                text
                (read (current-buffer))
                (get-file-buffer alarm-clock-cache-file))))))"##;
    let expect = expect![[
        r#"OK (t ";; Auto-generated file; don't edit\n((:time \"2030-01-02T03:04:05+0000\" :message \"Wake\")\n (:time \"2031-05-06T07:08:09+0000\" :message \"Ship\"))\n" ((:time "2030-01-02T03:04:05+0000" :message "Wake") (:time "2031-05-06T07:08:09+0000" :message "Ship")) nil)"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_restore_reads_cache_recreates_each_alarm_and_opens_list() {
    let elisp_form = r##"(let* ((root (expand-file-name "restore"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (alarm-clock-cache-file
                 (expand-file-name "alarms.el" root))
                calls)
         (make-directory root t)
         (with-temp-file alarm-clock-cache-file
           (insert
            ";; cache\n"
            "((:time \"2030-01-02T03:04:05+0000\" :message \"Wake\")\n"
            " (:time \"2031-05-06T07:08:09+0000\" :message \"Ship\"))\n"))
         (cl-letf
             (((symbol-function 'alarm-clock--kill-all)
               (lambda () (push '(kill-all) calls)))
              ((symbol-function 'alarm-clock--set)
               (lambda (time message)
                 (push
                  (list 'set
                        (format-time-string "%FT%T%z" time t)
                        message)
                  calls)))
              ((symbol-function 'alarm-clock-list-view)
               (lambda () (push '(view) calls) 'viewed)))
           (list
            (alarm-clock-restore)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (viewed ((kill-all) (set "2030-01-02T03:04:05+0000" "Wake") (set "2031-05-06T07:08:09+0000" "Ship") (view)))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_restore_empty_or_missing_cache_still_clears_and_opens_list() {
    let elisp_form = r##"(let* ((root (expand-file-name "restore-empty"
                                           (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (alarm-clock-cache-file
                 (expand-file-name "empty.el" root))
                calls)
         (make-directory root t)
         (with-temp-file alarm-clock-cache-file)
         (cl-letf
             (((symbol-function 'alarm-clock--kill-all)
               (lambda () (push '(kill-all) calls)))
              ((symbol-function 'alarm-clock--set)
               (lambda (&rest args) (push (cons 'set args) calls)))
              ((symbol-function 'alarm-clock-list-view)
               (lambda () (push '(view) calls) 'viewed)))
           (list
            (alarm-clock-restore)
            (nreverse calls))))"##;
    let expect = expect!["OK (viewed ((kill-all) (view)))"];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_kill_all_cancels_every_timer_and_clears_registry() {
    let elisp_form = r##"(let ((alarm-clock--alist
                '((:message "one" :timer timer-a)
                  (:message "two" :timer timer-b)
                  (:message "three" :timer timer-c)))
               calls)
         (cl-letf (((symbol-function 'cancel-timer)
                    (lambda (timer)
                      (push timer calls)
                      'cancelled)))
           (list
            (alarm-clock--kill-all)
            alarm-clock--alist
            (nreverse calls))))"##;
    let expect = expect!["OK (nil nil (timer-a timer-b timer-c))"];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_autosave_aliases_add_and_remove_the_exact_kill_hook() {
    let elisp_form = r##"(let ((kill-emacs-hook '(existing-hook)))
         (list
          (alarm-clock-turn-autosave-on)
          kill-emacs-hook
          (alarm-clock-turn-autosave-on)
          kill-emacs-hook
          (alarm-clock-turn-autosave-off)
          kill-emacs-hook
          (eq (indirect-function 'alarm-clock-turn-autosave-on)
              (symbol-function 'alarm-clock--turn-autosave-on))
          (eq (indirect-function 'alarm-clock-turn-autosave-off)
              (symbol-function 'alarm-clock--turn-autosave-off))))"##;
    let expect = expect!["OK (#1=(alarm-clock-save . #2=(existing-hook)) #1# #1# #1# #2# #2# t t)"];
    assert_alarm_clock_parity(elisp_form, expect);
}
