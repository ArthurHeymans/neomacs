use expect_test::expect;

use super::assert_ac_js2_parity;

#[test]
fn ac_js2_mode_without_auto_complete_installs_completion_save_and_skewer_hooks() {
    let elisp_form = r##"(let ((features
                    (delq
                     'auto-complete
                     (copy-sequence
                      features)))
                   (skewer-js-hook
                    '(existing-skewer-hook))
                   events)
               (cl-letf
                   (((symbol-function
                      'ac-js2-skewer-eval-wrapper)
                     (lambda (string
                              &optional extras)
                       (push
                        (list
                         string extras)
                        events)
                       'evaluated)))
                 (with-temp-buffer
                   (insert
                    "var fixture = 1;")
                   (setq
                    completion-at-point-functions
                    '(existing-completion)
                    before-save-hook
                    '(existing-save-hook))
                   (let ((first
                          (ac-js2-mode 1)))
                     (let ((after-enable
                            (list
                             first
                             ac-js2-mode
                             completion-at-point-functions
                             before-save-hook
                             skewer-js-hook
                             (local-variable-p
                              'completion-at-point-functions)
                             (local-variable-p
                              'before-save-hook))))
                       (let ((second
                              (ac-js2-mode -1)))
                         (list
                          after-enable
                          second
                          ac-js2-mode
                          completion-at-point-functions
                          before-save-hook
                          skewer-js-hook
                          (nreverse events))))))))"##;
    let expect = expect![[
        r#"OK ((t t #1=(ac-js2-completion-function existing-completion) #2=(ac-js2-save t) #3=(ac-js2-on-skewer-load existing-skewer-hook) t t) nil nil (ac-js2-completion-function . #1#) #2# #3# (("var fixture = 1;" nil) ("var fixture = 1;" nil)))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_mode_calls_auto_complete_setup_on_both_enable_and_disable_when_feature_exists() {
    let elisp_form = r##"(let ((already-auto-complete
                    (featurep
                     'auto-complete))
                   events)
               (provide
                'auto-complete)
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'ac-js2-setup-auto-complete-mode)
                         (lambda ()
                           (push '(setup) events)
                           'setup-done))
                        ((symbol-function
                          'ac-js2-skewer-eval-wrapper)
                         (lambda (string
                                  &optional extras)
                           (push
                            (list
                             'eval string extras)
                            events)
                           'evaluated)))
                     (with-temp-buffer
                       (insert
                        "fixture")
                       (list
                        (ac-js2-mode 1)
                        ac-js2-mode
                        (ac-js2-mode -1)
                        ac-js2-mode
                        (nreverse events))))
                 (unless already-auto-complete
                   (setq
                    features
                    (delq
                     'auto-complete
                     features)))))"##;
    let expect =
        expect![[r#"OK (t t nil nil (#1=(setup) (eval "fixture" nil) #1# (eval "fixture" nil)))"#]];

    assert_ac_js2_parity(elisp_form, expect);
}
