use expect_test::expect;

use super::assert_arduino_cli_mode_parity;

#[test]
fn upstream_keymap_prefix_is_a_customizable_key_sequence() {
    let elisp_form = r##"(list
         arduino-cli-mode-keymap-prefix
         (key-description arduino-cli-mode-keymap-prefix)
         (custom-variable-p 'arduino-cli-mode-keymap-prefix)
         (lookup-key arduino-cli-mode-map arduino-cli-mode-keymap-prefix)
         (keymapp
          (symbol-function
           (lookup-key arduino-cli-mode-map
                       arduino-cli-mode-keymap-prefix))))"##;
    let expect = expect![[
        r#"OK ("\3\1" "C-c C-a" ((funcall #'#[nil ((kbd "C-c C-a")) (t)])) arduino-cli-command-map t)"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn compilation_mode_enables_scrolling_ansi_filter_and_expected_parent() {
    let elisp_form = r##"(with-temp-buffer
         (arduino-cli-compilation-mode)
         (list major-mode
               (derived-mode-p 'compilation-mode)
               compilation-scroll-output
               (memq #'arduino-cli--compilation-filter
                     compilation-filter-hook)
               (featurep 'ansi-color)))"##;
    let expect = expect![
        "OK (arduino-cli-compilation-mode compilation-mode t (arduino-cli--compilation-filter) t)"
    ];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn compilation_filter_only_applies_ansi_colors_when_enabled() {
    let elisp_form = r##"(let (calls)
         (with-temp-buffer
           (insert "\e[31mred\e[0m plain")
           (let ((compilation-filter-start (point-min))
                 (arduino-cli-compile-color nil))
             (cl-letf (((symbol-function 'ansi-color-apply-on-region)
                        (lambda (start end)
                          (push (list start end) calls))))
               (arduino-cli--compilation-filter)
               (let ((arduino-cli-compile-color t))
                 (arduino-cli--compilation-filter)))))
         (nreverse calls))"##;
    let expect = expect!["OK ((1 19))"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn config_init_decline_is_a_true_noop() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'y-or-n-p)
                    (lambda (prompt)
                      (push prompt calls)
                      nil))
                   ((symbol-function 'arduino-cli--message)
                    (lambda (&rest args) (push args calls))))
           (list (arduino-cli-config-init)
                 (nreverse calls))))"##;
    let expect =
        expect![[r#"OK (nil ("Init will override any existing config files, are you sure? "))"#]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn compile_and_upload_without_active_monitor_does_not_register_callback() {
    let elisp_form = r##"(let ((compilation-finish-functions '(existing-hook))
               calls)
         (cl-letf (((symbol-function 'arduino-cli--serial-monitor-is-active)
                    (lambda () nil))
                   ((symbol-function 'arduino-cli--board)
                    (lambda ()
                      '((fqbn . "arduino:avr:uno")
                        (port (address . "/dev/ttyACM0")))))
                   ((symbol-function 'arduino-cli--compile)
                    (lambda (mode cmd)
                      (push (list mode cmd) calls))))
           (arduino-cli-compile-and-upload)
           (list (nreverse calls) compilation-finish-functions)))"##;
    let expect = expect![[
        r#"OK (((compile "compile --fqbn arduino:avr:uno --port /dev/ttyACM0 --upload")) (existing-hook))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn all_command_map_bindings_are_exact_and_publicly_callable() {
    let elisp_form = r##"(mapcar
         (lambda (key)
           (let ((binding
                  (lookup-key arduino-cli-command-map
                              (kbd key))))
             (list key binding (commandp binding))))
         '("c" "b" "u" "n" "l" "i" "U" "k" "m" "M"))"##;
    let expect = expect![[
        r#"OK (("c" arduino-cli-compile t) ("b" arduino-cli-compile-and-upload t) ("u" arduino-cli-upload t) ("n" arduino-cli-new-sketch t) ("l" arduino-cli-board-list t) ("i" arduino-cli-lib-install t) ("U" arduino-cli-lib-uninstall t) ("k" arduino-cli-kill-arduino-connection t) ("m" arduino-cli-start-serial-monitor t) ("M" arduino-cli-stop-serial-monitor t))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}
