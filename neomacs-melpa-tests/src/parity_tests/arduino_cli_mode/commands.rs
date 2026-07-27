use expect_test::expect;

use super::{assert_arduino_cli_mode_parity, assert_arduino_cli_mode_signal_parity};

#[test]
fn compile_builds_fqbn_command_and_routes_it_through_compile_mode() {
    let elisp_form = r##"(let (call)
         (cl-letf (((symbol-function 'arduino-cli--board)
                    (lambda ()
                      '((fqbn . "arduino:avr:uno")
                        (port (address . "/dev/ttyACM0")))))
                   ((symbol-function 'arduino-cli--compile)
                    (lambda (mode cmd) (setq call (list mode cmd)))))
           (list (arduino-cli-compile) call)))"##;
    let expect = expect![[r#"OK (#1=(compile "compile --fqbn arduino:avr:uno") #1#)"#]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn compile_without_fqbn_signals_exact_error_before_process_start() {
    let elisp_form = r##"(cl-letf (((symbol-function 'arduino-cli--board)
                    (lambda () '((port (address . "/dev/ttyACM0"))))))
         (arduino-cli-compile))"##;
    let expect = expect![[r#"ERR (error "ERROR: No fqbn specified")"#]];
    assert_arduino_cli_mode_signal_parity(elisp_form, expect);
}

#[test]
fn compile_and_upload_stops_active_monitor_and_registers_restart_callback() {
    let elisp_form = r##"(let ((compilation-finish-functions nil)
               calls)
         (cl-letf (((symbol-function 'arduino-cli--serial-monitor-is-active)
                    (lambda () t))
                   ((symbol-function 'arduino-cli-stop-serial-monitor)
                    (lambda (&optional reason)
                      (push (list :stop reason) calls)))
                   ((symbol-function 'arduino-cli--board)
                    (lambda ()
                      '((fqbn . "arduino:samd:mkrzero")
                        (port (address . "/dev/ttyACM4")))))
                   ((symbol-function 'arduino-cli--compile)
                    (lambda (mode cmd)
                      (push (list :compile mode cmd) calls))))
           (arduino-cli-compile-and-upload)
           (list (nreverse calls) compilation-finish-functions)))"##;
    let expect = expect![[
        r#"OK (((:stop "to upload a sketch") (:compile compile "compile --fqbn arduino:samd:mkrzero --port /dev/ttyACM4 --upload")) (arduino-cli--start-serial-monitor-callback))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upload_handles_present_and_missing_ports_and_applies_upload_mode() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--serial-monitor-is-active)
                    (lambda () nil))
                   ((symbol-function 'arduino-cli--board)
                    (lambda ()
                      (prog1
                          (if calls
                              '((fqbn . "arduino:avr:nano"))
                            '((fqbn . "arduino:avr:uno")
                              (port (address . "/dev/ttyACM0"))))
                        (push :board calls))))
                   ((symbol-function 'arduino-cli--compile)
                    (lambda (mode cmd)
                      (push (list mode cmd) calls))))
           (arduino-cli-upload)
           (arduino-cli-upload)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (:board (upload "upload --fqbn arduino:avr:uno --port /dev/ttyACM0") :board (upload "upload --fqbn arduino:avr:nano"))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn message_boundary_changes_directory_applies_general_flags_and_trims_output() {
    let elisp_form = r##"(let ((default-directory "/workspace/sketch/")
               (arduino-cli-verbosity 'verbose)
               (arduino-cli-compile-only-verbosity nil)
               calls messages)
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (cmd)
                      (push (list cmd default-directory) calls)
                      "  completed successfully \n"))
                   ((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (let ((text (apply #'format format-string args)))
                        (push text messages)
                        text))))
           (arduino-cli--message "config dump" "/chosen/config/")
           (list (nreverse calls) (nreverse messages)
                 default-directory)))"##;
    let expect = expect![[
        r#"OK ((("arduino-cli config dump --verbose" "/chosen/config/")) ("completed successfully") "/workspace/sketch/")"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn compile_boundary_saves_project_builds_quoted_path_and_remembers_buffer() {
    let elisp_form = r##"(let ((default-directory "/workspace/My Sketch/")
               (arduino-cli-verify t)
               (arduino-cli-warnings 'all)
               (arduino-cli-verbosity 'verbose)
               (arduino-cli-compile-color nil)
               calls)
         (cl-letf (((symbol-function 'save-some-buffers)
                    (lambda (arg predicate)
                      (push (list :save arg (funcall predicate)) calls)))
                   ((symbol-function 'compilation-start)
                    (lambda (cmd mode &rest _)
                      (push (list :compile cmd mode) calls)
                      'build-buffer)))
           (arduino-cli--compile 'compile
                                 "compile --fqbn arduino:avr:uno")
           (list arduino-cli--compilation-buffer
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (build-buffer ((:save nil "/workspace/My Sketch/") (:compile "arduino-cli compile --fqbn arduino:avr:uno /workspace/My\\ Sketch/ -t --warnings all --verbose --no-color" arduino-cli-compilation-mode)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn core_upgrade_workflows_update_index_then_upgrade_selected_or_all_cores() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--cores)
                    (lambda () '("arduino:avr" "esp32:esp32")))
                   ((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (push (list :select prompt choices) calls)
                      "esp32:esp32"))
                   ((symbol-function 'shell-command-to-string)
                    (lambda (cmd) (push (list :shell cmd) calls) "ok"))
                   ((symbol-function 'arduino-cli--message)
                    (lambda (cmd &rest path)
                      (push (list :message cmd path) calls))))
           (arduino-cli-core-upgrade)
           (arduino-cli-core-upgrade-all)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ((:select "Core " ("arduino:avr" "esp32:esp32")) (:shell "arduino-cli core update-index") (:message "core upgrade esp32:esp32" nil) (:shell "arduino-cli core update-index") (:message "core upgrade" nil))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn core_install_and_uninstall_use_search_selection_and_correct_process_boundaries() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--search-cores)
                    (lambda () "rp2040:rp2040"))
                   ((symbol-function 'arduino-cli--cores)
                    (lambda () '("arduino:avr" "rp2040:rp2040")))
                   ((symbol-function 'completing-read)
                    (lambda (_prompt _choices &rest _) "arduino:avr"))
                   ((symbol-function 'shell-command-to-string)
                    (lambda (cmd) (push (list :shell cmd) calls) "ok"))
                   ((symbol-function 'compilation-start)
                    (lambda (cmd mode &rest _)
                      (push (list :compile cmd mode) calls)
                      'core-install-buffer))
                   ((symbol-function 'arduino-cli--message)
                    (lambda (cmd &rest path)
                      (push (list :message cmd path) calls))))
           (arduino-cli-core-install)
           (arduino-cli-core-uninstall)
           (list arduino-cli--compilation-buffer
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (core-install-buffer ((:shell "arduino-cli core update-index") (:compile "arduino-cli core install rp2040:rp2040" arduino-cli-compilation-mode) (:message "core uninstall arduino:avr" nil)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn list_and_config_commands_route_exact_subcommands_through_message_boundary() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--message)
                    (lambda (cmd &rest path)
                      (push (list cmd path) calls)))
                   ((symbol-function 'shell-command-to-string)
                    (lambda (cmd) (push (list :shell cmd) calls) "ok"))
                   ((symbol-function 'y-or-n-p)
                    (lambda (prompt)
                      (push (list :confirm prompt) calls)
                      t)))
           (arduino-cli-board-list)
           (arduino-cli-core-list)
           (arduino-cli-lib-list)
           (arduino-cli-lib-upgrade)
           (arduino-cli-config-init)
           (arduino-cli-config-dump)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (("board list" nil) ("core list" nil) ("lib list" nil) (:shell "arduino-cli lib update-index") ("lib upgrade" nil) (:confirm "Init will override any existing config files, are you sure? ") ("config init" nil) ("config dump" nil))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn new_sketch_expands_selected_directory_and_executes_there() {
    let elisp_form = r##"(let ((default-directory "/workspace/")
               calls)
         (cl-letf (((symbol-function 'read-string)
                    (lambda (prompt)
                      (push prompt calls)
                      "Blink Example"))
                   ((symbol-function 'read-directory-name)
                    (lambda (prompt &rest _)
                      (push prompt calls)
                      "projects/arduino"))
                   ((symbol-function 'arduino-cli--message)
                    (lambda (cmd &rest path)
                      (push (list cmd path) calls))))
           (arduino-cli-new-sketch)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ("Sketch name: " "Sketch path: " ("sketch new Blink Example" ("/workspace/projects/arduino")))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn config_directory_browse_annotates_keys_and_opens_selected_directory() {
    let elisp_form = r##"(let (calls annotation)
         (cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd)
                      '((data . "/home/ada/.arduino15")
                        (downloads . "/home/ada/.arduino15/staging")
                        (user . "/home/ada/Arduino"))))
                   ((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (setq annotation
                            (funcall
                             (plist-get completion-extra-properties
                                        :annotation-function)
                             "user"))
                      (push (list prompt choices) calls)
                      "user"))
                   ((symbol-function 'find-file)
                    (lambda (path) (push (list :find path) calls) path)))
           (list (arduino-cli-config-directory-browse)
                 annotation
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/home/ada/Arduino" " (/home/ada/Arduino)" (("Directory " ("data" "downloads" "user")) (:find "/home/ada/Arduino")))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}
