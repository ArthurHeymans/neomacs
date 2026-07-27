use expect_test::expect;

use super::assert_android_mode_parity;

#[test]
fn exclusive_command_quotes_arguments_blocks_duplicates_and_sentinel_releases_terminal_process() {
    let elisp_form = r##"(let ((android-exclusive-processes nil)
                          starts
                          installed-sentinel
                          status)
                      (cl-letf
                          (((symbol-function
                             'start-process-shell-command)
                            (lambda
                                (name buffer command)
                              (push
                               (list name buffer command)
                               starts)
                              'fake-process))
                           ((symbol-function
                             'set-process-sentinel)
                            (lambda (process sentinel)
                              (setq installed-sentinel
                                    sentinel)
                              (list process sentinel)))
                           ((symbol-function
                             'process-status)
                            (lambda (_process)
                              status))
                           ((symbol-function
                             'process-name)
                            (lambda (_process)
                              "*android-emulator-Pixel 7*")))
                        (let ((first
                               (android-start-exclusive-command
                                "*android-emulator-Pixel 7*"
                                "/sdk/emulator"
                                "-avd"
                                "Pixel 7"
                                "owner's device"))
                              (duplicate
                               (android-start-exclusive-command
                                "*android-emulator-Pixel 7*"
                                "/sdk/emulator"
                                "-avd"
                                "Pixel 7")))
                          (setq status 'run)
                          (funcall
                           installed-sentinel
                           'fake-process
                           "running")
                          (let ((while-running
                                 android-exclusive-processes))
                            (setq status 'exit)
                            (funcall
                             installed-sentinel
                             'fake-process
                             "finished")
                            (list
                             first
                             duplicate
                             while-running
                             android-exclusive-processes
                             (nreverse starts))))))"##;
    let expect = expect![[
        r#"OK (#1=(*android-emulator-Pixel\ 7*) nil #1# nil (("*android-emulator-Pixel 7*" "*android-emulator-Pixel 7*" "/sdk/emulator -avd Pixel\\ 7 owner\\'s\\ device")))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn emulator_uses_configured_or_prompted_avd_and_reports_only_duplicate_launches() {
    let elisp_form = r##"(let (events messages
                          (responses
                           '(started nil)))
                      (cl-letf
                          (((symbol-function
                             'android-list-avd)
                            (lambda ()
                              (push 'list-avd events)
                              '("Prompted Tablet")))
                           ((symbol-function
                             'completing-read)
                            (lambda
                                (prompt choices)
                              (push
                               (list
                                'prompt
                                prompt
                                choices)
                               events)
                              "Prompted Tablet"))
                           ((symbol-function
                             'android-tool-path)
                            (lambda (name)
                              (push
                               (list 'tool name)
                               events)
                              "/sdk/emulator"))
                           ((symbol-function
                             'android-start-exclusive-command)
                            (lambda (&rest arguments)
                              (push
                               (cons 'start arguments)
                               events)
                              (pop responses)))
                           ((symbol-function 'message)
                            (lambda
                                (format-string
                                 &rest arguments)
                              (push
                               (apply
                                #'format
                                format-string
                                arguments)
                               messages)
                              nil)))
                        (let ((android-mode-avd
                               "Configured Phone"))
                          (android-start-emulator))
                        (let ((android-mode-avd ""))
                          (android-start-emulator))
                        (list
                         (nreverse events)
                         (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (((tool "emulator") (start "*android-emulator-Configured Phone*" "/sdk/emulator" "-avd" "Configured Phone") list-avd (prompt "Android Virtual Device: " ("Prompted Tablet")) (tool "emulator") (start "*android-emulator-Prompted Tablet*" "/sdk/emulator" "-avd" "Prompted Tablet")) ("emulator Prompted Tablet already running"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn ddms_launch_uses_sdk_tool_and_emits_duplicate_feedback_only_when_already_running() {
    let elisp_form = r##"(let ((responses
                           '(started nil))
                          events
                          messages)
                      (cl-letf
                          (((symbol-function
                             'android-tool-path)
                            (lambda (name)
                              (push
                               (list 'tool name)
                               events)
                              "/sdk/tools/ddms"))
                           ((symbol-function
                             'android-start-exclusive-command)
                            (lambda (&rest arguments)
                              (push arguments events)
                              (pop responses)))
                           ((symbol-function 'message)
                            (lambda
                                (format-string
                                 &rest arguments)
                              (push
                               (apply
                                #'format
                                format-string
                                arguments)
                               messages)
                              nil)))
                        (list
                         (android-start-ddms)
                         (android-start-ddms)
                         (nreverse events)
                         (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (nil nil ((tool "ddms") ("*android-ddms*" "/sdk/tools/ddms") (tool "ddms") ("*android-ddms*" "/sdk/tools/ddms")) ("ddms already running"))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}

#[test]
fn logcat_command_initializes_new_buffer_process_filter_navigation_and_mode_state_once() {
    let elisp_form = r##"(let ((android-logcat-buffer
                           "*android-logcat-test*")
                          starts
                          filters
                          start-result)
                      (when
                          (get-buffer
                           android-logcat-buffer)
                        (kill-buffer
                         android-logcat-buffer))
                      (unwind-protect
                          (cl-letf
                              (((symbol-function
                                 'android-tool-path)
                                (lambda (name)
                                  (list 'tool name)))
                               ((symbol-function
                                 'android-start-exclusive-command)
                                (lambda (&rest arguments)
                                  (push arguments starts)
                                  (when start-result
                                    (get-buffer-create
                                     android-logcat-buffer))
                                  start-result))
                               ((symbol-function
                                 'get-buffer-process)
                                (lambda (_buffer)
                                  'fake-logcat-process))
                               ((symbol-function
                                 'set-process-filter)
                                (lambda (process filter)
                                  (push
                                   (list process filter)
                                   filters)
                                  filter)))
                            (setq start-result t)
                            (android-logcat)
                            (let ((first
                                   (with-current-buffer
                                       android-logcat-buffer
                                     (list
                                      buffer-read-only
                                      tab-stop-list
                                      android-mode-log-filter-regexp
                                      (eq
                                       (current-local-map)
                                       android-logcat-map)
                                      android-mode
                                      (point)))))
                              (setq start-result nil)
                              (android-logcat)
                              (list
                               first
                               (buffer-name)
                               (nreverse starts)
                               (nreverse filters))))
                        (when
                            (get-buffer
                             android-logcat-buffer)
                          (kill-buffer
                           android-logcat-buffer))))"##;
    let expect = expect![[
        r#"OK ((t (2 30) "" t t 1) "*android-logcat-test*" (("*android-logcat-test*" (tool "adb") "logcat") ("*android-logcat-test*" (tool "adb") "logcat")) ((fake-logcat-process android-logcat-process-filter)))"#
    ]];
    assert_android_mode_parity(elisp_form, expect);
}
