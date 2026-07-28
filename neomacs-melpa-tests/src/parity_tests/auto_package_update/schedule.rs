use expect_test::expect;

use super::assert_auto_package_update_parity;

#[test]
fn auto_package_update_file_helpers_distinguish_missing_overwrite_and_unwritable_files() {
    let elisp_form = r##"(let*
                             ((root
                               (auto-package-update-test-root
                                "file-helpers"))
                              (file
                               (auto-package-update-test-path
                                root
                                "state/day"))
                              (missing
                               (auto-package-update-test-path
                                root
                                "missing")))
                           (auto-package-update-test-write
                            file
                            "old\n")
                           (let
                               ((before
                                 (list
                                  (apu--read-file-as-string
                                   file)
                                  (apu--read-file-as-string
                                   missing))))
                             (apu--write-string-to-file
                              file
                              "new-value")
                             (cl-letf
                                 (((symbol-function
                                    'file-writable-p)
                                   (lambda (_file) nil)))
                               (apu--write-string-to-file
                                file
                                "must-not-change"))
                             (list
                              before
                              (apu--read-file-as-string file)
                              (file-attribute-size
                               (file-attributes file))
                              (file-exists-p missing))))"##;
    let expect = expect![[r#"OK (("old\n" nil) "new-value" 9 nil)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_current_day_round_trips_through_configured_sandbox_file() {
    let elisp_form = r##"(let*
                             ((root
                               (auto-package-update-test-root
                                "current-day"))
                              (auto-package-update-last-update-day-path
                               (auto-package-update-test-path
                                root
                                "state/last-day")))
                           (make-directory
                            (file-name-directory
                             auto-package-update-last-update-day-path)
                            t)
                           (cl-letf
                               (((symbol-function
                                  'apu--today-day)
                                 (lambda () 24680)))
                             (let ((write-result
                                    (apu--write-current-day)))
                               (list
                                write-result
                                (apu--read-last-update-day)
                                (auto-package-update-test-read
                                 auto-package-update-last-update-day-path)
                                (file-exists-p
                                 auto-package-update-last-update-day-path)))))"##;
    let expect = expect![[r#"OK (nil 24680 "24680" t)"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_today_day_uses_current_time_and_time_to_days_contract() {
    let elisp_form = r##"(mapcar
                           (lambda (seconds)
                             (cl-letf
                                 (((symbol-function
                                    'current-time)
                                   (lambda ()
                                     (seconds-to-time
                                      seconds))))
                               (list
                                seconds
                                (apu--today-day)
                                (time-to-days
                                 (seconds-to-time
                                  seconds)))))
                           '(0 86399 86400 86401 172800 1234567890))"##;
    let expect = expect![
        "OK ((0 719163 719163) (86399 719163 719163) (86400 719164 719164) (86401 719164 719164) (172800 719165 719165) (1234567890 733451 733451))"
    ];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_due_decision_has_exact_interval_boundaries_and_short_circuiting() {
    let elisp_form = r##"(let
                             ((auto-package-update-interval 7)
                              (auto-package-update-last-update-day-path
                               "/fixture/last-day")
                              permission-calls)
                           (cl-letf
                               (((symbol-function
                                  'file-exists-p)
                                 (lambda (_file) t))
                                ((symbol-function
                                  'apu--today-day)
                                 (lambda () 100))
                                ((symbol-function
                                  'apu--get-permission-to-update-p)
                                 (lambda ()
                                   (setq
                                    permission-calls
                                    (1+ (or permission-calls 0)))
                                   t)))
                             (let
                                 ((existing
                                   (mapcar
                                    (lambda (last-day)
                                      (cl-letf
                                          (((symbol-function
                                             'apu--read-last-update-day)
                                            (lambda ()
                                              last-day)))
                                        (list
                                         last-day
                                         (apu--should-update-packages-p)
                                         permission-calls)))
                                    '(93 94 99 100 101 86))))
                               (cl-letf
                                   (((symbol-function
                                      'file-exists-p)
                                     (lambda (_file) nil))
                                    ((symbol-function
                                      'apu--read-last-update-day)
                                     (lambda ()
                                       (error
                                        "must not read missing state"))))
                                 (list
                                  existing
                                  (apu--should-update-packages-p)
                                  permission-calls)))))"##;
    let expect =
        expect!["OK (((93 t 1) (94 nil 1) (99 nil 1) (100 nil 1) (101 nil 1) (86 t 2)) t 3)"];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_zero_interval_only_signals_when_existing_state_requires_arithmetic() {
    let elisp_form = r##"(let
                             ((auto-package-update-interval 0)
                              (auto-package-update-last-update-day-path
                               "/fixture/last-day"))
                           (cl-letf
                               (((symbol-function
                                  'apu--today-day)
                                 (lambda () 100))
                                ((symbol-function
                                  'apu--read-last-update-day)
                                 (lambda () 90))
                                ((symbol-function
                                  'apu--get-permission-to-update-p)
                                 (lambda () :permitted)))
                             (list
                              (cl-letf
                                  (((symbol-function
                                     'file-exists-p)
                                    (lambda (_file) t)))
                                (auto-package-update-test-error
                                 #'apu--should-update-packages-p))
                              (cl-letf
                                  (((symbol-function
                                     'file-exists-p)
                                    (lambda (_file) nil)))
                                (apu--should-update-packages-p)))))"##;
    let expect = expect!["OK ((:signal arith-error nil) :permitted)"];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_permission_prompt_preview_and_hide_branches_are_exact() {
    let elisp_form = r##"(let
                             ((run-case
                               (lambda
                                   (prompt preview preview-result answer)
                                 (let
                                     ((auto-package-update-prompt-before-update
                                       prompt)
                                      (auto-package-update-show-preview
                                       preview)
                                      events)
                                   (cl-letf
                                       (((symbol-function
                                          'apu--show-preview)
                                         (lambda ()
                                           (push
                                            :preview
                                            events)
                                           preview-result))
                                        ((symbol-function
                                          'y-or-n-p)
                                         (lambda (question)
                                           (push
                                            (list
                                             :prompt
                                             question)
                                            events)
                                           answer))
                                        ((symbol-function
                                          'apu--hide-preview)
                                         (lambda ()
                                           (push
                                            :hide
                                            events))))
                                     (list
                                      (apu--get-permission-to-update-p)
                                      (nreverse events)))))))
                           (list
                            (funcall run-case nil nil nil nil)
                            (funcall run-case t nil nil t)
                            (funcall run-case t t nil nil)
                            (funcall run-case t t t t)))"##;
    let expect = expect![[
        r#"OK ((t nil) (t ((:prompt "Auto-update packages now?") :hide)) (nil (:preview (:prompt "Auto-update packages now?") :hide)) (nil (:preview)))"#
    ]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_daily_timer_uses_exact_period_callback_and_return_value() {
    let elisp_form = r##"(let (calls)
                           (cl-letf
                               (((symbol-function
                                  'run-at-time)
                                 (lambda
                                     (time repeat function
                                           &rest arguments)
                                   (push
                                    (list
                                     time
                                     repeat
                                     function
                                     arguments)
                                    calls)
                                   'fixture-timer)))
                             (list
                              (auto-package-update-at-time
                               "03:15")
                              (nreverse calls))))"##;
    let expect = expect![[r#"OK (fixture-timer (("03:15" 86400 auto-package-update-maybe nil)))"#]];

    assert_auto_package_update_parity(elisp_form, expect);
}

#[test]
fn auto_package_update_maybe_runs_now_only_when_due_and_returns_underlying_value() {
    let elisp_form = r##"(let
                             ((run-case
                               (lambda (due)
                                 (let (calls)
                                   (cl-letf
                                       (((symbol-function
                                          'apu--should-update-packages-p)
                                         (lambda ()
                                           (push :checked calls)
                                           due))
                                        ((symbol-function
                                          'auto-package-update-now)
                                         (lambda ()
                                           (push :updated calls)
                                           :update-result)))
                                     (list
                                      (auto-package-update-maybe)
                                      (nreverse calls)))))))
                           (list
                            (funcall run-case nil)
                            (funcall run-case t)))"##;
    let expect = expect!["OK ((nil (:checked)) (:update-result (:checked :updated)))"];

    assert_auto_package_update_parity(elisp_form, expect);
}
