use expect_test::expect;

use super::assert_alarm_clock_parity;

#[test]
fn alarm_clock_stop_sets_state_and_emits_user_message() {
    let elisp_form = r##"(let ((alarm-clock--stopped nil)
                messages)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args)
                            messages))))
           (list
            (alarm-clock-stop)
            alarm-clock--stopped
            (nreverse messages))))"##;
    let expect = expect![[r#"OK (#1=("Alarm stopped.") t #1#)"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_ding_timer_starts_sound_and_schedules_decrement_until_stopped() {
    let elisp_form = r##"(let ((alarm-clock--stopped nil)
                calls)
         (cl-letf (((symbol-function 'start-process)
                    (lambda (name buffer program sound)
                      (push (list 'start name buffer program sound) calls)
                      'process))
                   ((symbol-function 'run-at-time)
                    (lambda (time repeat function &rest args)
                      (push (list 'timer time repeat
                                  (functionp function) args)
                            calls)
                      'timer)))
           (let ((running
                  (alarm-clock--ding-on-timer
                   "mpg123" "/sound/alarm.mp3" 3)))
             (setq alarm-clock--stopped t)
             (list running
                   (alarm-clock--ding-on-timer
                    "mpg123" "/sound/alarm.mp3" 2)
                   (let ((alarm-clock--stopped nil))
                     (alarm-clock--ding-on-timer
                      "mpg123" "/sound/alarm.mp3" 0))
                   (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (timer nil nil ((start "Alarm Clock" nil "mpg123" "/sound/alarm.mp3") (timer 2 nil t (2))))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_ding_selects_platform_player_validates_assets_and_arms_repeat() {
    let elisp_form = r##"(let ((alarm-clock-sound-file "sounds/alarm.mp3")
                (alarm-clock-play-sound-repeat 4)
                (alarm-clock--stopped 'old)
                calls)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (program)
                 (push (list 'executable program) calls)
                 (unless (string-empty-p program)
                   (concat "/bin/" program))))
              ((symbol-function 'file-exists-p)
               (lambda (path)
                 (push (list 'exists
                             (file-name-nondirectory path))
                       calls)
                 t))
              ((symbol-function 'run-at-time)
               (lambda (time repeat function &rest args)
                 (push (list 'timer time repeat
                             (functionp function) args)
                       calls)
                 'armed)))
           (let ((system-type 'gnu/linux))
             (list
              (alarm-clock--ding)
              alarm-clock--stopped
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (armed nil ((executable "mpg123") (exists "alarm.mp3") (timer "0" nil t (4))))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_system_notify_builds_linux_and_macos_process_arguments() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (program)
                 (push (list 'find program) calls)
                 (concat "/bin/" program)))
              ((symbol-function 'start-process)
               (lambda (&rest args)
                 (push (cons 'start args) calls)
                 'started))
              ((symbol-function 'alarm-clock--get-macos-sender)
               (lambda () '("-sender" "org.gnu.Emacs"))))
           (list
            (let ((system-type 'gnu/linux))
              (alarm-clock--system-notify "Alarm" "Stand up"))
            (let ((system-type 'darwin))
              (alarm-clock--system-notify "Alarm" "Stand up"))
            (let ((system-type 'windows-nt))
              (alarm-clock--system-notify "Alarm" "Stand up"))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (started started started ((find "notify-send") (start "Alarm" nil "notify-send" "-u" "critical" "Alarm" "Stand up") (find "terminal-notifier") (start "Alarm" nil "terminal-notifier" "-title" "Alarm" "-sender" "org.gnu.Emacs" "-message" "Stand up" "-ignoreDnD") (find "") (start "Alarm" nil "")))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_notify_runs_view_sound_alert_system_and_minibuffer_workflow() {
    let elisp_form = r##"(let ((alarm-clock-play-auto-view-alarms t)
                (alarm-clock-play-sound t)
                (alarm-clock-alert-notify t)
                (alarm-clock-system-notify t)
                calls)
         (cl-letf
             (((symbol-function 'alarm-clock-list-view)
               (lambda () (push '(view) calls)))
              ((symbol-function 'alarm-clock--ding)
               (lambda () (push '(ding) calls)))
              ((symbol-function 'alert)
               (lambda (message &rest args)
                 (push (list 'alert message args) calls)))
              ((symbol-function 'alarm-clock--system-notify)
               (lambda (title message)
                 (push (list 'system title message) calls)))
              ((symbol-function 'message)
               (lambda (format-string &rest args)
                 (let ((value (apply #'format format-string args)))
                   (push (list 'message value) calls)
                   value))))
           (list
            (alarm-clock--notify "Alarm Clock" "Drink water")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("[Alarm Clock] - Drink water" ((view) (ding) (alert "Drink water" (:title "Alarm Clock")) (system "Alarm Clock" "Drink water") (message "[Alarm Clock] - Drink water")))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_notify_respects_every_disabled_delivery_channel() {
    let elisp_form = r##"(let ((alarm-clock-play-auto-view-alarms nil)
                (alarm-clock-play-sound nil)
                (alarm-clock-alert-notify nil)
                (alarm-clock-system-notify nil)
                calls)
         (cl-letf
             (((symbol-function 'alarm-clock-list-view)
               (lambda () (push '(view) calls)))
              ((symbol-function 'alarm-clock--ding)
               (lambda () (push '(ding) calls)))
              ((symbol-function 'alert)
               (lambda (&rest args) (push (cons 'alert args) calls)))
              ((symbol-function 'alarm-clock--system-notify)
               (lambda (&rest args) (push (cons 'system args) calls)))
              ((symbol-function 'message)
               (lambda (format-string &rest args)
                 (let ((value (apply #'format format-string args)))
                   (push (list 'message value) calls)
                   value))))
           (list
            (alarm-clock--notify "Quiet" "Still visible")
            (nreverse calls))))"##;
    let expect =
        expect![[r#"OK ("[Quiet] - Still visible" ((message "[Quiet] - Still visible")))"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_macos_sender_handles_old_new_and_cached_version_results() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (push command calls)
                 "10.14.6\n")))
           (let ((alarm-clock--macos-sender nil))
             (list
              (alarm-clock--get-macos-sender)
              (alarm-clock--get-macos-sender)
              (nreverse calls)
              (let ((alarm-clock--macos-sender nil))
                (cl-letf
                    (((symbol-function
                       'shell-command-to-string)
                      (lambda (_) "13.5.1\n")))
                  (alarm-clock--get-macos-sender)))))))"##;
    let expect =
        expect![[r#"OK (#1=("-sender" "org.gnu.Emacs") #1# ("sw_vers -productVersion") #1#)"#]];
    assert_alarm_clock_parity(elisp_form, expect);
}
