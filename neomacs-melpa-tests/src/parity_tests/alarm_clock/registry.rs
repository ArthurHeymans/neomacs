use expect_test::expect;

use super::{assert_alarm_clock_autoload_parity, assert_alarm_clock_parity};

#[test]
fn alarm_clock_registry_defaults_custom_types_and_packaged_sound_match() {
    let elisp_form = r##"(list
         (featurep 'alarm-clock)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (if (eq symbol 'alarm-clock-sound-file)
                      (list (file-name-nondirectory (symbol-value symbol))
                            (file-exists-p (symbol-value symbol)))
                    (symbol-value symbol))
                  (get symbol 'custom-type)
                  (get symbol 'custom-group)))
          '(alarm-clock-sound-file
            alarm-clock-play-sound
            alarm-clock-play-sound-repeat
            alarm-clock-play-auto-view-alarms
            alarm-clock-system-notify
            alarm-clock-alert-notify
            alarm-clock-cache-file
            alarm-clock-auto-save))
         (mapcar #'symbol-value
                 '(alarm-clock--alist
                   alarm-clock--macos-sender
                   alarm-clock--stopped))
         (get 'alarm-clock 'group-documentation))"##;
    let expect = expect![[
        r#"OK (t ((alarm-clock-sound-file ("alarm.mp3" t) file nil) (alarm-clock-play-sound t boolean nil) (alarm-clock-play-sound-repeat 1 integer nil) (alarm-clock-play-auto-view-alarms nil boolean nil) (alarm-clock-system-notify t boolean nil) (alarm-clock-alert-notify t boolean nil) (alarm-clock-cache-file "[ORACLE-HOME]/.emacs.d/.alarm-clock.cache" string nil) (alarm-clock-auto-save t boolean nil)) (nil nil nil) "An alarm clock management.")"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_complete_callable_surface_arglists_aliases_and_commands_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)
                 (autoloadp (symbol-function symbol))
                 (and (memq symbol
                            '(alarm-clock-turn-autosave-on
                              alarm-clock-turn-autosave-off))
                      (indirect-function symbol))))
         '(alarm-clock-mode
           alarm-clock-set
           alarm-clock--set
           alarm-clock--preparse-time
           alarm-clock--maybe-auto-save
           alarm-clock-list-view
           alarm-clock--compare
           alarm-clock--sort-list
           alarm-clock--list-prepare
           alarm-clock-stop
           alarm-clock-kill
           alarm-clock--unexpired-alarms
           alarm-clock--remove-expired
           alarm-clock--ding-on-timer
           alarm-clock--ding
           alarm-clock--system-notify
           alarm-clock--notify
           alarm-clock-restore
           alarm-clock--formatted-cache
           alarm-clock-save
           alarm-clock--kill-all
           alarm-clock-turn-autosave-on
           alarm-clock--turn-autosave-on
           alarm-clock-turn-autosave-off
           alarm-clock--turn-autosave-off
           alarm-clock--get-macos-sender))"##;
    let expect = expect![[
        r#"OK ((alarm-clock-mode nil t nil nil nil) (alarm-clock-set (time message) t nil nil nil) (alarm-clock--set (time message) nil nil nil nil) (alarm-clock--preparse-time (time) nil nil nil nil) (alarm-clock--maybe-auto-save nil nil nil nil nil) (alarm-clock-list-view nil t nil nil nil) (alarm-clock--compare (a b) nil nil nil nil) (alarm-clock--sort-list nil nil nil nil nil) (alarm-clock--list-prepare nil nil nil nil nil) (alarm-clock-stop nil t nil nil nil) (alarm-clock-kill nil t nil nil nil) (alarm-clock--unexpired-alarms nil nil nil nil nil) (alarm-clock--remove-expired nil nil nil nil nil) (alarm-clock--ding-on-timer (program sound repeat) nil nil nil nil) (alarm-clock--ding nil nil nil nil nil) (alarm-clock--system-notify (title message) nil nil nil nil) (alarm-clock--notify (title message) nil nil nil nil) (alarm-clock-restore nil t nil nil nil) (alarm-clock--formatted-cache nil nil nil nil nil) (alarm-clock-save nil t nil nil nil) (alarm-clock--kill-all nil nil nil nil nil) (alarm-clock-turn-autosave-on nil nil nil nil #[nil ((add-hook 'kill-emacs-hook #'alarm-clock-save)) #1=(alarm-clock-mode-abbrev-table alarm-clock-mode-syntax-table t) nil "Enable saving the alarm when killing emacs"]) (alarm-clock--turn-autosave-on nil nil nil nil nil) (alarm-clock-turn-autosave-off nil nil nil nil #[nil ((remove-hook 'kill-emacs-hook #'alarm-clock-save)) #1# nil "Disable auto-saving the alarm when killing emacs"]) (alarm-clock--turn-autosave-off nil nil nil nil nil) (alarm-clock--get-macos-sender nil nil nil nil nil))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_autoload_contract_exposes_documented_commands_without_loading_source() {
    let elisp_form = r##"(list
         (featurep 'alarm-clock)
         (mapcar
          (lambda (symbol)
            (let ((definition (symbol-function symbol)))
              (list symbol
                    (autoloadp definition)
                    (nth 1 definition)
                    (nth 4 definition)
                    (commandp symbol))))
          '(alarm-clock-set
            alarm-clock-list-view
            alarm-clock-stop
            alarm-clock-restore
            alarm-clock-save
            alarm-clock--set
            alarm-clock--notify)))"##;
    let expect = expect![[
        r#"OK (nil ((alarm-clock-set t "alarm-clock" nil t) (alarm-clock-list-view t "alarm-clock" nil t) (alarm-clock-stop t "alarm-clock" nil t) (alarm-clock-restore t "alarm-clock" nil t) (alarm-clock-save t "alarm-clock" nil t) (alarm-clock--set nil nil nil nil) (alarm-clock--notify nil nil nil nil)))"#
    ]];
    assert_alarm_clock_autoload_parity(elisp_form, expect);
}

#[test]
fn alarm_clock_mode_real_buffer_contract_and_all_documented_keys_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert "discarded undo history")
         (alarm-clock-mode)
         (list
          major-mode
          mode-name
          (derived-mode-p 'special-mode)
          buffer-read-only
          truncate-lines
          (eq buffer-undo-list t)
          (mapcar
           (lambda (key)
             (list key (key-binding (kbd key))))
           '("C-k" "d" "a" "i" "+" "-" "g" "SPC"))))"##;
    let expect = expect![[
        r#"OK (alarm-clock-mode "Alarm Clock" special-mode t t t (("C-k" alarm-clock-kill) ("d" alarm-clock-kill) ("a" alarm-clock-set) ("i" alarm-clock-set) ("+" alarm-clock-set) ("-" alarm-clock-kill) ("g" alarm-clock-list-view) ("SPC" alarm-clock-stop)))"#
    ]];
    assert_alarm_clock_parity(elisp_form, expect);
}
