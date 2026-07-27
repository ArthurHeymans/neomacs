use expect_test::expect;

use super::assert_amread_mode_parity;

#[test]
fn line_word_and_length_helpers_measure_real_multilingual_and_empty_lines_without_moving_point() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "one two three\n"
                       "中文 mixed 文本\n"
                       "\n"
                       "last-word")
                      (goto-char 5)
                      (let ((original (point)))
                        (list
                         (amread--get-line-words)
                         (amread--get-line-length)
                         (amread--get-line-words
                          (save-excursion
                            (forward-line 1)
                            (point)))
                         (amread--get-line-length
                          (save-excursion
                            (forward-line 1)
                            (point)))
                         (amread--get-line-words
                          (save-excursion
                            (forward-line 2)
                            (point)))
                         (amread--get-line-length
                          (save-excursion
                            (forward-line 2)
                            (point)))
                         (amread--get-line-words
                          (point-max))
                         (amread--get-line-length
                          (point-max))
                         (point)
                         original)))"##;
    let expect = expect!["OK (3 13 3 11 0 0 2 9 5 5)"];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn next_line_helpers_follow_point_across_boundaries_and_preserve_the_historical_length_contract() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "current line\n"
                       "alpha beta gamma\n"
                       "123456789\n")
                      (goto-char (point-min))
                      (let ((first-point (point)))
                        (list
                         (amread--get-next-line-words)
                         (amread--get-next-line-length)
                         (point)
                         first-point
                         (progn
                           (forward-line 1)
                           (list
                            (amread--get-next-line-words)
                            (amread--get-next-line-length)))
                         (progn
                           (forward-line 1)
                           (list
                            (amread--get-next-line-words)
                            (amread--get-next-line-length)))
                         (point))))"##;
    let expect = expect!["OK (3 3 1 1 (1 1) (0 0) 31)"];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn word_updates_advance_real_mixed_text_overlay_and_resume_position_then_stop_at_eob() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "Alpha beta—中文 gamma ")
                      (goto-char (point-min))
                      (let ((amread--overlay nil)
                            (amread--current-position nil)
                            (amread-mode t)
                            words disabled states)
                        (cl-letf
                            (((symbol-function
                               'amread--voice-reader-read-text)
                              (lambda (text)
                                (push text words)))
                             ((symbol-function 'amread-mode)
                              (lambda (argument)
                                (push argument disabled)
                                (setq amread-mode
                                      (> argument 0)))))
                          (dotimes (_ 4)
                            (amread--word-update)
                            (push
                             (list
                              (point)
                              amread--current-position
                              (and amread--overlay
                                   (overlay-start
                                    amread--overlay))
                              (and amread--overlay
                                   (overlay-end
                                    amread--overlay))
                              (and amread--overlay
                                   (buffer-substring-no-properties
                                    (overlay-start
                                     amread--overlay)
                                    (overlay-end
                                     amread--overlay)))
                              (and amread--overlay
                                   (overlay-get
                                    amread--overlay 'face)))
                             states))
                          (amread--word-update)
                          (list
                           (nreverse states)
                           (nreverse words)
                           (nreverse disabled)
                           (point)
                           amread--current-position
                           amread-mode))))"##;
    let expect = expect![[
        r#"OK (((7 6 1 6 "Alpha" amread-highlight-face) (12 12 7 12 "beta—" amread-highlight-face) (15 14 12 14 "中文" amread-highlight-face) (21 20 15 20 "gamma" amread-highlight-face)) ("Alpha" "beta—" "中文" "gamma") (-1) 21 nil nil)"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn word_update_at_page_boundary_calls_nov_navigation_after_highlighting_last_word() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "chapter-end ")
                      (goto-char (point-min))
                      (setq major-mode 'nov-mode)
                      (let ((amread--overlay nil)
                            (amread--current-position nil)
                            events)
                        (cl-letf
                            (((symbol-function
                               'amread--voice-reader-read-text)
                              (lambda (text)
                                (push
                                 (list 'voice text)
                                 events)))
                             ((symbol-function 'nov-next-document)
                              (lambda ()
                                (push '(next-document)
                                      events)
                                'next))
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
                          (amread--word-update)
                          (list
                           (point)
                           (eobp)
                           amread--current-position
                           (buffer-substring-no-properties
                            (overlay-start
                             amread--overlay)
                            (overlay-end
                             amread--overlay))
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (13 t 12 "chapter-end" ((voice "chapter-end") (next-document) (message "[amread] nov.el next page.")))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn line_updates_highlight_and_read_each_real_line_then_disable_cleanly_at_eob() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "alpha beta\n"
                       "中文 line\n"
                       "last\n")
                      (goto-char (point-min))
                      (let ((amread--overlay nil)
                            (amread--current-position 17)
                            (amread-mode t)
                            lines disabled states)
                        (cl-letf
                            (((symbol-function
                               'amread--voice-reader-read-text)
                              (lambda (text)
                                (push text lines)))
                             ((symbol-function 'amread-mode)
                              (lambda (argument)
                                (push argument disabled)
                                (setq amread-mode
                                      (> argument 0)))))
                          (dotimes (_ 3)
                            (amread--line-update)
                            (push
                             (list
                              (line-number-at-pos)
                              (point)
                              (buffer-substring-no-properties
                               (overlay-start
                                amread--overlay)
                               (overlay-end
                                amread--overlay))
                              (overlay-get
                               amread--overlay 'face))
                             states))
                          (amread--line-update)
                          (list
                           (nreverse states)
                           (nreverse lines)
                           (nreverse disabled)
                           (point)
                           amread--current-position
                           amread-mode))))"##;
    let expect = expect![[
        r#"OK (((2 12 "alpha beta" amread-highlight-face) (3 20 "中文 line" amread-highlight-face) (4 25 "last" amread-highlight-face)) ("alpha beta" "中文 line" "last") (-1) 25 nil nil)"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn line_update_at_page_boundary_reads_line_and_advances_nov_document() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "final page line\n")
                      (goto-char (point-min))
                      (setq major-mode 'nov-mode)
                      (let ((amread--overlay nil)
                            events)
                        (cl-letf
                            (((symbol-function
                               'amread--voice-reader-read-text)
                              (lambda (text)
                                (push
                                 (list 'voice text)
                                 events)))
                             ((symbol-function 'nov-next-document)
                              (lambda ()
                                (push '(next-document)
                                      events)
                                'next))
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
                          (amread--line-update)
                          (list
                           (point)
                           (eobp)
                           (buffer-substring-no-properties
                            (overlay-start
                             amread--overlay)
                            (overlay-end
                             amread--overlay))
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (17 t "final page line" ((voice "final page line") (next-document) (message "[amread] nov.el next page.")))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn update_dispatcher_honors_every_voice_process_state_for_word_scrolling() {
    let elisp_form = r##"(let ((amread-scroll-style 'word)
                          (amread-voice-reader-enabled nil)
                          (amread--voice-reader-proc-finished
                           'running)
                          events states)
                      (cl-letf
                          (((symbol-function 'amread--word-update)
                            (lambda ()
                              (push
                               (list
                                'word
                                amread--voice-reader-proc-finished)
                               events)
                              'updated)))
                        (amread--update)
                        (push
                         amread--voice-reader-proc-finished
                         states)
                        (setq
                         amread-voice-reader-enabled t
                         amread--voice-reader-proc-finished
                         'not-started)
                        (amread--update)
                        (push
                         amread--voice-reader-proc-finished
                         states)
                        (setq amread--voice-reader-proc-finished
                              'running)
                        (amread--update)
                        (push
                         amread--voice-reader-proc-finished
                         states)
                        (setq amread--voice-reader-proc-finished
                              'finished)
                        (amread--update)
                        (push
                         amread--voice-reader-proc-finished
                         states)
                        (setq amread--voice-reader-proc-finished
                              'unexpected)
                        (amread--update)
                        (push
                         amread--voice-reader-proc-finished
                         states)
                        (list
                         (nreverse events)
                         (nreverse states))))"##;
    let expect = expect![
        "OK (((word running) (word not-started) (word finished)) (running not-started running finished not-started))"
    ];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn line_dispatcher_updates_real_overlay_and_adapts_running_timer_to_upcoming_words() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "first\n"
                       "second\n"
                       "alpha beta gamma delta epsilon zeta\n"
                       "tail\n")
                      (goto-char (point-min))
                      (let ((amread-scroll-style 'line)
                            (amread-voice-reader-enabled nil)
                            (amread-word-speed 2.0)
                            (amread--timer
                             (timer-create))
                            (amread--overlay nil)
                            voices)
                        (setf
                         (timer--repeat-delay
                          amread--timer)
                         9)
                        (cl-letf
                            (((symbol-function
                               'amread--voice-reader-read-text)
                              (lambda (text)
                                (push text voices))))
                          (amread--update)
                          (let ((first
                                 (list
                                  (line-number-at-pos)
                                  (buffer-substring-no-properties
                                   (overlay-start
                                    amread--overlay)
                                   (overlay-end
                                    amread--overlay))
                                  (timer--repeat-delay
                                   amread--timer))))
                            (amread--update)
                            (list
                             first
                             (list
                              (line-number-at-pos)
                              (buffer-substring-no-properties
                               (overlay-start
                                amread--overlay)
                               (overlay-end
                                amread--overlay))
                              (timer--repeat-delay
                               amread--timer))
                             (nreverse voices))))))"##;
    let expect = expect![[r#"OK ((2 "first" 3) (3 "second" 3) ("first" "second"))"#]];
    assert_amread_mode_parity(elisp_form, expect);
}

#[test]
fn update_dispatcher_rejects_missing_or_unknown_scroll_styles_with_exact_signal() {
    let elisp_form = r##"(mapcar
                      (lambda (style)
                        (let ((amread-scroll-style style))
                          (condition-case error-data
                              (amread--update)
                            (error
                             (list
                              (car error-data)
                              (cdr error-data))))))
                      '(nil page paragraph))"##;
    let expect = expect![[
        r#"OK ((user-error ("Seems amread-mode is not normally started or not running")) (user-error ("Seems amread-mode is not normally started or not running")) (user-error ("Seems amread-mode is not normally started or not running")))"#
    ]];
    assert_amread_mode_parity(elisp_form, expect);
}
