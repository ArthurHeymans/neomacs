use expect_test::expect;

use super::assert_arduino_cli_mode_parity;

#[test]
fn flag_helpers_cover_the_full_option_cross_product() {
    let elisp_form = r##"(let (rows)
         (dolist (verify '(nil t))
           (dolist (warnings '(nil default more all))
             (dolist (verbosity '(nil quiet verbose))
               (dolist (compile-only '(nil t))
                 (dolist (color '(nil t))
                   (let ((arduino-cli-verify verify)
                         (arduino-cli-warnings warnings)
                         (arduino-cli-verbosity verbosity)
                         (arduino-cli-compile-only-verbosity compile-only)
                         (arduino-cli-compile-color color))
                     (push
                      (list verify warnings verbosity compile-only color
                            (arduino-cli--general-flags)
                            (arduino-cli--compile-flags)
                            (arduino-cli--add-flags 'compile "compile")
                            (arduino-cli--add-flags 'message "message"))
                      rows)))))))
         (secure-hash 'sha256 (prin1-to-string (nreverse rows))))"##;
    let expect =
        expect![[r#"OK "c1d6edae56ab65b26a731ecb25e6b56ae4a17b0498e8e3ce7bdec251d2c77b8d""#]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn map_put_builds_default_board_without_mutating_false_values() {
    let elisp_form = r##"(let ((arduino-cli-default-fqbn "arduino:avr:uno")
               (arduino-cli-default-port "/dev/ttyACM0"))
         (list
          (arduino-cli--default-board)
          (let ((arduino-cli-default-port nil))
            (arduino-cli--default-board))
          (let ((arduino-cli-default-fqbn nil))
            (arduino-cli--default-board))
          (let ((arduino-cli-default-fqbn nil)
                (arduino-cli-default-port nil))
            (arduino-cli--default-board))
          (arduino-cli--?map-put '((kept . 1)) nil 'ignored)))"##;
    let expect = expect![[
        r#"OK (((port (address . "/dev/ttyACM0")) (fqbn . "arduino:avr:uno")) ((fqbn . "arduino:avr:uno")) ((port (address . "/dev/ttyACM0"))) nil ((kept . 1)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_default_fqbn_custom_type_accepts_strings_and_warns_for_numbers() {
    let elisp_form = r##"(let ((original (default-value
                          'arduino-cli-default-fqbn))
               results)
         (dolist (value (list (default-value 'arduino-cli-default-fqbn)
                              "vendor:architecture:board_id"
                              1))
           (let ((warnings nil))
             (cl-letf (((symbol-function 'warn)
                        (lambda (&rest args) (push args warnings))))
               (setopt arduino-cli-default-fqbn value)
               (push (list value arduino-cli-default-fqbn
                           (not (null warnings)))
                     results))))
         (list original
               (nreverse results)
               (get 'arduino-cli-default-fqbn 'custom-type)))"##;
    let expect = expect![[
        r#"OK (nil ((nil nil nil) ("vendor:architecture:board_id" "vendor:architecture:board_id" nil) (1 1 t)) (choice (const :tag "No default (error message if board selection fails)" nil) (string :tag "Fully qualified board name")))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_default_port_custom_type_accepts_unix_and_windows_ports() {
    let elisp_form = r##"(let ((original (default-value
                          'arduino-cli-default-port))
               results)
         (dolist (value (list (default-value 'arduino-cli-default-port)
                              "/dev/ttyACM2" "COM3" 1))
           (let ((warnings nil))
             (cl-letf (((symbol-function 'warn)
                        (lambda (&rest args) (push args warnings))))
               (setopt arduino-cli-default-port value)
               (push (list value arduino-cli-default-port
                           (not (null warnings)))
                     results))))
         (list original
               (nreverse results)
               (get 'arduino-cli-default-port 'custom-type)))"##;
    let expect = expect![[
        r#"OK (nil ((nil nil nil) ("/dev/ttyACM2" "/dev/ttyACM2" nil) ("COM3" "COM3" nil) (1 1 t)) (choice (const :tag "No default (error message if board selection fails)" nil) (string :tag "Port address")))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_verify_custom_option_accepts_nil_and_t() {
    let elisp_form = r##"(let ((original
                (default-value 'arduino-cli-verify)))
         (list original
               (setopt arduino-cli-verify original)
               arduino-cli-verify
               (setopt arduino-cli-verify nil)
               arduino-cli-verify
               (setopt arduino-cli-verify t)
               arduino-cli-verify
               (get 'arduino-cli-verify 'custom-type)))"##;
    let expect = expect!["OK (nil nil nil nil nil t t boolean)"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_compile_only_verbosity_custom_option_accepts_nil_and_t() {
    let elisp_form = r##"(let ((original
                (default-value
                 'arduino-cli-compile-only-verbosity)))
         (list original
               (setopt arduino-cli-compile-only-verbosity
                       original)
               arduino-cli-compile-only-verbosity
               (setopt arduino-cli-compile-only-verbosity nil)
               arduino-cli-compile-only-verbosity
               (setopt arduino-cli-compile-only-verbosity t)
               arduino-cli-compile-only-verbosity
               (get 'arduino-cli-compile-only-verbosity
                    'custom-type)))"##;
    let expect = expect!["OK (t t t nil nil t t boolean)"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_compile_color_custom_option_accepts_nil_and_t() {
    let elisp_form = r##"(let ((original
                (default-value 'arduino-cli-compile-color)))
         (list original
               (setopt arduino-cli-compile-color original)
               arduino-cli-compile-color
               (setopt arduino-cli-compile-color nil)
               arduino-cli-compile-color
               (setopt arduino-cli-compile-color t)
               arduino-cli-compile-color
               (get 'arduino-cli-compile-color 'custom-type)))"##;
    let expect = expect!["OK (t t t nil nil t t boolean)"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_warning_option_accepts_documented_symbols_and_warns_for_numbers() {
    let elisp_form = r##"(let ((original (default-value
                          'arduino-cli-warnings))
               results)
         (dolist (value '(nil default more all 1))
           (let ((warnings nil))
             (cl-letf (((symbol-function 'warn)
                        (lambda (&rest args) (push args warnings))))
               (setopt arduino-cli-warnings value)
               (push (list value arduino-cli-warnings
                           (not (null warnings)))
                     results))))
         (list original
               (nreverse results)
               (get 'arduino-cli-warnings 'custom-type)))"##;
    let expect = expect![[
        r#"OK (nil ((nil nil nil) (default default nil) (more more nil) (all all nil) (1 1 t)) (choice (const :tag "--warnings default" default) (const :tag "--warnings more" more) (const :tag "--warnings all" all) (const :tag "No warnings flag; default level is \"none\"" nil)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn upstream_verbosity_option_accepts_documented_symbols_and_warns_for_numbers() {
    let elisp_form = r##"(let ((original (default-value
                          'arduino-cli-verbosity))
               results)
         (dolist (value '(nil quiet verbose 1))
           (let ((warnings nil))
             (cl-letf (((symbol-function 'warn)
                        (lambda (&rest args) (push args warnings))))
               (setopt arduino-cli-verbosity value)
               (push (list value arduino-cli-verbosity
                           (not (null warnings)))
                     results))))
         (list original
               (nreverse results)
               (get 'arduino-cli-verbosity 'custom-type)))"##;
    let expect = expect![[
        r#"OK (nil ((nil nil nil) (quiet quiet nil) (verbose verbose nil) (1 1 t)) (choice (const :tag "Quiet" quiet) (const :tag "Verbose" verbose) (const :tag "None" nil)))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}
