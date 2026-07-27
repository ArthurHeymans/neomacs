use expect_test::expect;

use super::assert_amread_mode_autoload_parity;

#[test]
fn generated_autoloads_register_commands_and_expand_the_hydra_without_loading_feature() {
    let elisp_form = r##"(list
                      (featurep 'amread-mode)
                      (mapcar
                       (lambda (symbol)
                         (let ((definition
                                (symbol-function symbol)))
                           (if (autoloadp definition)
                               (list
                                symbol
                                'autoload
                                (nth 1 definition)
                                (nth 3 definition)
                                (commandp symbol))
                             (list
                              symbol
                              'expanded
                              (byte-code-function-p definition)
                              (commandp symbol)))))
                       '(amread-start
                         amread-stop
                         amread-pause-or-resume
                         amread-mode-quit
                         amread-speed-up
                         amread-speed-down
                         amread-voice-reader-toggle
                         amread-voice-reader-switch-language-voice
                         amread-voice-reader-read-buffer
                         amread-hydra/body
                         amread-mode))
                      (and
                       (member
                        (file-name-directory
                         (getenv "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![[
        r#"OK (nil ((amread-start autoload "amread-mode" t t) (amread-stop autoload "amread-mode" t t) (amread-pause-or-resume autoload "amread-mode" t t) (amread-mode-quit autoload "amread-mode" t t) (amread-speed-up autoload "amread-mode" t t) (amread-speed-down autoload "amread-mode" t t) (amread-voice-reader-toggle autoload "amread-mode" t t) (amread-voice-reader-switch-language-voice autoload "amread-mode" t t) (amread-voice-reader-read-buffer autoload "amread-mode" t t) (amread-hydra/body expanded nil t) (amread-mode autoload "amread-mode" t t)) nil)"#
    ]];
    assert_amread_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn invoking_speed_command_through_autoload_loads_package_and_preserves_user_value() {
    let elisp_form = r##"(progn
                      (setq amread-word-speed 2.5)
                      (let (messages)
                        (cl-letf
                            (((symbol-function 'message)
                              (lambda
                                  (format-string &rest arguments)
                                (let ((rendered
                                       (apply
                                        #'format
                                        format-string arguments)))
                                  (when
                                      (string-prefix-p
                                       "[amread]" rendered)
                                    (push rendered messages)))
                                nil)))
                          (list
                           (featurep 'amread-mode)
                           (amread-speed-up)
                           amread-word-speed
                           (featurep 'amread-mode)
                           (commandp 'amread-speed-up)
                           (nreverse messages)))))"##;
    let expect = expect![[r#"OK (nil nil 2.7 t t ("[amread] word speed increased -> 2.7"))"#]];
    assert_amread_mode_autoload_parity(elisp_form, expect);
}
