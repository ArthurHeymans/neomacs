use expect_test::expect;

use super::assert_ac_html_bootstrap_parity;

#[test]
fn ac_html_bootstrap_activation_prepends_once_and_makes_sources_local() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                web-completion-data-sources
                '(("Fixture" . fixture-directory)))
               (let ((first
                      (ac-html-bootstrap+))
                     (after-first
                      web-completion-data-sources))
                 (let ((second
                        (ac-html-bootstrap+)))
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
        r#"OK (#1=(("Bootstrap" . ac-html-bootstrap-source-dir) ("Fixture" . fixture-directory)) #1# nil #1# t t)"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_fa_activation_prepends_once_and_makes_sources_local() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (with-temp-buffer
                 (setq
                  web-completion-data-sources
                  '(("Fixture" . fixture-directory)))
                 (let ((first
                        (ac-html-fa+))
                       (after-first
                        web-completion-data-sources))
                   (let ((second
                          (ac-html-fa+)))
                     (list
                      first
                      after-first
                      second
                      web-completion-data-sources
                      (eq
                       after-first
                       web-completion-data-sources)
                      (local-variable-p
                       'web-completion-data-sources))))))"##;
    let expect = expect![[
        r#"OK (#1=(("Font Aws" . ac-html-fa-source-dir) ("Fixture" . fixture-directory)) #1# nil #1# t t)"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_and_fa_compose_in_call_order_without_duplicates() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (list
                (with-temp-buffer
                  (setq
                   web-completion-data-sources
                   nil)
                  (ac-html-bootstrap+)
                  (ac-html-fa+)
                  (ac-html-bootstrap+)
                  (ac-html-fa+)
                  web-completion-data-sources)
                (with-temp-buffer
                  (setq
                   web-completion-data-sources
                   nil)
                  (ac-html-fa+)
                  (ac-html-bootstrap+)
                  web-completion-data-sources)))"##;
    let expect = expect![[
        r#"OK ((("Font Aws" . ac-html-fa-source-dir) ("Bootstrap" . ac-html-bootstrap-source-dir)) (("Bootstrap" . ac-html-bootstrap-source-dir) ("Font Aws" . ac-html-fa-source-dir)))"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_and_fa_keep_existing_values_and_positions() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (with-temp-buffer
                 (setq
                  web-completion-data-sources
                  '(("Before" . before-directory)
                    ("Bootstrap" . custom-bootstrap)
                    ("Middle" . middle-directory)
                    ("Font Aws" . custom-fa)
                    ("After" . after-directory)))
                 (let ((before
                        web-completion-data-sources))
                   (list
                    (ac-html-bootstrap+)
                    (ac-html-fa+)
                    web-completion-data-sources
                    (eq
                     before
                     web-completion-data-sources)
                    (cdr
                     (assoc
                      "Bootstrap"
                      web-completion-data-sources))
                    (cdr
                     (assoc
                      "Font Aws"
                      web-completion-data-sources))
                    (local-variable-p
                     'web-completion-data-sources)))))"##;
    let expect = expect![[
        r#"OK (nil nil (("Before" . before-directory) ("Bootstrap" . custom-bootstrap) ("Middle" . middle-directory) ("Font Aws" . custom-fa) ("After" . after-directory)) t custom-bootstrap custom-fa t)"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_and_fa_activation_is_buffer_local_and_preserves_default() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (let ((default-before
                      (default-value
                       'web-completion-data-sources))
                     (first
                      (get-buffer-create
                       " *ac-html-bootstrap-first*"))
                     (second
                      (get-buffer-create
                       " *ac-html-bootstrap-second*")))
                 (unwind-protect
                     (progn
                       (with-current-buffer first
                         (ac-html-bootstrap+))
                       (with-current-buffer second
                         (ac-html-fa+))
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
                   (kill-buffer second))))"##;
    let expect = expect![[
        r#"OK ((("Bootstrap" . ac-html-bootstrap-source-dir) . #1=(("html" . web-completion-data-html-source-dir))) (("Font Aws" . ac-html-fa-source-dir) . #1#) t #1#)"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_company_aliases_match_primary_activation_results() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (mapcar
                (lambda (function)
                  (with-temp-buffer
                    (setq
                     web-completion-data-sources
                     nil)
                    (list
                     function
                     (funcall function)
                     web-completion-data-sources)))
                '(ac-html-bootstrap+
                  company-web-bootstrap+
                  ac-html-fa+
                  company-web-fa+)))"##;
    let expect = expect![[
        r#"OK ((ac-html-bootstrap+ #1=(("Bootstrap" . ac-html-bootstrap-source-dir)) #1#) (company-web-bootstrap+ #2=(("Bootstrap" . ac-html-bootstrap-source-dir)) #2#) (ac-html-fa+ #3=(("Font Aws" . ac-html-fa-source-dir)) #3#) (company-web-fa+ #4=(("Font Aws" . ac-html-fa-source-dir)) #4#))"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}
