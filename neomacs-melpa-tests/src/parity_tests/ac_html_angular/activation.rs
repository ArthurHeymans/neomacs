use expect_test::expect;

use super::assert_ac_html_angular_parity;

#[test]
fn ac_html_angular_activation_prepends_once_and_makes_the_sources_local() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                web-completion-data-sources
                '(("Fixture" . fixture-directory)))
               (let ((first
                      (ac-html-angular+))
                     (after-first
                      web-completion-data-sources))
                 (let ((second
                        (ac-html-angular+)))
                   (list
                    first
                    after-first
                    second
                    web-completion-data-sources
                    (eq
                     after-first
                     web-completion-data-sources)
                    (local-variable-p
                     'web-completion-data-sources)))))"##;
    let expect = expect![[
        r#"OK (#1=(("Angular15" . ac-html-angular-source-dir) ("Fixture" . fixture-directory)) #1# nil #1# t t)"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_activation_keeps_an_existing_entry_value_and_position() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                web-completion-data-sources
                '(("Before" . before-directory)
                  ("Angular15" . custom-directory)
                  ("After" . after-directory)))
               (let ((before
                      web-completion-data-sources))
                 (list
                  (ac-html-angular+)
                  web-completion-data-sources
                  (eq
                   before
                   web-completion-data-sources)
                  (cdr
                   (assoc
                    "Angular15"
                    web-completion-data-sources))
                  (local-variable-p
                   'web-completion-data-sources))))"##;
    let expect = expect![[
        r#"OK (nil (("Before" . before-directory) ("Angular15" . custom-directory) ("After" . after-directory)) t custom-directory t)"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_activation_is_isolated_between_buffers_and_preserves_default() {
    let elisp_form = r##"(let ((default-before
                    (default-value
                     'web-completion-data-sources))
                   (first
                    (get-buffer-create
                     " *ac-html-angular-first*"))
                   (second
                    (get-buffer-create
                     " *ac-html-angular-second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (ac-html-angular+))
                     (with-current-buffer second
                       (setq-local
                        web-completion-data-sources
                        '(("Second" . second-directory)))
                       (ac-html-angular+))
                     (list
                      (with-current-buffer first
                        web-completion-data-sources)
                      (with-current-buffer second
                        web-completion-data-sources)
                      (eq
                       default-before
                       (default-value
                        'web-completion-data-sources))
                      (default-value
                       'web-completion-data-sources)))
                 (kill-buffer first)
                 (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK ((("Angular15" . ac-html-angular-source-dir) . #1=(("html" . web-completion-data-html-source-dir))) (("Angular15" . ac-html-angular-source-dir) ("Second" . second-directory)) t #1#)"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_company_alias_has_identical_activation_and_return_values() {
    let elisp_form = r##"(list
               (with-temp-buffer
                 (setq
                  web-completion-data-sources
                  nil)
                 (list
                  (ac-html-angular+)
                  web-completion-data-sources))
               (with-temp-buffer
                 (setq
                  web-completion-data-sources
                  nil)
                 (list
                  (company-web-angular+)
                  web-completion-data-sources)))"##;
    let expect = expect![[
        r#"OK ((#1=(("Angular15" . ac-html-angular-source-dir)) #1#) (#2=(("Angular15" . ac-html-angular-source-dir)) #2#))"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_activation_shadows_a_nonlocal_binding_without_mutating_it() {
    let elisp_form = r##"(let ((web-completion-data-sources
                    '(("Dynamic" . dynamic-directory))))
               (let ((inside
                      (with-temp-buffer
                        (list
                         (local-variable-p
                          'web-completion-data-sources)
                         (ac-html-angular+)
                         web-completion-data-sources
                         (local-variable-p
                          'web-completion-data-sources)))))
                 (list
                  inside
                  web-completion-data-sources)))"##;
    let expect = expect![[
        r#"OK ((nil #1=(("Angular15" . ac-html-angular-source-dir) . #2=(("Dynamic" . dynamic-directory))) #1# t) #2#)"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}
