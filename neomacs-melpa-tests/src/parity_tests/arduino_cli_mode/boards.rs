use expect_test::expect;

use super::{assert_arduino_cli_mode_parity, assert_arduino_cli_mode_signal_parity};

#[test]
fn board_accessors_handle_flat_default_and_nested_detected_board_shapes() {
    let elisp_form = r##"(let ((flat '((fqbn . "vendor:arch:default")
                       (port (address . "/dev/default"))))
               (nested '((name . "Uno")
                         (port (address . "/dev/ttyACM0")
                               (protocol . "serial"))
                         (matching_boards
                          . [((name . "Arduino Uno")
                              (fqbn . "arduino:avr:uno"))
                             ((name . "Other")
                              (fqbn . "other:arch:board"))]))))
         (list
          (arduino-cli--board-fqbn flat)
          (arduino-cli--board-address flat)
          (arduino-cli--board-fqbn nested)
          (arduino-cli--board-address nested)
          (arduino-cli--board-name nested)
          (arduino-cli--selected-board? nested "/dev/ttyACM0")
          (arduino-cli--selected-board? nested "/dev/ttyUSB9")))"##;
    let expect = expect![[
        r#"OK ("vendor:arch:default" "/dev/default" "arduino:avr:uno" "/dev/ttyACM0" "Uno @ /dev/ttyACM0" t nil)"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn dispatch_board_covers_zero_one_and_multiple_board_workflows() {
    let elisp_form = r##"(let ((one '((name . "Uno")
                      (port (address . "/dev/ttyACM0"))))
               (two '((name . "Nano")
                      (port (address . "/dev/ttyUSB1"))))
               prompts)
         (cl-letf (((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (push (list prompt choices) prompts)
                      "Nano @ /dev/ttyUSB1")))
           (list
            (arduino-cli--dispatch-board nil)
            (arduino-cli--dispatch-board (list one))
            (arduino-cli--dispatch-board (list one two))
            (nreverse prompts))))"##;
    let expect = expect![[
        r#"OK (nil ((name . "Uno") (port (address . "/dev/ttyACM0"))) ((name . "Nano") (port (address . "/dev/ttyUSB1"))) (("Board " ("Uno @ /dev/ttyACM0" "Nano @ /dev/ttyUSB1"))))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn board_filters_non_arduino_ports_merges_first_matching_board_and_selects_one() {
    let elisp_form = r##"(let ((payload
                '((detected_ports
                   . [((port (address . "/dev/ttyS0"))
                       (protocol . "serial"))
                      ((port (address . "/dev/ttyACM0"))
                       (protocol . "serial")
                       (matching_boards
                        . [((name . "Arduino Uno")
                            (fqbn . "arduino:avr:uno"))]))
                      ((port (address . "/dev/ttyUSB1"))
                       (protocol . "serial")
                       (matching_boards
                        . [((name . "Nano")
                            (fqbn . "arduino:avr:nano"))]))]))))
         (cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) payload))
                   ((symbol-function 'completing-read)
                    (lambda (_prompt _choices &rest _)
                      "Nano @ /dev/ttyUSB1")))
           (arduino-cli--board)))"##;
    let expect = expect![[
        r#"OK ((port (address . "/dev/ttyUSB1")) (protocol . "serial") (matching_boards . [((name . "Nano") (fqbn . "arduino:avr:nano"))]))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn board_uses_complete_default_when_no_hardware_is_detected() {
    let elisp_form = r##"(let ((arduino-cli-default-fqbn "esp8266:esp8266:nodemcuv2")
               (arduino-cli-default-port "/dev/ttyUSB0"))
         (cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) '((detected_ports . [])))))
           (arduino-cli--board)))"##;
    let expect =
        expect![[r#"OK ((port (address . "/dev/ttyUSB0")) (fqbn . "esp8266:esp8266:nodemcuv2"))"#]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn board_without_hardware_or_defaults_signals_the_documented_error() {
    let elisp_form = r##"(let ((arduino-cli-default-fqbn nil)
               (arduino-cli-default-port nil))
         (cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) '((detected_ports . [])))))
           (arduino-cli--board)))"##;
    let expect = expect![[r#"ERR (error "ERROR: No board connected")"#]];
    assert_arduino_cli_mode_signal_parity(elisp_form, expect);
}

#[test]
fn json_command_constructs_exact_cli_command_and_parses_nested_payload() {
    let elisp_form = r##"(let (command)
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (value)
                      (setq command value)
                      "{\"platforms\":[{\"id\":\"arduino:avr\"}],\"ok\":true}")))
           (list command
                 (prog1 (arduino-cli--cmd-json "core list")
                   (setq command command))
                 command)))"##;
    let expect = expect![[
        r#"OK (nil ((platforms . [((id . "arduino:avr"))]) (ok . t)) "arduino-cli core list --format json")"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn cores_and_search_cores_extract_ids_and_preserve_completion_choices() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (cmd)
                      (push cmd calls)
                      (if (string= cmd "core list")
                          '((platforms
                             . [((id . "arduino:avr"))
                                ((id . "esp32:esp32"))]))
                        '((platforms
                           . [((id . "arduino:samd"))
                              ((id . "rp2040:rp2040"))])))))
                   ((symbol-function 'completing-read)
                    (lambda (prompt choices &rest _)
                      (push (list prompt choices) calls)
                      "rp2040:rp2040")))
           (list (arduino-cli--cores)
                 (arduino-cli--search-cores)
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("arduino:avr" "esp32:esp32") "rp2040:rp2040" ("core list" "core search" ("Core " ("arduino:samd" "rp2040:rp2040"))))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn cores_without_installed_platforms_signals_exact_error() {
    let elisp_form = r##"(cl-letf (((symbol-function 'arduino-cli--cmd-json)
                    (lambda (_cmd) '((platforms . [])))))
         (arduino-cli--cores))"##;
    let expect = expect![[r#"ERR (error "ERROR: No cores installed")"#]];
    assert_arduino_cli_mode_signal_parity(elisp_form, expect);
}
