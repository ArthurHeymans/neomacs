use expect_test::expect;

use super::assert_alarm_clock_parity;

#[test]
fn alarm_clock_list_prepare_renders_sorted_rows_header_mode_and_alarm_properties() {
    let elisp_form = r##"(let* ((first (list :time '(100 0 0 0)
                                  :message "Breakfast"
                                  :timer 'timer-a))
                (second (list :time '(200 0 0 0)
                              :message "Meeting"
                              :timer 'timer-b))
                (alarm-clock--alist (list second first)))
         (cl-letf
             (((symbol-function 'alarm-clock--remove-expired) #'ignore)
              ((symbol-function 'format-time-string)
               (lambda (format time &optional _universal)
                 (cond
                  ((equal format "%F %X")
                   (if (equal time '(100 0 0 0))
                       "2030-01-01 08:00:00"
                     "2030-01-01 09:00:00"))
                  ((equal format "%H:%2M:%2S")
                   (if (< (float-time time) 150)
                       "00:10:00"
                     "01:10:00")))))
              ((symbol-function 'time-subtract)
               (lambda (time _) time)))
           (unwind-protect
               (progn
                 (alarm-clock--list-prepare)
                 (with-current-buffer "*alarm clock*"
                   (list
                    major-mode
                    header-line-format
                    buffer-read-only
                    (buffer-string)
                    (mapcar
                     (lambda (position)
                       (let ((alarm
                              (get-text-property
                               position 'alarm-clock)))
                         (and alarm
                              (plist-get alarm :message))))
                     (list (point-min)
                           (save-excursion
                             (goto-char (point-min))
                             (forward-line 1)
                             (point)))))))
             (when (get-buffer "*alarm clock*")
               (kill-buffer "*alarm clock*")))))"##;
    let expect = expect![[
        r#"OK (alarm-clock-mode "Time                 Remaining      Message" t #("2030-01-01 08:00:00  01:10:00       Breakfast\n2030-01-01 09:00:00  01:10:00       Meeting\n" 0 1 (alarm-clock (:time (100 0 0 0) :message "Breakfast" :timer timer-a)) 46 47 (alarm-clock (:time (200 0 0 0) :message "Meeting" :timer timer-b))) ("Breakfast" "Meeting"))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_kill_deletes_selected_row_cancels_timer_and_auto_saves() {
    let elisp_form = r##"(let* ((selected (list :time '(100 0 0 0)
                                     :message "Selected"
                                     :timer 'timer-a))
                (other (list :time '(200 0 0 0)
                             :message "Other"
                             :timer 'timer-b))
                (alarm-clock--alist (list selected other))
                calls)
         (with-temp-buffer
           (alarm-clock-mode)
           (let ((inhibit-read-only t))
             (insert "Selected row\nOther row\n")
             (put-text-property
              (point-min) (1+ (point-min))
              'alarm-clock selected))
           (goto-char (point-min))
           (cl-letf (((symbol-function 'cancel-timer)
                      (lambda (timer)
                        (push (list 'cancel timer) calls)))
                     ((symbol-function 'alarm-clock--maybe-auto-save)
                      (lambda ()
                        (push '(save) calls)
                        'saved)))
             (list
              (alarm-clock-kill)
              (buffer-string)
              (mapcar
               (lambda (alarm) (plist-get alarm :message))
               alarm-clock--alist)
              (nreverse calls)))))"##;
    let expect = expect![[r#"OK (saved "Other row\n" ("Other") ((cancel timer-a) (save)))"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_kill_on_header_or_empty_line_signals_user_error_without_mutation() {
    let elisp_form = r##"(let ((alarm-clock--alist nil))
         (with-temp-buffer
           (alarm-clock-mode)
           (let ((inhibit-read-only t))
             (insert "No alarm here\n"))
           (condition-case error
               (alarm-clock-kill)
             (error
              (list (car error)
                    (cdr error)
                    (buffer-string)
                    alarm-clock--alist)))))"##;
    let expect = expect![[
        r#"OK (user-error ("No alarm clock on the current line") "No alarm here\n" nil)"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_list_view_refreshes_then_selects_the_named_buffer() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'alarm-clock--list-prepare)
                    (lambda ()
                      (push '(prepare) calls)
                      (get-buffer-create "*alarm clock*")))
                   ((symbol-function 'pop-to-buffer)
                    (lambda (buffer)
                      (push (list 'pop buffer) calls)
                      'displayed)))
           (unwind-protect
               (list (alarm-clock-list-view)
                     (nreverse calls))
             (when (get-buffer "*alarm clock*")
               (kill-buffer "*alarm clock*")))))"##;
    let expect = expect![[r#"OK (displayed ((prepare) (pop "*alarm clock*")))"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}
