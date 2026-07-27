use expect_test::expect;

use super::{assert_arduino_cli_mode_autoload_parity, assert_arduino_cli_mode_parity};

#[test]
fn descriptor_records_exact_pin_dependencies_and_payload() {
    let elisp_form = r##"(let* ((desc (cadr (assq 'arduino-cli-mode package-alist)))
              (dir (package-desc-dir desc)))
         (list
          (package-version-join (package-desc-version desc))
          (package-desc-reqs desc)
          (package-desc-kind desc)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files dir t "^[^.].*"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20260628.2219" ((emacs (25 1))) nil ("README-elpa" "arduino-cli-mode-autoloads.el" "arduino-cli-mode-pkg.el" "arduino-cli-mode.el" "arduino-cli-mode.elc"))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arities_and_command_kinds() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)))
         '(arduino-cli--compilation-filter
           arduino-cli--?map-put
           arduino-cli--verify
           arduino-cli--verbosity
           arduino-cli--warnings
           arduino-cli--compile-color
           arduino-cli--general-flags
           arduino-cli--compile-flags
           arduino-cli--add-flags
           arduino-cli--compile
           arduino-cli--message
           arduino-cli--arduino?
           arduino-cli--selected-board?
           arduino-cli--cmd-json
           arduino-cli--default-board
           arduino-cli--board
           arduino-cli--dispatch-board
           arduino-cli--board-fqbn
           arduino-cli--board-address
           arduino-cli--board-name
           arduino-cli--select-board
           arduino-cli--cores
           arduino-cli--search-cores
           arduino-cli--libs
           arduino-cli--search-libs
           arduino-cli--select
           arduino-cli-compile
           arduino-cli-compile-and-upload
           arduino-cli-upload
           arduino-cli-board-list
           arduino-cli-core-list
           arduino-cli-core-upgrade
           arduino-cli-core-upgrade-all
           arduino-cli-kill-arduino-connection
           arduino-cli-core-install
           arduino-cli-core-uninstall
           arduino-cli-lib-list
           arduino-cli-lib-upgrade
           arduino-cli-lib-install
           arduino-cli-lib-uninstall
           arduino-cli-lib-browse
           arduino-cli-new-sketch
           arduino-cli-config-init
           arduino-cli-config-dump
           arduino-cli-config-directory-browse
           arduino-cli--serial-monitor-is-active
           arduino-cli--start-serial-monitor-callback
           arduino-cli-start-serial-monitor
           arduino-cli-stop-serial-monitor
           arduino-cli-mode))"##;
    let expect = expect![
        "OK ((arduino-cli--compilation-filter nil nil nil) (arduino-cli--?map-put (m v k) nil nil) (arduino-cli--verify nil nil nil) (arduino-cli--verbosity nil nil nil) (arduino-cli--warnings nil nil nil) (arduino-cli--compile-color nil nil nil) (arduino-cli--general-flags nil nil nil) (arduino-cli--compile-flags nil nil nil) (arduino-cli--add-flags (mode cmd) nil nil) (arduino-cli--compile (mode cmd) nil nil) (arduino-cli--message (cmd &rest path) nil nil) (arduino-cli--arduino? (usb-device) nil nil) (arduino-cli--selected-board? (board selected-board) nil nil) (arduino-cli--cmd-json (cmd) nil nil) (arduino-cli--default-board nil nil nil) (arduino-cli--board nil nil nil) (arduino-cli--dispatch-board (boards) nil nil) (arduino-cli--board-fqbn (board) nil nil) (arduino-cli--board-address (board) nil nil) (arduino-cli--board-name (board) nil nil) (arduino-cli--select-board (boards) nil nil) (arduino-cli--cores nil nil nil) (arduino-cli--search-cores nil nil nil) (arduino-cli--libs (&optional full) nil nil) (arduino-cli--search-libs nil nil nil) (arduino-cli--select (xs msg) nil nil) (arduino-cli-compile nil t nil) (arduino-cli-compile-and-upload nil t nil) (arduino-cli-upload nil t nil) (arduino-cli-board-list nil t nil) (arduino-cli-core-list nil t nil) (arduino-cli-core-upgrade nil t nil) (arduino-cli-core-upgrade-all nil t nil) (arduino-cli-kill-arduino-connection nil t nil) (arduino-cli-core-install nil t nil) (arduino-cli-core-uninstall nil t nil) (arduino-cli-lib-list nil t nil) (arduino-cli-lib-upgrade nil t nil) (arduino-cli-lib-install (select-version-p) t nil) (arduino-cli-lib-uninstall nil t nil) (arduino-cli-lib-browse nil t nil) (arduino-cli-new-sketch nil t nil) (arduino-cli-config-init nil t nil) (arduino-cli-config-dump nil t nil) (arduino-cli-config-directory-browse nil t nil) (arduino-cli--serial-monitor-is-active nil nil nil) (arduino-cli--start-serial-monitor-callback (compilation-buffer process-finish-status) nil nil) (arduino-cli-start-serial-monitor (&optional monitor-baud-rate) t nil) (arduino-cli-stop-serial-monitor (&optional reason) t nil) (arduino-cli-mode (&optional arg) t nil))"
    ];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn customization_surface_records_defaults_types_and_group_membership() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (default-value symbol)
                 (custom-variable-p symbol)
                 (get symbol 'custom-type)
                 (get symbol 'custom-group)))
         '(arduino-cli-mode-keymap-prefix
           arduino-cli-default-fqbn
           arduino-cli-default-port
           arduino-cli-verify
           arduino-cli-warnings
           arduino-cli-verbosity
           arduino-cli-compile-only-verbosity
           arduino-cli-compile-color
           arduino-cli-monitor-buffer-name))"##;
    let expect = expect![[
        r#"OK ((arduino-cli-mode-keymap-prefix "\3\1" ((funcall #'#[nil ((kbd "C-c C-a")) #1=(t)])) string nil) (arduino-cli-default-fqbn nil ((funcall #'#[nil (nil) #1#])) (choice (const :tag "No default (error message if board selection fails)" nil) (string :tag "Fully qualified board name")) nil) (arduino-cli-default-port nil ((funcall #'#[nil (nil) #1#])) (choice (const :tag "No default (error message if board selection fails)" nil) (string :tag "Port address")) nil) (arduino-cli-verify nil ((funcall #'#[nil (nil) #1#])) boolean nil) (arduino-cli-warnings nil ((funcall #'#[nil (nil) #1#])) (choice (const :tag "--warnings default" default) (const :tag "--warnings more" more) (const :tag "--warnings all" all) (const :tag "No warnings flag; default level is \"none\"" nil)) nil) (arduino-cli-verbosity nil ((funcall #'#[nil (nil) #1#])) (choice (const :tag "Quiet" quiet) (const :tag "Verbose" verbose) (const :tag "None" nil)) nil) (arduino-cli-compile-only-verbosity t ((funcall #'#[nil (t) #1#])) boolean nil) (arduino-cli-compile-color t ((funcall #'#[nil (t) #1#])) boolean nil) (arduino-cli-monitor-buffer-name "arduino cli monitor" ((funcall #'#[nil ("arduino cli monitor") (arduino-cli-compilation-mode-abbrev-table arduino-cli-compilation-mode-syntax-table . #1#)])) string nil))"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn package_loads_declared_dependencies_feature_and_compilation_mode() {
    let elisp_form = r##"(list
         (featurep 'arduino-cli-mode)
         (mapcar
          (lambda (feature) (list feature (featurep feature)))
          '(compile json map seq subr-x))
         (derived-mode-p 'arduino-cli-compilation-mode)
         (get 'arduino-cli-compilation-mode 'derived-mode-parent)
         (get 'arduino-cli 'group-documentation))"##;
    let expect = expect![[
        r#"OK (t ((compile t) (json t) (map t) (seq t) (subr-x t)) nil compilation-mode "Arduino-cli-mode functions and settings.")"#
    ]];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}

#[test]
fn autoload_file_records_exact_public_command_and_mode_surface() {
    let elisp_form = r##"(list
         (featurep 'arduino-cli-mode)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (autoloadp (symbol-function symbol))
                  (commandp symbol)))
          '(arduino-cli-compile
            arduino-cli-compile-and-upload
            arduino-cli-upload
            arduino-cli-board-list
            arduino-cli-core-list
            arduino-cli-core-upgrade
            arduino-cli-core-upgrade-all
            arduino-cli-kill-arduino-connection
            arduino-cli-core-install
            arduino-cli-core-uninstall
            arduino-cli-lib-list
            arduino-cli-lib-upgrade
            arduino-cli-lib-install
            arduino-cli-lib-uninstall
            arduino-cli-lib-browse
            arduino-cli-new-sketch
            arduino-cli-config-init
            arduino-cli-config-dump
            arduino-cli-config-directory-browse
            arduino-cli-start-serial-monitor
            arduino-cli-stop-serial-monitor
            arduino-cli-mode)))"##;
    let expect = expect![
        "OK (nil ((arduino-cli-compile nil nil) (arduino-cli-compile-and-upload nil nil) (arduino-cli-upload nil nil) (arduino-cli-board-list nil nil) (arduino-cli-core-list nil nil) (arduino-cli-core-upgrade nil nil) (arduino-cli-core-upgrade-all nil nil) (arduino-cli-kill-arduino-connection nil nil) (arduino-cli-core-install nil nil) (arduino-cli-core-uninstall nil nil) (arduino-cli-lib-list nil nil) (arduino-cli-lib-upgrade nil nil) (arduino-cli-lib-install nil nil) (arduino-cli-lib-uninstall nil nil) (arduino-cli-lib-browse nil nil) (arduino-cli-new-sketch nil nil) (arduino-cli-config-init nil nil) (arduino-cli-config-dump nil nil) (arduino-cli-config-directory-browse nil nil) (arduino-cli-start-serial-monitor nil nil) (arduino-cli-stop-serial-monitor nil nil) (arduino-cli-mode t t)))"
    ];
    assert_arduino_cli_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn reload_is_idempotent_for_feature_hooks_and_keymap_state() {
    let elisp_form = r##"(let ((source (locate-library "arduino-cli-mode")))
         (load source nil 'nomessage)
         (load source nil 'nomessage)
         (load source nil 'nomessage)
         (list
          (cl-count 'arduino-cli-mode features)
          (lookup-key arduino-cli-command-map (kbd "c"))
          (lookup-key arduino-cli-mode-map arduino-cli-mode-keymap-prefix)
          (cl-count #'arduino-cli--compilation-filter
                    compilation-filter-hook)))"##;
    let expect = expect!["OK (1 arduino-cli-compile arduino-cli-command-map 0)"];
    assert_arduino_cli_mode_parity(elisp_form, expect);
}
