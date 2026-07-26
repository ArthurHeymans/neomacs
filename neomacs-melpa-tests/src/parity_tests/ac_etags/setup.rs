use expect_test::expect;

use super::assert_ac_etags_parity;

#[test]
fn ac_etags_setup_defines_the_exact_source_and_captures_current_requires() {
    let elisp_form = r##"(let ((initially-bound
                    (boundp 'ac-source-etags))
                   (ac-etags-requires 5))
               (when initially-bound
                 (makunbound 'ac-source-etags))
               (list
                initially-bound
                (ac-etags-setup)
                (boundp 'ac-source-etags)
                ac-source-etags
                (interactive-form
                 #'ac-etags-setup)))"##;
    let expect = expect![[
        r#"OK (nil ac-complete-etags t ((candidates . ac-etags--candidates) (candidate-face . ac-etags-candidate-face) (selection-face . ac-etags-selection-face) (requires . 5) (symbol . "s")) (interactive nil))"#
    ]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_setup_rebuilds_source_from_each_live_requires_value() {
    let elisp_form = r##"(let ((original-requires
                    ac-etags-requires))
               (unwind-protect
                   (progn
                     (setq ac-etags-requires 1)
                     (ac-etags-setup)
                     (let ((first
                            (copy-tree
                             ac-source-etags)))
                       (setq ac-etags-requires 9)
                       (ac-etags-setup)
                       (list
                        first
                        ac-source-etags
                        (equal
                         first
                         ac-source-etags))))
                 (setq ac-etags-requires
                       original-requires)))"##;
    let expect = expect![[
        r#"OK (((candidates . ac-etags--candidates) (candidate-face . ac-etags-candidate-face) (selection-face . ac-etags-selection-face) (requires . 1) (symbol . "s")) ((candidates . ac-etags--candidates) (candidate-face . ac-etags-candidate-face) (selection-face . ac-etags-selection-face) (requires . 9) (symbol . "s")) nil)"#
    ]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_ac_setup_prepends_once_and_enables_a_disabled_mode_with_positive_arg() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing-source))
                   (auto-complete-mode nil)
                   calls)
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (&optional argument)
                       (push argument calls)
                       (setq auto-complete-mode t)
                       'enabled)))
                 (list
                  (ac-etags-ac-setup)
                  ac-sources
                  auto-complete-mode
                  (nreverse calls)
                  (ac-etags-ac-setup)
                  ac-sources
                  (nreverse calls)
                  (interactive-form
                   #'ac-etags-ac-setup))))"##;
    let expect = expect![
        "OK (enabled #1=(ac-source-etags existing-source) t #2=(1) nil #1# #2# (interactive nil))"
    ];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_ac_setup_keeps_existing_source_position_and_enabled_mode_untouched() {
    let elisp_form = r##"(let ((ac-sources
                    '(before
                      ac-source-etags
                      after))
                   (auto-complete-mode t)
                   calls)
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (&optional argument)
                       (push argument calls)
                       'unexpected)))
                 (list
                  (ac-etags-ac-setup)
                  ac-sources
                  calls
                  auto-complete-mode)))"##;
    let expect = expect!["OK (nil (before ac-source-etags after) nil t)"];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_ac_setup_changes_only_each_buffers_local_completion_state() {
    let elisp_form = r##"(let ((first
                    (get-buffer-create
                     " *ac-etags-first*"))
                   (second
                    (get-buffer-create
                     " *ac-etags-second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (setq-local
                        ac-sources
                        '(first-source))
                       (setq-local
                        auto-complete-mode t)
                       (ac-etags-ac-setup))
                     (with-current-buffer second
                       (setq-local
                        ac-sources
                        '(second-source))
                       (setq-local
                        auto-complete-mode t)
                       (ac-etags-ac-setup)
                       (ac-etags-ac-setup))
                     (list
                      (with-current-buffer first
                        (list
                         ac-sources
                         auto-complete-mode))
                      (with-current-buffer second
                        (list
                         ac-sources
                         auto-complete-mode))
                      (default-value
                       'ac-sources)))
                 (kill-buffer first)
                 (kill-buffer second)))"##;
    let expect = expect![
        "OK (((ac-source-etags first-source) t) ((ac-source-etags second-source) t) (ac-source-words-in-same-mode-buffers))"
    ];

    assert_ac_etags_parity(elisp_form, expect);
}
