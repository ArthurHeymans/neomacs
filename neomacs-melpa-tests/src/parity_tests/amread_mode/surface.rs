use expect_test::expect;

use super::assert_amread_mode_parity;

#[test]
fn package_defaults_custom_safety_face_and_dependency_contracts_match() {
    let elisp_form = r##"(list
                      (featurep 'amread-mode)
                      (featurep 'pyim)
                      (featurep 'hydra)
                      (list
                       amread-word-speed
                       amread-line-speed
                       amread-scroll-style
                       amread-voice-reader-enabled
                       amread-voice-reader-command
                       amread-voice-reader-command-options
                       amread-voice-reader-language)
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (get symbol 'custom-type)
                          (get symbol 'safe-local-variable)
                          (get symbol 'custom-group)))
                       '(amread-word-speed
                         amread-line-speed
                         amread-scroll-style
                         amread-voice-reader-enabled
                         amread-voice-reader-command
                         amread-voice-reader-command-options
                         amread-voice-reader-language))
                      (get
                       'amread-highlight-face 'face-defface-spec)
                      amread--voice-reader-voice-models
                      (list
                       amread--timer
                       amread--current-position
                       amread--overlay
                       amread--voice-reader-proc-finished
                       amread--voice-reader-engine-initialized))"##;
    let expect = expect![[
        r#"OK (t t t (3.0 4.0 nil nil nil ("--rate=200") chinese) ((amread-word-speed float floatp nil) (amread-line-speed float floatp nil) (amread-scroll-style (choice (const :tag "scroll by word" word) (const :tag "scroll by line" line)) symbolp nil) (amread-voice-reader-enabled boolean booleanp nil) (amread-voice-reader-command string stringp nil) (amread-voice-reader-command-options string stringp nil) (amread-voice-reader-language symbol symbolp nil)) ((t :foreground "black" :background "ForestGreen")) ("Samantha" "Ava" "Vicki" "Alex" "Tingting" "Binbin" "Sinji" "Meijia" "Kyoko" "Otoya" "Yuna") (nil nil nil nil nil))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn complete_shipped_callable_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (symbol)
                        (list
                         symbol
                         (fboundp symbol)
                         (help-function-arglist symbol t)
                         (commandp symbol)))
                      '(amread--voice-reader-set-language
                        amread--voice-reader-read-text
                        amread--voice-reader-run-python-code-to-string
                        amread--voice-reader-run-python-file-to-string
                        amread--voice-reader-run-python-code-in-repl
                        amread--voice-reader-read-text-with-tts
                        amread--voice-reader-read-text-with-say
                        amread--word-update
                        amread--line-update
                        amread--update
                        amread--scroll-style-ask
                        amread--get-line-words
                        amread--get-next-line-words
                        amread--get-line-length
                        amread--get-next-line-length
                        amread-start
                        amread-stop
                        amread-pause-or-resume
                        amread-mode-quit
                        amread-speed-up
                        amread-speed-down
                        amread-voice-reader-toggle
                        amread--voice-reader-detect-language
                        amread-voice-reader-switch-language-voice
                        amread-voice-reader-read-buffer
                        amread-hydra/body
                        amread-mode))"##;
    let expect = expect![
        "OK ((amread--voice-reader-set-language t (&optional language) nil) (amread--voice-reader-read-text t (text) nil) (amread--voice-reader-run-python-code-to-string t (&rest python-code-lines) nil) (amread--voice-reader-run-python-file-to-string t (python-code-file) nil) (amread--voice-reader-run-python-code-in-repl t (&rest python-code-lines) nil) (amread--voice-reader-read-text-with-tts t (text) nil) (amread--voice-reader-read-text-with-say t (text &optional language voice) nil) (amread--word-update t nil nil) (amread--line-update t nil nil) (amread--update t nil nil) (amread--scroll-style-ask t nil nil) (amread--get-line-words t (&optional pos) nil) (amread--get-next-line-words t nil nil) (amread--get-line-length t (&optional pos) nil) (amread--get-next-line-length t nil nil) (amread-start t nil t) (amread-stop t nil t) (amread-pause-or-resume t nil t) (amread-mode-quit t nil t) (amread-speed-up t nil t) (amread-speed-down t nil t) (amread-voice-reader-toggle t nil t) (amread--voice-reader-detect-language t (&optional string) nil) (amread-voice-reader-switch-language-voice t (&optional language) t) (amread-voice-reader-read-buffer t nil t) (amread-hydra/body t nil t) (amread-mode t (&optional arg) t))"
    ];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn mode_keymap_and_hydra_expose_the_complete_reading_control_panel() {
    let elisp_form = r##"(list
                      (mapcar
                       (lambda (key)
                         (cons
                          key
                          (lookup-key
                           amread-mode-map (kbd key))))
                       '("q" "SPC" "C-g" "+" "-" "v" "L" "."))
                      (mapcar
                       (lambda (key)
                         (cons
                          key
                          (lookup-key
                           amread-hydra/keymap (kbd key))))
                       '("SPC" "q" "+" "-" "v" "L"))
                      (commandp 'amread-hydra/body)
                      (keymapp amread-mode-map)
                      (keymapp amread-hydra/keymap)
                      (where-is-internal
                       'amread-mode-quit amread-mode-map))"##;
    let expect = expect![[
        r#"OK ((("q" . amread-mode-quit) ("SPC" . amread-pause-or-resume) ("C-g") ("+" . amread-speed-up) ("-" . amread-speed-down) ("v" . amread-voice-reader-toggle) ("L" . amread-voice-reader-switch-language-voice) ("." . amread-hydra/body)) (("SPC" . amread-hydra/amread-pause-or-resume-and-exit) ("q" . amread-hydra/amread-mode-quit) ("+" . amread-hydra/amread-speed-up-and-exit) ("-" . amread-hydra/amread-speed-down-and-exit) ("v" . amread-hydra/amread-voice-reader-toggle) ("L" . amread-hydra/amread-voice-reader-switch-language-voice)) t t t ([113] [7]))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn minor_mode_lifecycle_is_buffer_local_runs_hooks_and_delegates_start_stop() {
    let elisp_form = r##"(let ((first
                           (generate-new-buffer
                            " *amread-first*"))
                          (second
                           (generate-new-buffer
                            " *amread-second*"))
                          events)
                      (unwind-protect
                          (cl-letf
                              (((symbol-function 'amread-start)
                                (lambda ()
                                  (push
                                   (list
                                    'start
                                    (buffer-name)
                                    amread-mode)
                                   events)
                                  'started))
                               ((symbol-function 'amread-stop)
                                (lambda ()
                                  (push
                                   (list
                                    'stop
                                    (buffer-name)
                                    amread-mode)
                                   events)
                                  'stopped)))
                            (with-current-buffer first
                              (add-hook
                               'amread-mode-hook
                               (lambda ()
                                 (push
                                  (list
                                   'hook
                                   (buffer-name)
                                   amread-mode)
                                  events))
                               nil t)
                              (amread-mode 1))
                            (with-current-buffer second
                              (amread-mode 1)
                              (amread-mode -1))
                            (list
                             (with-current-buffer first
                               amread-mode)
                             (with-current-buffer second
                               amread-mode)
                             (nreverse events)
                             (assq
                              'amread-mode
                              minor-mode-alist)
                             (assq
                              'amread-mode
                              minor-mode-map-alist)))
                        (kill-buffer first)
                        (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK (t nil ((start " *amread-first*" t) (hook " *amread-first*" t) (start " *amread-second*" t) (stop " *amread-second*" nil)) (amread-mode " amread") (amread-mode keymap (46 . amread-hydra/body) (76 . amread-voice-reader-switch-language-voice) (118 . amread-voice-reader-toggle) (45 . amread-speed-down) (43 . amread-speed-up) (remap keymap (keyboard-quit . amread-mode-quit)) (32 . amread-pause-or-resume) (113 . amread-mode-quit)))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}
