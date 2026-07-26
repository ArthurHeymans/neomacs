use expect_test::expect;

use super::assert_ac_ispell_parity;

#[test]
fn ac_ispell_correct_word_short_circuits_without_async_process_or_with_empty_word() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ispell-set-spellchecker-params)
                     (lambda ()
                       (push 'params calls)
                       (error
                        "protocol must not start")))
                    ((symbol-function
                      'ispell-accept-buffer-local-defs)
                     (lambda ()
                       (push 'defs calls)
                       (error
                        "protocol must not start")))
                    ((symbol-function
                      'ispell-send-string)
                     (lambda (_string)
                       (push 'send calls)
                       (error
                        "protocol must not start")))
                    ((symbol-function
                      'accept-process-output)
                     (lambda (&rest _arguments)
                       (push 'accept calls)
                       (error
                        "protocol must not start")))
                    ((symbol-function
                      'ispell-parse-output)
                     (lambda (_output)
                       (push 'parse calls)
                       (error
                        "protocol must not start"))))
                 (list
                  (let ((ispell-async-processp
                         nil))
                    (ac-ispell--correct-word
                     "word"))
                  (let ((ispell-async-processp
                         t))
                    (ac-ispell--correct-word
                     ""))
                  calls)))"##;
    let expect = expect![[r#"OK (nil nil nil)"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_correct_word_runs_the_exact_async_protocol_and_returns_suggestions() {
    let elisp_form = r##"(let ((ispell-async-processp
                    t)
                   (ispell-process
                    'fixture-process)
                   (ispell-filter
                    '("pending"
                      "unused"))
                   (accept-count 0)
                   events)
               (cl-letf
                   (((symbol-function
                      'ispell-set-spellchecker-params)
                     (lambda ()
                       (push '(params) events)))
                    ((symbol-function
                      'ispell-accept-buffer-local-defs)
                     (lambda ()
                       (push '(defs) events)))
                    ((symbol-function
                      'ispell-send-string)
                     (lambda (string)
                       (push
                        (list 'send string)
                        events)))
                    ((symbol-function
                      'accept-process-output)
                     (lambda (&rest arguments)
                       (push
                        (cons 'accept arguments)
                        events)
                       (setq accept-count
                             (1+ accept-count)
                             ispell-filter
                             (if (= accept-count 1)
                                 '("still-running"
                                   "unused")
                               '(""
                                 "parsed-output")))
                       t))
                    ((symbol-function
                      'ispell-parse-output)
                     (lambda (output)
                       (push
                        (list 'parse output)
                        events)
                       '(fixture
                         original
                         ("word" "ward")
                         metadata))))
                 (list
                  (ac-ispell--correct-word
                   "wrod")
                  accept-count
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (("word" "ward") 2 ((params) (defs) (send "%\n") (send "^wrod\n") (accept fixture-process nil nil 1) (accept fixture-process nil nil 1) (parse "parsed-output")))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_correct_word_returns_nil_for_non_list_parser_results() {
    let elisp_form = r##"(let ((ispell-async-processp
                    t)
                   (ispell-process
                    'fixture-process)
                   (ispell-filter
                    '("pending"
                      "unused")))
               (cl-letf
                   (((symbol-function
                      'ispell-set-spellchecker-params)
                     #'ignore)
                    ((symbol-function
                      'ispell-accept-buffer-local-defs)
                     #'ignore)
                    ((symbol-function
                      'ispell-send-string)
                     #'ignore)
                    ((symbol-function
                      'accept-process-output)
                     (lambda (&rest _arguments)
                       (setq
                        ispell-filter
                        '(""
                          "parser-input"))
                       t))
                    ((symbol-function
                      'ispell-parse-output)
                     (lambda (_output)
                       'not-a-list)))
                 (ac-ispell--correct-word
                  "word")))"##;
    let expect = expect![[r#"OK nil"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_correct_word_returns_nil_when_parser_list_has_no_suggestions_slot() {
    let elisp_form = r##"(let ((ispell-async-processp
                    t)
                   (ispell-process
                    'fixture-process)
                   (ispell-filter
                    '("pending"
                      "unused")))
               (cl-letf
                   (((symbol-function
                      'ispell-set-spellchecker-params)
                     #'ignore)
                    ((symbol-function
                      'ispell-accept-buffer-local-defs)
                     #'ignore)
                    ((symbol-function
                      'ispell-send-string)
                     #'ignore)
                    ((symbol-function
                      'accept-process-output)
                     (lambda (&rest _arguments)
                       (setq
                        ispell-filter
                        '(""
                          "parser-input"))
                       t))
                    ((symbol-function
                      'ispell-parse-output)
                     (lambda (_output)
                       '(fixture original))))
                 (ac-ispell--correct-word
                  "word")))"##;
    let expect = expect![[r#"OK nil"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_fuzzy_candidates_forwards_the_exact_prefix_and_result() {
    let elisp_form = r##"(let ((ac-prefix
                    "Wrod")
                   events
                   (result
                    '("Word" "Woad")))
               (cl-letf
                   (((symbol-function
                      'ac-ispell--correct-word)
                     (lambda (word)
                       (push word events)
                       result)))
                 (let ((returned
                        (ac-ispell--fuzzy-candidates)))
                   (list
                    returned
                    (eq returned result)
                    (nreverse events)))))"##;
    let expect = expect![[r#"OK (("Word" "Woad") t ("Wrod"))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}
