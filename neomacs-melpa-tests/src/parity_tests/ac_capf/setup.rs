use expect_test::expect;

use super::assert_ac_capf_parity;

#[test]
fn ac_capf_exact_pin_public_surface_and_auto_complete_source_contract_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-capf package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (featurep 'auto-complete)
                (featurep 'cl-lib)
                (featurep 'ac-capf)
                (mapcar
                 #'fboundp
                 '(ac-capf--candidates-response
                   ac-capf--candidates
                   ac-capf-setup))
                (mapcar
                 #'commandp
                 '(ac-capf--candidates-response
                   ac-capf--candidates
                   ac-capf-setup))
                (interactive-form
                 'ac-capf-setup)
                ac-source-capf))"##;
    let expect = expect![[
        r#"OK (ac-capf "20151101.217" ((auto-complete (1 4)) (cl-lib (0 5))) t t t (t t t) (nil nil t) (interactive nil) ((candidates . ac-capf--candidates) (requires . 0) (symbol . "s")))"#
    ]];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_setup_prepends_the_source_and_enables_auto_complete_when_disabled() {
    let elisp_form = r##"(let ((ac-sources
                    '(existing ac-source-capf-tail))
                   (auto-complete-mode nil)
                   events)
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (argument)
                       (push argument events)
                       (setq auto-complete-mode
                             (> argument 0))
                       'enabled)))
                 (list
                  (ac-capf-setup)
                  ac-sources
                  auto-complete-mode
                  (nreverse events))))"##;
    let expect = expect!["OK (enabled (ac-source-capf existing ac-source-capf-tail) t (1))"];

    assert_ac_capf_parity(elisp_form, expect);
}

#[test]
fn ac_capf_setup_is_idempotent_and_does_not_reenable_an_active_mode() {
    let elisp_form = r##"(let ((ac-sources
                    '(ac-source-capf existing
                      ac-source-capf))
                   (auto-complete-mode t)
                   events)
               (cl-letf
                   (((symbol-function
                      'auto-complete-mode)
                     (lambda (argument)
                       (push argument events)
                       'unexpected)))
                 (list
                  (ac-capf-setup)
                  (ac-capf-setup)
                  ac-sources
                  auto-complete-mode
                  events)))"##;
    let expect = expect!["OK (nil nil (ac-source-capf existing ac-source-capf) t nil)"];

    assert_ac_capf_parity(elisp_form, expect);
}
