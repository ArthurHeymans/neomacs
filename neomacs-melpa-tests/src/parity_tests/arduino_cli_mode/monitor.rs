use expect_test::expect;

use super::assert_arduino_cli_mode_parity;

#[test]
fn serial_monitor_activity_reflects_buffer_process_liveness() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'get-buffer-process)
                    (lambda (buffer)
                      (push (list :buffer buffer) calls)
                      (and buffer 'fake-process)))
                   ((symbol-function 'process-live-p)
                    (lambda (process)
                      (push (list :process process) calls)
                      (eq process 'fake-process))))
           (let ((arduino-cli--monitor-buffer nil))
             (list
              (arduino-cli--serial-monitor-is-active)
              (let ((arduino-cli--monitor-buffer 'monitor-buffer))
                (arduino-cli--serial-monitor-is-active))
              (nreverse calls)))))"##;
    let expect = expect![
        "OK (nil t ((:buffer nil) (:process nil) (:buffer monitor-buffer) (:process fake-process)))"
    ];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn monitor_callback_only_restarts_after_successful_matching_compilation() {
    let elisp_form = r##"(let ((arduino-cli--compilation-buffer 'owned-buffer)
               (compilation-finish-functions
                '(arduino-cli--start-serial-monitor-callback other-hook))
               calls)
         (cl-letf (((symbol-function 'arduino-cli-start-serial-monitor)
                    (lambda (&optional rate)
                      (push (list :start rate) calls))))
           (arduino-cli--start-serial-monitor-callback
            'other-buffer "finished\n")
           (arduino-cli--start-serial-monitor-callback
            'owned-buffer "exited abnormally\n")
           (arduino-cli--start-serial-monitor-callback
            'owned-buffer "finished\n")
           (list (nreverse calls) compilation-finish-functions)))"##;
    let expect = expect!["OK (((:start nil)) (other-hook))"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn start_monitor_builds_exact_default_baud_command_and_remembers_window_buffer() {
    let elisp_form = r##"(let ((arduino-cli--monitor-buffer nil)
               (arduino-cli-verbosity 'quiet)
               (arduino-cli-monitor-default-baud-rate 115200)
               calls)
         (cl-letf (((symbol-function 'arduino-cli--serial-monitor-is-active)
                    (lambda () nil))
                   ((symbol-function 'arduino-cli--board)
                    (lambda ()
                      '((port (address . "/dev/tty USB0")))))
                   ((symbol-function 'async-shell-command)
                    (lambda (cmd buffer &rest _)
                      (push (list cmd
                                  (buffer-name
                                   (get-buffer-create buffer))
                                  async-shell-command-buffer
                                  shell-command-dont-erase-buffer)
                            calls)
                      'fake-window))
                   ((symbol-function 'window-buffer)
                    (lambda (_window) 'monitor-buffer)))
           (arduino-cli-start-serial-monitor)
           (list arduino-cli--monitor-buffer (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (monitor-buffer (("arduino-cli monitor --port /dev/tty\\ USB0 --config baudrate=115200 " "arduino cli monitor" confirm-kill-process t)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn start_monitor_honors_prefix_baud_and_general_verbosity_flags() {
    let elisp_form = r##"(let ((arduino-cli--monitor-buffer nil)
               (arduino-cli-verbosity 'verbose)
               (arduino-cli-compile-only-verbosity nil)
               command)
         (cl-letf (((symbol-function 'arduino-cli--serial-monitor-is-active)
                    (lambda () nil))
                   ((symbol-function 'arduino-cli--board)
                    (lambda ()
                      '((port (address . "COM3")))))
                   ((symbol-function 'format-time-string)
                    (lambda (&rest _) "12:34:56"))
                   ((symbol-function 'async-shell-command)
                    (lambda (cmd _buffer &rest _)
                      (setq command cmd)
                      'fake-window))
                   ((symbol-function 'window-buffer)
                    (lambda (_window)
                      (get-buffer-create "monitor-result"))))
           (arduino-cli-start-serial-monitor '(4))
           (list command
                 (with-current-buffer arduino-cli--monitor-buffer
                   (buffer-string)))))"##;
    let expect =
        expect![[r#"OK ("arduino-cli monitor --port COM3 --config baudrate=4  --verbose" "")"#]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn restart_active_monitor_stops_then_waits_until_process_exits() {
    let elisp_form = r##"(let ((states '(t t nil))
               calls)
         (cl-letf (((symbol-function 'arduino-cli--serial-monitor-is-active)
                    (lambda ()
                      (prog1 (car states)
                        (setq states (cdr states)))))
                   ((symbol-function 'arduino-cli-stop-serial-monitor)
                    (lambda (&optional reason)
                      (push (list :stop reason) calls)))
                   ((symbol-function 'sit-for)
                    (lambda (seconds)
                      (push (list :wait seconds) calls)))
                   ((symbol-function 'arduino-cli--board)
                    (lambda () '((port (address . "/dev/ttyACM0")))))
                   ((symbol-function 'async-shell-command)
                    (lambda (cmd buffer &rest _)
                      (push (list :start cmd buffer) calls)
                      'fake-window))
                   ((symbol-function 'window-buffer)
                    (lambda (_window) 'monitor-buffer)))
           (arduino-cli-start-serial-monitor 9600)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ((:stop "to restart the serial monitor") (:wait 0.01) (:start "arduino-cli monitor --port /dev/ttyACM0 --config baudrate=9600 " (:buffer "arduino cli monitor")))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn stop_monitor_kills_live_process_and_writes_deterministic_reason() {
    let elisp_form = r##"(let ((arduino-cli--monitor-buffer
                (get-buffer-create "arduino-stop-contract"))
               (arduino-cli-verbosity nil)
               calls)
         (cl-letf (((symbol-function 'get-buffer-process)
                    (lambda (_buffer) 'fake-process))
                   ((symbol-function 'process-live-p)
                    (lambda (_process) t))
                   ((symbol-function 'kill-process)
                    (lambda (process) (push process calls)))
                   ((symbol-function 'format-time-string)
                    (lambda (&rest _) "12:34:56")))
           (with-current-buffer arduino-cli--monitor-buffer
             (erase-buffer))
           (arduino-cli-stop-serial-monitor "to upload a sketch")
           (list (nreverse calls)
                 (with-current-buffer arduino-cli--monitor-buffer
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((fake-process) "\nStopped serial monitor to upload a sketch at 12:34:56...\n\n")"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn mode_toggle_sets_lighter_keymap_menu_and_local_state() {
    let elisp_form = r##"(with-temp-buffer
         (arduino-cli-mode 1)
         (let ((enabled
                (list arduino-cli-mode
                      (assq 'arduino-cli-mode minor-mode-alist)
                      (lookup-key
                       (current-local-map)
                       arduino-cli-mode-keymap-prefix)
                      (lookup-key arduino-cli-command-map (kbd "c"))
                      (lookup-key arduino-cli-command-map (kbd "M")))))
           (arduino-cli-mode -1)
           (list enabled arduino-cli-mode)))"##;
    let expect = expect![[
        r#"OK ((t (arduino-cli-mode " arduino-cli") 1 arduino-cli-compile arduino-cli-stop-serial-monitor) nil)"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}
