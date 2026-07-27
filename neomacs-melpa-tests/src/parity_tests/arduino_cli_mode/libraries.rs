use expect_test::expect;

use super::{assert_arduino_cli_mode_parity, assert_arduino_cli_mode_signal_parity};

#[test]
fn installed_libraries_return_names_or_complete_records() {
    let elisp_form = r##"(let ((payload
                '((installed_libraries
                   . [((library
                        (name . "ArduinoJson")
                        (version . "7.4.1")
                        (install_dir . "/opt/ArduinoJson")))
                      ((library
                        (name . "Servo")
                        (version . "1.2.2")
                        (install_dir . "/opt/Servo")))]))))
         (cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (cmd)
                      (list cmd payload)
                      payload)))
           (list (arduino-cli--libs)
                 (arduino-cli--libs t))))"##;
    let expect = expect![[
        r#"OK (("ArduinoJson" "Servo") [((library (name . "ArduinoJson") (version . "7.4.1") (install_dir . "/opt/ArduinoJson"))) ((library (name . "Servo") (version . "1.2.2") (install_dir . "/opt/Servo")))])"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn installed_libraries_preserve_a_successful_empty_json_array() {
    let elisp_form = r##"(cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) '((installed_libraries . [])))))
         (arduino-cli--libs))"##;
    let expect = expect!["OK nil"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn installed_libraries_missing_result_signals_exact_error() {
    let elisp_form = r##"(cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) nil)))
         (arduino-cli--libs))"##;
    let expect = expect![[r#"ERR (error "ERROR: No libraries installed")"#]];
    assert_arduino_cli_mode_signal_parity(elisp_form, expect);
}

#[test]
fn library_search_returns_complete_candidates_and_rejects_empty_results() {
    let elisp_form = r##"(cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd)
                      '((libraries
                         . [((name . "ArduinoJson")
                             (available_versions . ["7.4.1" "7.3.0"]))
                            ((name . "Servo")
                             (available_versions . ["1.2.2"]))])))))
         (arduino-cli--search-libs))"##;
    let expect = expect![[
        r#"OK [((name . "ArduinoJson") (available_versions . ["7.4.1" "7.3.0"])) ((name . "Servo") (available_versions . ["1.2.2"]))]"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn library_search_preserves_a_successful_empty_json_array() {
    let elisp_form = r##"(cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) '((libraries . [])))))
         (arduino-cli--search-libs))"##;
    let expect = expect!["OK []"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn library_search_missing_result_signals_exact_error() {
    let elisp_form = r##"(cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) nil)))
         (arduino-cli--search-libs))"##;
    let expect = expect![[r#"ERR (error "ERROR: Unable to find libraries")"#]];
    assert_arduino_cli_mode_signal_parity(elisp_form, expect);
}

#[test]
fn library_install_without_version_updates_index_quotes_choice_and_starts_compilation() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--search-libs)
                    (lambda ()
                      '(((name . "Arduino Json"))
                        ((name . "Servo")))))
                   ((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (push (list :completion prompt choices) calls)
                      "Arduino Json"))
                   ((symbol-function 'shell-command-to-string)
                    (lambda (cmd) (push (list :shell cmd) calls) "updated"))
                   ((symbol-function 'compilation-start)
                    (lambda (cmd mode &rest _)
                      (push (list :compile cmd mode) calls)
                      'compilation-buffer)))
           (list (arduino-cli-lib-install nil)
                 arduino-cli--compilation-buffer
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (compilation-buffer compilation-buffer ((:completion "Library " ("Arduino Json" "Servo")) (:shell "arduino-cli lib update-index") (:compile "arduino-cli lib install Arduino\\ Json" arduino-cli-compilation-mode)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn library_install_with_version_flattens_all_name_version_pairs() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--search-libs)
                    (lambda ()
                      '(((name . "ArduinoJson")
                         (available_versions . ["7.4.1" "7.3.0"]))
                        ((name . "Servo")
                         (available_versions . ["1.2.2"])))))
                   ((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (push (list prompt choices) calls)
                      "ArduinoJson@7.3.0"))
                   ((symbol-function 'shell-command-to-string)
                    (lambda (cmd) (push cmd calls) "updated"))
                   ((symbol-function 'compilation-start)
                    (lambda (cmd mode &rest _)
                      (push (list cmd mode) calls)
                      'versioned-install)))
           (list (arduino-cli-lib-install t)
                 arduino-cli--compilation-buffer
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (versioned-install versioned-install (("Library " ("ArduinoJson@7.4.1" "ArduinoJson@7.3.0" "Servo@1.2.2")) "arduino-cli lib update-index" ("arduino-cli lib install ArduinoJson\\@7.3.0" arduino-cli-compilation-mode)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn library_uninstall_selects_installed_name_and_routes_through_message_boundary() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--libs)
                    (lambda (&optional _full)
                      '("ArduinoJson" "Servo")))
                   ((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (push (list prompt choices) calls)
                      "Servo"))
                   ((symbol-function 'arduino-cli--message)
                    (lambda (cmd &rest path)
                      (push (list cmd path) calls)
                      'sent)))
           (list (arduino-cli-lib-uninstall)
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (sent (("Library " ("ArduinoJson" "Servo")) ("lib uninstall Servo" nil)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn library_browse_annotates_candidates_and_opens_selected_install_directory() {
    let elisp_form = r##"(let (calls annotation)
         (cl-letf (((symbol-function 'arduino-cli--libs)
                    (lambda (&optional _full)
                      '(((library
                          (name . "ArduinoJson")
                          (install_dir . "/opt/ArduinoJson")))
                        ((library
                          (name . "Servo")
                          (install_dir . "/opt/Servo"))))))
                   ((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (setq annotation
                            (funcall
                             (plist-get completion-extra-properties
                                        :annotation-function)
                             "Servo"))
                      (push (list prompt choices) calls)
                      "Servo"))
                   ((symbol-function 'find-file)
                    (lambda (path) (push (list :find path) calls) path)))
           (list (arduino-cli-lib-browse)
                 annotation
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/opt/Servo" " (/opt/Servo)" (("Library " ("ArduinoJson" "Servo")) (:find "/opt/Servo")))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}
