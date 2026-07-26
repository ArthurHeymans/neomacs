use expect_test::expect;

use super::assert_ac_octave_parity;

#[test]
fn ac_octave_do_complete_sends_word_at_point_sorts_deduplicates_and_updates_global_state() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "before foo_bar after")
               (search-backward
                " after")
               (let ((inferior-octave-output-list
                      nil)
                     (ac-octave-complete-list
                      'old)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'inferior-octave-send-list-and-digest)
                       (lambda (commands)
                         (push commands calls)
                         (setq
                          inferior-octave-output-list
                          '("zeta"
                            "alpha"
                            "zeta"
                            "beta"))
                         'sent)))
                   (let ((before
                          (point)))
                     (list
                      (ac-octave-do-complete)
                      before
                      (point)
                      (nreverse calls)
                      ac-octave-complete-list
                      inferior-octave-output-list)))))"##;
    let expect = expect![[
        r#"OK (#1=("alpha" "beta" "zeta") 15 15 (("completion_matches (\"foo_bar\");\n")) #1# #1#)"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_do_complete_uses_empty_command_at_nonword_and_is_interactively_callable() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "foo ")
               (let ((inferior-octave-output-list
                      nil)
                     calls)
                 (cl-letf
                     (((symbol-function
                        'inferior-octave-send-list-and-digest)
                       (lambda (commands)
                         (push commands calls)
                         (setq
                          inferior-octave-output-list
                          nil))))
                   (list
                    (call-interactively
                     'ac-octave-do-complete)
                    (nreverse calls)
                    ac-octave-complete-list
                    (point)))))"##;
    let expect = expect![[r#"OK (nil (("completion_matches (\"\");\n")) nil 5)"#]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_candidate_invokes_completion_once_and_reverses_the_live_candidate_list() {
    let elisp_form = r##"(let ((ac-octave-complete-list
                    'stale)
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-octave-do-complete)
                     (lambda ()
                       (push
                        'complete
                        calls)
                       (setq
                        ac-octave-complete-list
                        '("alpha"
                          "beta"
                          "beta"
                          "gamma"))
                       'ignored-return)))
                 (list
                  (ac-octave-candidate)
                  (nreverse calls)
                  ac-octave-complete-list)))"##;
    let expect = expect![[
        r#"OK (("gamma" "beta" "beta" "alpha") (complete) ("alpha" "beta" "beta" "gamma"))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_documentation_sends_raw_symbol_and_joins_every_output_line() {
    let elisp_form = r##"(let ((inferior-octave-output-list
                    nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'inferior-octave-send-list-and-digest)
                     (lambda (commands)
                       (push commands calls)
                       (setq
                        inferior-octave-output-list
                        '("first line"
                          ""
                          "third line"))
                       'sent)))
                 (list
                  (ac-octave-documentation
                   "plot\"; injected")
                  (nreverse calls)
                  inferior-octave-output-list)))"##;
    let expect = expect![[
        r#"OK ("first line\n\nthird line" (("help plot\"; injected;\n")) ("first line" "" "third line"))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_documentation_swallows_send_errors_without_using_stale_output() {
    let elisp_form = r##"(let ((inferior-octave-output-list
                    '("stale"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'inferior-octave-send-list-and-digest)
                     (lambda (commands)
                       (push commands calls)
                       (error
                        "octave unavailable"))))
                 (list
                  (ac-octave-documentation
                   "helpme")
                  (nreverse calls)
                  inferior-octave-output-list)))"##;
    let expect = expect![[r#"OK (nil (("help helpme;\n")) ("stale"))"#]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_documentation_success_with_no_output_returns_the_empty_string() {
    let elisp_form = r##"(let ((inferior-octave-output-list
                    'stale)
                   calls)
               (cl-letf
                   (((symbol-function
                      'inferior-octave-send-list-and-digest)
                     (lambda (commands)
                       (push commands calls)
                       (setq
                        inferior-octave-output-list
                        nil)
                       'sent)))
                 (list
                  (ac-octave-documentation
                   "empty")
                  (nreverse calls)
                  inferior-octave-output-list)))"##;
    let expect = expect![[r#"OK ("" (("help empty;\n")) nil)"#]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_do_complete_send_error_preserves_point_and_previous_completion_state() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "alpha")
               (let ((inferior-octave-output-list
                      '("output"))
                     (ac-octave-complete-list
                      '("previous"))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'inferior-octave-send-list-and-digest)
                       (lambda (commands)
                         (push commands calls)
                         (error
                          "send failed"))))
                   (condition-case error-data
                       (ac-octave-do-complete)
                     (error
                      (list
                       error-data
                       (point)
                       (nreverse calls)
                       ac-octave-complete-list
                       inferior-octave-output-list))))))"##;
    let expect = expect![[
        r#"OK ((error "send failed") 6 (("completion_matches (\"alpha\");\n")) ("previous") ("output"))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_documentation_propagates_quit_and_preserves_output_state() {
    let elisp_form = r##"(let ((inferior-octave-output-list
                    '("stale"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'inferior-octave-send-list-and-digest)
                     (lambda (commands)
                       (push commands calls)
                       (signal
                        'quit
                        nil))))
                 (condition-case error-data
                     (list
                      'returned
                      (ac-octave-documentation
                       "interrupt")
                      (nreverse calls)
                      inferior-octave-output-list)
                   (quit
                    (list
                     'quit
                     error-data
                     (nreverse calls)
                     inferior-octave-output-list)))))"##;
    let expect = expect![[r#"OK (quit (quit) (("help interrupt;\n")) ("stale"))"#]];

    assert_ac_octave_parity(elisp_form, expect);
}
