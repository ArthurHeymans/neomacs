use expect_test::expect;

use super::assert_arch_packer_parity;

#[test]
fn backend_command_and_database_refresh_apply_sudo_and_history_suppression_exactly() {
    let elisp_form = r##"(let (calls)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-call-shell-process)
                          (lambda (process command)
                            (push (list process command) calls))))
                      (let ((arch-packer-default-command "pacman")
                            (arch-packer-no-shell-history
                             "; forget-history"))
                        (let ((pacman-command
                               (arch-packer-shell-command)))
                          (arch-packer-refresh-database)
                          (let ((arch-packer-default-command
                                 "pacaur"))
                            (let ((pacaur-command
                                   (arch-packer-shell-command)))
                              (arch-packer-refresh-database)
                              (list
                               pacman-command pacaur-command
                               (nreverse calls))))))))"##;
    let expect = expect![[
        r#"OK ("sudo pacman" "pacaur" (("arch-packer-process" "sudo pacman -Sy; forget-history") ("arch-packer-process" "pacaur -Sy; forget-history")))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn upgrade_and_delete_workflows_send_exact_commands_then_wait_and_request_status() {
    let elisp_form = r##"(let ((arch-packer-default-command "pacman")
                        (arch-packer-no-shell-history "; no-history")
                        events)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-call-shell-process)
                          (lambda (process command)
                            (push (list :send process command) events)))
                         ((symbol-function
                           'arch-packer-wait-shell-subprocess)
                          (lambda () (push :wait events)))
                         ((symbol-function
                           'arch-packer-get-exit-status)
                          (lambda () (push :status events))))
                      (arch-packer-upgrade-package
                       "linux linux-headers")
                      (arch-packer-delete-package
                       "old-one old-two")
                      (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((:send "arch-packer-process" "sudo pacman -S --noconfirm linux linux-headers; no-history") :wait :status (:send "arch-packer-process" "sudo pacman -Rsn --noconfirm old-one old-two; no-history") :wait :status)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn pacaur_upgrade_and_delete_commands_do_not_gain_sudo() {
    let elisp_form = r##"(let ((arch-packer-default-command "pacaur")
                        (arch-packer-no-shell-history "; hidden")
                        commands)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-call-shell-process)
                          (lambda (_process command)
                            (push command commands)))
                         ((symbol-function
                           'arch-packer-wait-shell-subprocess)
                          (lambda () nil))
                         ((symbol-function
                           'arch-packer-get-exit-status)
                          (lambda () nil)))
                      (arch-packer-upgrade-package "yay")
                      (arch-packer-delete-package "obsolete")
                      (nreverse commands)))"##;
    let expect = expect![[
        r#"OK ("pacaur -S --noconfirm yay; hidden" "pacaur -Rsn --noconfirm obsolete; hidden")"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn shell_send_appends_one_newline_and_enables_status_reporting_after_dispatch() {
    let elisp_form = r##"(let (events)
                    (cl-letf
                        (((symbol-function 'process-send-string)
                          (lambda (process string)
                            (push (list :send process string) events)))
                         ((symbol-function
                           'arch-packer-enable-status-reporter)
                          (lambda () (push :enable events))))
                      (arch-packer-call-shell-process
                       'fake-process
                       "sudo pacman -S linux")
                      (nreverse events)))"##;
    let expect = expect![[r#"OK ((:send fake-process "sudo pacman -S linux\n") :enable)"#]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn exit_status_command_waits_then_sends_shell_conditional_with_history_cleanup() {
    let elisp_form = r##"(let ((arch-packer-no-shell-history
                         "; erase-history")
                        events)
                    (cl-letf
                        (((symbol-function
                           'arch-packer-wait-shell-subprocess)
                          (lambda () (push :wait events)))
                         ((symbol-function
                           'arch-packer-call-shell-process)
                          (lambda (process command)
                            (push (list :send process command) events))))
                      (arch-packer-get-exit-status)
                      (nreverse events)))"##;
    let expect = expect![[
        r#"OK (:wait (:send "arch-packer-process" "if [ `echo $?` -ne 0 ];\n                                           then echo \"Pacman error\n\";\n                                           else echo \"Pacman finished\n\" ;fi; erase-history"))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn wait_loop_polls_until_child_process_finishes_without_real_time_delay() {
    let elisp_form = r##"(let ((states '(t t nil))
                        waits)
                    (cl-letf
                        (((symbol-function
                           'process-running-child-p)
                          (lambda (_process)
                            (prog1 (car states)
                              (setq states (cdr states)))))
                         ((symbol-function 'sit-for)
                          (lambda (seconds)
                            (push seconds waits)
                            t)))
                      (arch-packer-wait-shell-subprocess)
                      (list (nreverse waits) states)))"##;
    let expect = expect!["OK ((0.1 0.1) nil)"];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn status_reporter_hooks_enable_disable_idempotently_under_configuration_policy() {
    let elisp_form = r##"(let ((post-command-hook nil))
                    (let ((arch-packer-display-status-reporter t))
                      (arch-packer-enable-status-reporter)
                      (arch-packer-enable-status-reporter)
                      (let ((enabled (copy-sequence post-command-hook)))
                        (arch-packer-disable-status-reporter)
                        (let ((disabled
                               (copy-sequence post-command-hook)))
                          (let ((arch-packer-display-status-reporter
                                 nil))
                            (arch-packer-enable-status-reporter)
                            (list
                             enabled disabled
                             post-command-hook))))))"##;
    let expect = expect!["OK ((arch-packer-status-reporter) nil nil)"];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn output_buffer_factory_is_idempotent_and_configures_output_mode_once() {
    let elisp_form = r##"(let ((arch-packer-process-output-buffer
                         "*arch-packer-output-contract*"))
                    (when
                        (get-buffer
                         arch-packer-process-output-buffer)
                      (kill-buffer
                       arch-packer-process-output-buffer))
                    (unwind-protect
                        (let ((first
                               (arch-packer-get-output-buffer-create))
                              (second
                               (arch-packer-get-output-buffer-create)))
                          (with-current-buffer first
                            (list
                             (eq first second)
                             (buffer-name)
                             major-mode mode-name
                             buffer-read-only
                             truncate-lines
                             (eq
                              (current-local-map)
                              arch-packer-output-mode-map))))
                      (when
                          (get-buffer
                           arch-packer-process-output-buffer)
                        (kill-buffer
                         arch-packer-process-output-buffer))))"##;
    let expect = expect![[
        r#"OK (t "*arch-packer-output-contract*" arch-packer-output-mode "Process output" nil t t)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn ordinary_process_filter_output_appends_chunks_and_ignores_bracket_prefixed_noise() {
    let elisp_form = r##"(let* ((arch-packer-process-output-buffer
                          "*arch-packer-filter-output*")
                         (process-buffer
                          (get-buffer-create
                           "*arch-packer-filter-process*"))
                         (arch-packer-subprocess-output nil))
                    (unwind-protect
                        (cl-letf
                            (((symbol-function 'process-buffer)
                              (lambda (_process)
                                process-buffer))
                             ((symbol-function 'get-buffer-window)
                              (lambda (_buffer) nil)))
                          (arch-packer-process-filter
                           'fake-process
                           "downloading package 1/2\n")
                          (arch-packer-process-filter
                           'fake-process
                           "[########] 100%\n")
                          (arch-packer-process-filter
                           'fake-process
                           "installing package\n")
                          (list
                           arch-packer-subprocess-output
                           (with-current-buffer
                               arch-packer-process-output-buffer
                             (buffer-string))
                           (with-current-buffer
                               arch-packer-process-output-buffer
                             major-mode)))
                      (dolist
                          (buffer
                           (list
                            process-buffer
                            (get-buffer
                             arch-packer-process-output-buffer)))
                        (when buffer (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK (nil "downloading package 1/2\ninstalling package\n" arch-packer-output-mode)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn process_filter_error_finished_and_password_branches_dispatch_exact_lifecycle_events() {
    let elisp_form = r##"(let ((process-buffer
                         (get-buffer-create
                          "*arch-packer-filter-branches*"))
                        events)
                    (setq arch-packer-subprocess-output
                          "permission denied")
                    (unwind-protect
                        (cl-letf
                            (((symbol-function 'process-buffer)
                              (lambda (_process) process-buffer))
                             ((symbol-function
                               'arch-packer-disable-status-reporter)
                              (lambda ()
                                (push :disable events)))
                             ((symbol-function
                               'arch-packer-enable-status-reporter)
                              (lambda ()
                                (push :enable events)))
                             ((symbol-function
                               'arch-packer-generate-search-menu)
                              (lambda ()
                                (push :search-menu events)
                                t))
                             ((symbol-function
                               'arch-packer-pkg-menu-async)
                              (lambda ()
                                (push :package-menu events)))
                             ((symbol-function
                               'arch-packer-send-root)
                              (lambda ()
                                (push :password events)))
                             ((symbol-function
                               'arch-packer-wait-shell-subprocess)
                              (lambda ()
                                (push :wait events)))
                             ((symbol-function
                               'arch-packer-get-exit-status)
                              (lambda ()
                                (push :status events)))
                             ((symbol-function 'message)
                              (lambda (format-string &rest args)
                                (push
                                 (apply #'format format-string args)
                                 events))))
                          (arch-packer-process-filter
                           'fake-process "Pacman error\n")
                          (let ((arch-packer-search-string "linux"))
                            (arch-packer-process-filter
                             'fake-process "Pacman finished\n")
                            (push
                             (list :search-after
                                   arch-packer-search-string)
                             events))
                          (let ((arch-packer-search-string nil))
                            (arch-packer-process-filter
                             'fake-process "Pacman finished\n"))
                          (arch-packer-process-filter
                           'fake-process
                           "[sudo] password for user: ")
                          (nreverse events))
                      (kill-buffer process-buffer)))"##;
    let expect = expect![[
        r#"OK (:disable "permission denied" :search-menu :disable "Pacman finished" (:search-after nil) :package-menu :disable "Pacman finished" :disable :password :enable :wait :status)"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn shell_process_open_registers_filter_sentinel_and_initial_output_poll() {
    let elisp_form = r##"(let (events)
                    (cl-letf
                        (((symbol-function 'start-process)
                          (lambda (name buffer program &rest args)
                            (push
                             (list :start name buffer program args)
                             events)
                            'fake-process))
                         ((symbol-function 'get-buffer-process)
                          (lambda (buffer)
                            (push (list :get buffer) events)
                            'fake-process))
                         ((symbol-function 'set-process-filter)
                          (lambda (process filter)
                            (push
                             (list :filter process filter)
                             events)))
                         ((symbol-function 'set-process-sentinel)
                          (lambda (process sentinel)
                            (push
                             (list :sentinel process sentinel)
                             events)))
                         ((symbol-function 'accept-process-output)
                          (lambda (process seconds)
                            (push
                             (list :accept process seconds)
                             events)
                            t)))
                      (arch-packer-open-shell-process)
                      (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((:start "arch-packer-process" "*Pacman-Packages*" "/bin/bash" nil) (:get "*Pacman-Packages*") (:filter fake-process arch-packer-process-filter) (:sentinel fake-process arch-packer-process-sentinel) (:accept fake-process 0.1))"#
    ]];
    assert_arch_packer_parity(elisp_form, expect);
}

#[test]
fn process_sentinel_removes_existing_output_buffer_and_tolerates_missing_buffer() {
    let elisp_form = r##"(let ((arch-packer-process-output-buffer
                         "*arch-packer-sentinel-output*"))
                    (get-buffer-create
                     arch-packer-process-output-buffer)
                    (let ((before
                           (buffer-live-p
                            (get-buffer
                             arch-packer-process-output-buffer))))
                      (arch-packer-process-sentinel
                       'fake-process "finished")
                      (let ((after
                             (get-buffer
                              arch-packer-process-output-buffer)))
                        (arch-packer-process-sentinel
                         'fake-process "finished")
                        (list before after))))"##;
    let expect = expect!["OK (t nil)"];
    assert_arch_packer_parity(elisp_form, expect);
}
