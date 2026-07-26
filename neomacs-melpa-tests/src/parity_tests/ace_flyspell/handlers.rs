use expect_test::expect;

use super::assert_ace_flyspell_parity;

#[test]
fn ace_flyspell_help_default_sets_and_returns_the_exact_prompt() {
    let elisp_form = r##"(list
               (ace-flyspell-help-default)
               (current-message))"##;
    let expect = expect![[r#"OK ("[.]: correct word; [,]: save to personal dictionary" nil)"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_auto_correct_calls_flyspell_before_refreshing_help() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'flyspell-auto-correct-word)
                     (lambda ()
                       (push
                        'correct
                        calls)
                       'correction-result))
                    ((symbol-function
                      'ace-flyspell-help-default)
                     (lambda ()
                       (push
                        'help
                        calls)
                       'help-result)))
                 (list
                  (ace-flyspell--auto-correct-word)
                  (nreverse
                   calls))))"##;
    let expect = expect!["OK (help-result (correct help))"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_reset_clears_the_message_and_deletes_an_active_overlay() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "misspelt")
               (let ((ace-flyspell--ov
                      (make-overlay
                       1
                       5)))
                 (message
                  "before")
                 (let ((result
                        (ace-flyspell--reset)))
                   (list
                    result
                    (current-message)
                    (overlay-buffer
                     ace-flyspell--ov)
                    (overlay-start
                     ace-flyspell--ov)
                    (overlay-end
                     ace-flyspell--ov)))))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_reset_is_idempotent_for_its_initial_deleted_overlay() {
    let elisp_form = r##"(list
               (ace-flyspell--reset)
               (ace-flyspell--reset)
               (overlay-buffer
                ace-flyspell--ov)
               (overlay-start
                ace-flyspell--ov)
               (overlay-end
                ace-flyspell--ov))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_insert_word_sends_unhighlights_saves_resets_and_returns_to_mark() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "wrng tail")
               (goto-char
                (point-max))
               (push-mark
                6
                t)
               (let ((ispell-pdict-modified-p
                      nil)
                     (ace-flyspell-new-word-no-query
                      nil)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional following)
                         (push
                          (list
                           'word
                           following
                           (point))
                          calls)
                         '("wrng" 1 5)))
                      ((symbol-function
                        'ispell-send-string)
                       (lambda (string)
                         (push
                          (list
                           'send
                           string)
                          calls)))
                      ((symbol-function
                        'flyspell-unhighlight-at)
                       (lambda (position)
                         (push
                          (list
                           'unhighlight
                           position)
                          calls)))
                      ((symbol-function
                        'ispell-pdict-save)
                       (lambda (no-query)
                         (push
                          (list
                           'save
                           no-query)
                          calls)))
                      ((symbol-function
                        'ace-flyspell--reset)
                       (lambda ()
                         (push
                          'reset
                          calls))))
                   (list
                    (ace-flyspell--insert-word)
                    (point)
                    (mark)
                    ispell-pdict-modified-p
                    (nreverse
                     calls)))))"##;
    let expect = expect![[
        r#"OK (6 6 6 (t) ((word nil 10) (send "*wrng\n") (unhighlight 1) (save nil) reset))"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_insert_word_honors_no_query_and_skips_absent_unhighlight_support() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "wrng")
               (push-mark
                3
                t)
               (let ((ispell-pdict-modified-p
                      nil)
                     (ace-flyspell-new-word-no-query
                      t)
                     (real-fboundp
                      (symbol-function
                       'fboundp))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'flyspell-get-word)
                       (lambda (&optional _following)
                         '("wrng" 1 5)))
                      ((symbol-function
                        'ispell-send-string)
                       (lambda (string)
                         (push
                          (list
                           'send
                           string)
                          calls)))
                      ((symbol-function
                        'ispell-pdict-save)
                       (lambda (no-query)
                         (push
                          (list
                           'save
                           no-query)
                          calls)))
                      ((symbol-function
                        'ace-flyspell--reset)
                       (lambda ()
                         (push
                          'reset
                          calls)))
                      ((symbol-function
                        'fboundp)
                       (lambda (symbol)
                         (if
                             (eq
                              symbol
                              'flyspell-unhighlight-at)
                             nil
                           (funcall
                            real-fboundp
                            symbol)))))
                   (list
                    (ace-flyspell--insert-word)
                    (point)
                    ispell-pdict-modified-p
                    (nreverse
                     calls)))))"##;
    let expect = expect![[r#"OK (3 3 (t) ((send "*wrng\n") (save t) reset))"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_insert_word_missing_word_mutates_before_unhighlight_signals() {
    let elisp_form = r##"(let ((ispell-pdict-modified-p
                    nil)
                   (real-unhighlight
                    (symbol-function
                     'flyspell-unhighlight-at))
                   calls)
               (cl-letf
                   (((symbol-function
                      'flyspell-get-word)
                     (lambda (&optional _following)
                       (push
                        'word
                        calls)
                       nil))
                    ((symbol-function
                      'ispell-send-string)
                     (lambda (string)
                       (push
                        (list
                         'send
                         string)
                        calls)))
                    ((symbol-function
                      'flyspell-unhighlight-at)
                     (lambda (position)
                       (push
                        (list
                         'unhighlight
                         position)
                        calls)
                       (funcall
                        real-unhighlight
                        position)))
                    ((symbol-function
                      'ispell-pdict-save)
                     (lambda (no-query)
                       (push
                        (list
                         'save
                         no-query)
                        calls)))
                    ((symbol-function
                      'ace-flyspell--reset)
                     (lambda ()
                       (push
                        'reset
                        calls))))
                 (let ((error-value
                        (condition-case error-data
                            (ace-flyspell--insert-word)
                          (error
                           error-data))))
                   (list
                    error-value
                    ispell-pdict-modified-p
                    (nreverse
                     calls)))))"##;
    let expect = expect![[
        r#"OK ((wrong-type-argument integer-or-marker-p nil) (t) (word (send "*\n") (unhighlight nil)))"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_default_handler_exits_immediately_when_read_key_returns_nil() {
    let elisp_form = r##"(let ((help-calls
                    0)
                   (read-calls
                    0))
               (cl-letf
                   (((symbol-function
                      'ace-flyspell-help-default)
                     (lambda ()
                       (setq help-calls
                             (1+
                              help-calls))))
                    ((symbol-function
                      'read-key)
                     (lambda ()
                       (setq read-calls
                             (1+
                              read-calls))
                       nil)))
                 (list
                  (ace-flyspell-default-handler)
                  help-calls
                  read-calls)))"##;
    let expect = expect!["OK (nil 1 1)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_default_handler_repeats_period_corrections_until_another_key() {
    let elisp_form = r##"(let ((keys
                    '(46 46 120))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ace-flyspell-help-default)
                     (lambda ()
                       (push
                        'help
                        calls)))
                    ((symbol-function
                      'read-key)
                     (lambda ()
                       (let ((key
                              (pop
                               keys)))
                         (push
                          (list
                           'read
                           key)
                          calls)
                         key)))
                    ((symbol-function
                      'ace-flyspell--auto-correct-word)
                     (lambda ()
                       (push
                        'correct
                        calls))))
                 (list
                  (ace-flyspell-default-handler)
                  (nreverse
                   calls)
                  keys)))"##;
    let expect = expect![
        "OK (nil (help (read 46) help correct (read 46) help correct (read 120) help) nil)"
    ];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_default_handler_comma_inserts_then_continues_until_another_key() {
    let elisp_form = r##"(let ((keys
                    '(44 113))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ace-flyspell-help-default)
                     (lambda ()
                       (push
                        'help
                        calls)))
                    ((symbol-function
                      'read-key)
                     (lambda ()
                       (let ((key
                              (pop
                               keys)))
                         (push
                          (list
                           'read
                           key)
                          calls)
                         key)))
                    ((symbol-function
                      'ace-flyspell--insert-word)
                     (lambda ()
                       (push
                        'insert
                        calls))))
                 (list
                  (ace-flyspell-default-handler)
                  (nreverse
                   calls)
                  keys)))"##;
    let expect = expect!["OK (nil (help (read 44) help insert (read 113) help) nil)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_default_handler_control_g_restores_the_original_word() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "wrng tail")
               (goto-char
                6)
               (let ((ace-flyspell--current-word
                      "wrong")
                     (ace-flyspell--ov
                      (make-overlay
                       1
                       5))
                     (help-calls
                      0)
                     (read-calls
                      0))
                 (cl-letf
                     (((symbol-function
                        'ace-flyspell-help-default)
                       (lambda ()
                         (setq help-calls
                               (1+
                                help-calls))))
                      ((symbol-function
                        'read-key)
                       (lambda ()
                         (setq read-calls
                               (1+
                                read-calls))
                         7)))
                   (list
                    (ace-flyspell-default-handler)
                    (buffer-string)
                    (point)
                    help-calls
                    read-calls
                    (overlay-start
                     ace-flyspell--ov)
                    (overlay-end
                     ace-flyspell--ov)))))"##;
    let expect = expect![[r#"OK (nil " wrongtail" 7 2 1 1 1)"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}
