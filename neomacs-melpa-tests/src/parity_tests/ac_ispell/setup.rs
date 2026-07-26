use expect_test::expect;

use super::assert_ac_ispell_parity;

#[test]
fn ac_ispell_ac_setup_adds_both_sources_once_and_enables_auto_complete() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                ac-sources
                '(fixture-source)
                auto-complete-mode
                nil)
               (let ((ac-ispell-fuzzy-limit
                      2)
                     events)
                 (cl-letf
                     (((symbol-function
                        'auto-complete-mode)
                       (lambda (argument)
                         (push argument events)
                         (setq
                          auto-complete-mode
                          (> argument 0))
                         'enabled)))
                   (let ((first
                          (ac-ispell-ac-setup))
                         (after-first
                          ac-sources))
                     (let ((second
                            (ac-ispell-ac-setup)))
                       (list
                        first
                        after-first
                        second
                        ac-sources
                        (eq
                         after-first
                         ac-sources)
                        auto-complete-mode
                        (local-variable-p
                         'ac-sources)
                        (nreverse events)))))))"##;
    let expect = expect![[
        r#"OK (enabled #1=(ac-source-ispell ac-source-ispell-fuzzy fixture-source) nil #1# t t t (1))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_ac_setup_skips_fuzzy_source_and_enabled_mode_when_limit_is_not_positive() {
    let elisp_form = r##"(mapcar
               (lambda (limit)
                 (with-temp-buffer
                   (setq
                    ac-sources
                    '(fixture-source)
                    auto-complete-mode
                    t)
                   (let ((ac-ispell-fuzzy-limit
                          limit)
                         calls)
                     (cl-letf
                         (((symbol-function
                            'auto-complete-mode)
                           (lambda (&rest arguments)
                             (push arguments calls)
                             (error
                              "mode already enabled"))))
                       (list
                        limit
                        (ac-ispell-ac-setup)
                        ac-sources
                        calls)))))
               '(0 -1))"##;
    let expect = expect![[
        r#"OK ((0 nil (ac-source-ispell . #1=(fixture-source)) nil) (-1 nil (ac-source-ispell . #1#) nil))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_ac_setup_enables_mode_without_adding_fuzzy_source_for_nonpositive_limit() {
    let elisp_form = r##"(mapcar
               (lambda (limit)
                 (with-temp-buffer
                   (setq
                    ac-sources
                    '(fixture-source)
                    auto-complete-mode
                    nil)
                   (let ((ac-ispell-fuzzy-limit
                          limit)
                         events)
                     (cl-letf
                         (((symbol-function
                            'auto-complete-mode)
                           (lambda (argument)
                             (push argument events)
                             (setq
                              auto-complete-mode
                              (> argument 0))
                             'enabled)))
                       (list
                        limit
                        (ac-ispell-ac-setup)
                        ac-sources
                        auto-complete-mode
                        (nreverse events))))))
               '(0 -1))"##;
    let expect = expect![[
        r#"OK ((0 enabled (ac-source-ispell . #1=(fixture-source)) t (1)) (-1 enabled (ac-source-ispell . #1#) t (1)))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_setup_defines_exact_sources_from_current_limits() {
    let elisp_form = r##"(let ((ac-ispell-requires
                    7)
                   (ac-ispell-fuzzy-limit
                    4))
               (list
                (ac-ispell-setup)
                ac-source-ispell
                ac-source-ispell-fuzzy
                (functionp
                 'ac-complete-ispell)
                (functionp
                 'ac-complete-ispell-fuzzy)
                (interactive-form
                 'ac-complete-ispell)
                (interactive-form
                 'ac-complete-ispell-fuzzy)))"##;
    let expect = expect![[
        r#"OK (ac-complete-ispell-fuzzy ((candidates . ac-ispell--candidates) (requires . 7) (symbol . "s")) ((candidates . ac-ispell--fuzzy-candidates) (match lambda (prefix candidates) candidates) (requires . 7) (limit . 4) (symbol . "s") (candidate-face . ac-ispell-fuzzy-candidate-face)) t t (interactive nil) (interactive nil))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_setup_rebuilds_sources_from_each_live_configuration() {
    let elisp_form = r##"(let ((ac-ispell-requires
                    3)
                   (ac-ispell-fuzzy-limit
                    2))
               (ac-ispell-setup)
               (let ((first-ispell
                      ac-source-ispell)
                     (first-fuzzy
                      ac-source-ispell-fuzzy))
                 (setq
                  ac-ispell-requires
                  9
                  ac-ispell-fuzzy-limit
                  0)
                 (ac-ispell-setup)
                 (list
                  first-ispell
                  first-fuzzy
                  ac-source-ispell
                  ac-source-ispell-fuzzy
                  (eq
                   first-ispell
                   ac-source-ispell)
                  (eq
                   first-fuzzy
                   ac-source-ispell-fuzzy))))"##;
    let expect = expect![[
        r#"OK ((#1=(candidates . ac-ispell--candidates) (requires . 3) . #2=((symbol . "s"))) (#3=(candidates . ac-ispell--fuzzy-candidates) #4=(match lambda (prefix candidates) candidates) (requires . 3) (limit . 2) . #5=((symbol . "s") (candidate-face . ac-ispell-fuzzy-candidate-face))) (#1# (requires . 9) . #2#) (#3# #4# (requires . 9) (limit . 0) . #5#) nil nil)"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_generated_completion_commands_call_auto_complete_with_exact_sources() {
    let elisp_form = r##"(let ((ac-ispell-requires
                    3)
                   (ac-ispell-fuzzy-limit
                    2)
                   events)
               (ac-ispell-setup)
               (cl-letf
                   (((symbol-function
                      'auto-complete)
                     (lambda (sources)
                       (push sources events)
                       (length events))))
                 (list
                  (call-interactively
                   'ac-complete-ispell)
                  (call-interactively
                   'ac-complete-ispell-fuzzy)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (1 2 ((ac-source-ispell) (ac-source-ispell-fuzzy)))"#]];

    assert_ac_ispell_parity(elisp_form, expect);
}
