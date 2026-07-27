use expect_test::expect;

use super::assert_amread_mode_parity;

#[test]
fn scroll_style_prompt_returns_and_stores_the_selected_reading_strategy() {
    let elisp_form = r##"(let ((amread-scroll-style nil)
                          prompts)
                      (cl-letf
                          (((symbol-function 'completing-read)
                            (lambda
                                (prompt collection
                                 &rest arguments)
                              (push
                               (list
                                prompt collection
                                arguments)
                               prompts)
                              "line")))
                        (list
                         (amread--scroll-style-ask)
                         amread-scroll-style
                         (nreverse prompts))))"##;
    let expect =
        expect![[r#"OK (line line (("amread-mode scroll style: " ("word" "line") nil)))"#]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn start_word_session_resumes_exact_position_and_builds_fractional_timer_and_controls() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "zero one two three")
                      (goto-char (point-min))
                      (let ((amread-scroll-style 'word)
                            (amread-word-speed 2.5)
                            (amread--current-position 10)
                            (amread--timer nil)
                            events)
                        (cl-letf
                            (((symbol-function
                               'amread--voice-reader-set-language)
                              (lambda (&optional language)
                                (push
                                 (list 'language language)
                                 events)
                                'english))
                             ((symbol-function 'run-with-timer)
                              (lambda
                                  (delay repeat function
                                   &rest arguments)
                                (push
                                 (list
                                  'timer delay repeat
                                  function arguments)
                                 events)
                                'word-timer))
                             ((symbol-function
                               'amread-hydra/body)
                              (lambda ()
                                (push '(hydra) events)
                                'opened))
                             ((symbol-function 'message)
                              (lambda
                                  (format-string &rest arguments)
                                (push
                                 (list
                                  'message
                                  (apply
                                   #'format
                                   format-string arguments))
                                 events))))
                          (list
                           (amread-start)
                           buffer-read-only
                           (point)
                           (char-after)
                           amread--timer
                           amread--voice-reader-proc-finished
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (#1=((message "[amread] start reading...")) t 10 116 word-timer not-started ((language nil) (timer 0 0.4 amread--update nil) (hydra) . #1#))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn start_line_session_prompts_once_resumes_line_number_and_uses_line_timer() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "line zero\n"
                       "line one\n"
                       "line two\n"
                       "line three\n")
                      (goto-char (point-max))
                      (let ((amread-scroll-style nil)
                            (amread-line-speed 4.5)
                            (amread--current-position 2)
                            events)
                        (cl-letf
                            (((symbol-function
                               'amread--scroll-style-ask)
                              (lambda ()
                                (push '(ask-style) events)
                                (setq amread-scroll-style
                                      'line)))
                             ((symbol-function
                               'amread--voice-reader-set-language)
                              (lambda (&optional language)
                                (push
                                 (list 'language language)
                                 events)
                                'chinese))
                             ((symbol-function 'run-with-timer)
                              (lambda
                                  (delay repeat function
                                   &rest arguments)
                                (push
                                 (list
                                  'timer delay repeat
                                  function arguments)
                                 events)
                                'line-timer))
                             ((symbol-function
                               'amread-hydra/body)
                              (lambda ()
                                (push '(hydra) events)
                                'opened))
                             ((symbol-function 'message)
                              (lambda
                                  (format-string &rest arguments)
                                (push
                                 (list
                                  'message
                                  (apply
                                   #'format
                                   format-string arguments))
                                 events))))
                          (list
                           (amread-start)
                           amread-scroll-style
                           (line-number-at-pos)
                           (buffer-substring-no-properties
                            (line-beginning-position)
                            (line-end-position))
                           amread--timer
                           buffer-read-only
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (#1=((message "[amread] start reading...")) line 3 "line two" line-timer t ((ask-style) (language nil) (timer 1 4.5 amread--update nil) (hydra) . #1#))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn start_rejects_unknown_style_after_read_only_and_language_initialization() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "reading")
                      (let ((amread-scroll-style 'page)
                            events result)
                        (cl-letf
                            (((symbol-function
                               'amread--voice-reader-set-language)
                              (lambda (&optional language)
                                (push
                                 (list 'language language)
                                 events)
                                'english))
                             ((symbol-function
                               'amread-hydra/body)
                              (lambda ()
                                (push '(hydra) events))))
                          (setq result
                                (condition-case error-data
                                    (amread-start)
                                  (error
                                   (list
                                    (car error-data)
                                    (cdr error-data)))))
                          (prog1
                              (list
                               result
                               buffer-read-only
                               amread--voice-reader-proc-finished
                               (nreverse events))
                            (read-only-mode -1)))))"##;
    let expect = expect![[
        r#"OK ((user-error ("Seems amread-mode is not normally started because of not selecting scroll style OR just not running")) t not-started ((language nil)))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn stop_cancels_real_timer_removes_overlay_resets_style_and_restores_editing() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "editable reading text")
                      (let* ((amread-scroll-style 'word)
                             (amread--timer
                              (run-with-timer
                               3600 nil #'ignore))
                             (original-timer
                              amread--timer)
                             (amread--overlay
                              (make-overlay 1 9))
                             (original-overlay
                              amread--overlay)
                             events)
                        (read-only-mode 1)
                        (overlay-put
                         amread--overlay
                         'face 'amread-highlight-face)
                        (cl-letf
                            (((symbol-function
                               'hydra-keyboard-quit)
                              (lambda ()
                                (push '(hydra-quit)
                                      events)
                                'quit))
                             ((symbol-function 'message)
                              (lambda
                                  (format-string &rest arguments)
                                (push
                                 (list
                                  'message
                                  (apply
                                   #'format
                                   format-string arguments))
                                 events))))
                          (list
                           (amread-stop)
                           amread--timer
                           amread-scroll-style
                           buffer-read-only
                           (timerp original-timer)
                           (overlay-buffer
                            original-overlay)
                           (overlay-start
                            original-overlay)
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (#1=((message "[amread] stopped.")) nil nil nil t nil nil ((hydra-quit) . #1#))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn pause_resume_and_quit_route_to_exact_lifecycle_actions() {
    let elisp_form = r##"(let ((amread--timer 'running-timer)
                          events)
                      (cl-letf
                          (((symbol-function 'amread-stop)
                            (lambda ()
                              (push '(stop) events)
                              (setq amread--timer nil)
                              'stopped))
                           ((symbol-function 'amread-start)
                            (lambda ()
                              (push '(start) events)
                              (setq amread--timer
                                    'new-timer)
                              'started))
                           ((symbol-function 'amread-mode)
                            (lambda (argument)
                              (push
                               (list 'mode argument)
                               events)
                              'disabled))
                           ((symbol-function
                             'hydra-keyboard-quit)
                            (lambda ()
                              (push '(hydra-quit)
                                    events)
                              'quit)))
                        (list
                         (amread-pause-or-resume)
                         amread--timer
                         (amread-pause-or-resume)
                         amread--timer
                         (amread-mode-quit)
                         (nreverse events))))"##;
    let expect =
        expect!["OK (stopped nil started new-timer quit ((stop) (start) (mode -1) (hydra-quit)))"];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn speed_and_voice_controls_mutate_session_settings_and_emit_user_feedback() {
    let elisp_form = r##"(let ((amread-word-speed 3.0)
                          (amread-voice-reader-enabled nil)
                          messages)
                      (cl-letf
                          (((symbol-function 'message)
                            (lambda
                                (format-string &rest arguments)
                              (push
                               (apply
                                #'format
                                format-string arguments)
                               messages))))
                        (list
                         (amread-speed-up)
                         amread-word-speed
                         (amread-speed-up)
                         amread-word-speed
                         (amread-speed-down)
                         amread-word-speed
                         (amread-voice-reader-toggle)
                         amread-voice-reader-enabled
                         (amread-voice-reader-toggle)
                         amread-voice-reader-enabled
                         (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (#5=("[amread] word speed increased -> 3.2" . #1=("[amread] word speed increased -> 3.4000000000000004" . #2=("[amread] word speed decreased -> 3.2" . #3=("[amread] voice reader enabled." . #4=("[amread] voice reader disabled."))))) 3.2 #1# 3.4000000000000004 #2# 3.2 #3# t #4# nil #5#)"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}
