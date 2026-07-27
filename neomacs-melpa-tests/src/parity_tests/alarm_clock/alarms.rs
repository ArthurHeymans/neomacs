use expect_test::expect;

use super::assert_alarm_clock_parity;

#[test]
fn alarm_clock_preparse_time_expands_supported_abbreviations_and_preserves_other_inputs() {
    let elisp_form = r##"(mapcar
         #'alarm-clock--preparse-time
         '(" 2s "
           "3m"
           "4h"
           "5h6m"
           "7h8m9s"
           "10m20s"
           "11:40pm"
           "0s"
           "  two minutes  "
           30
           nil))"##;
    let expect = expect![[
        r#"OK ("2second" "3minute" "4hour" "5hour6minute" "7hour8minute9second" "10minute20second" "11:40pm" "0s" "two minutes" 30 nil)"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_set_builds_a_real_timer_record_with_trimmed_message_and_callback_contract() {
    let elisp_form = r##"(let (scheduled
                (alarm-clock--alist nil))
         (cl-letf
             (((symbol-function 'alarm-clock--preparse-time)
               (lambda (time) (list 'parsed time)))
              ((symbol-function 'run-at-time)
               (lambda (time repeat function &rest args)
                 (setq scheduled
                       (list time repeat
                             (functionp function)
                             args))
                 (let ((timer (timer-create)))
                   (timer-set-time
                    timer
                    (encode-time 0 30 8 1 1 2030))
                   timer))))
           (alarm-clock--set " 2m " "  Stand up  ")
           (let ((alarm (car alarm-clock--alist)))
             (list scheduled
                   (format-time-string
                    "%FT%T%z" (plist-get alarm :time) t)
                   (plist-get alarm :message)
                   (timerp (plist-get alarm :timer))
                   (length alarm-clock--alist)))))"##;
    let expect = expect![[
        r#"OK (((parsed " 2m ") nil t ("Stand up")) "2030-01-01T08:30:00+0000" "Stand up" t 1)"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_public_set_runs_set_list_refresh_and_conditional_auto_save_in_order() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'alarm-clock--set)
                    (lambda (time message)
                      (push (list 'set time message) calls)))
                   ((symbol-function 'alarm-clock--list-prepare)
                    (lambda () (push '(list) calls)))
                   ((symbol-function 'alarm-clock--maybe-auto-save)
                    (lambda () (push '(save) calls) 'saved)))
           (list
            (alarm-clock-set "10m" "Tea")
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (saved ((set "10m" "Tea") (list) (save)))"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_compare_and_sort_order_real_absolute_times_earliest_first() {
    let elisp_form = r##"(let* ((early (list :time (encode-time 0 0 8 1 1 2030)
                                  :message "early"))
                (middle (list :time (encode-time 0 30 8 1 1 2030)
                              :message "middle"))
                (late (list :time (encode-time 0 0 9 1 1 2030)
                            :message "late"))
                (alarm-clock--alist (list middle late early)))
         (list
          (alarm-clock--compare early late)
          (alarm-clock--compare late early)
          (mapcar
           (lambda (alarm) (plist-get alarm :message))
           (alarm-clock--sort-list))
          (mapcar
           (lambda (alarm) (plist-get alarm :message))
           alarm-clock--alist)))"##;
    let expect = expect![[r#"OK (nil t ("late" "middle" "early") ("late" "middle" "early"))"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_unexpired_filter_and_removal_use_current_time_boundary_exactly() {
    let elisp_form = r##"(let* ((now (encode-time 0 0 12 1 1 2030))
                (past (list :time (encode-time 59 59 11 1 1 2030)
                            :message "past"))
                (equal (list :time now :message "equal"))
                (future (list :time (encode-time 1 0 12 1 1 2030)
                              :message "future"))
                (later (list :time (encode-time 0 1 12 1 1 2030)
                             :message "later"))
                (alarm-clock--alist
                 (list past equal future later)))
         (cl-letf (((symbol-function 'current-time)
                    (lambda () now)))
           (let ((selected
                  (mapcar
                   (lambda (alarm) (plist-get alarm :message))
                   (alarm-clock--unexpired-alarms))))
             (alarm-clock--remove-expired)
             (list selected
                   (mapcar
                    (lambda (alarm) (plist-get alarm :message))
                    alarm-clock--alist)))))"##;
    let expect = expect![[r#"OK (("future" "later") ("future" "later"))"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_maybe_auto_save_respects_setting_and_returns_save_result() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'alarm-clock-save)
                    (lambda ()
                      (push 'saved calls)
                      42)))
           (let ((alarm-clock-auto-save nil))
             (list
              (alarm-clock--maybe-auto-save)
              (let ((alarm-clock-auto-save t))
                (alarm-clock--maybe-auto-save))
              (nreverse calls)))))"##;
    let expect = expect!["OK (nil 42 (saved))"];
    assert_alarm_clock_parity(elisp_form, expect);
}
