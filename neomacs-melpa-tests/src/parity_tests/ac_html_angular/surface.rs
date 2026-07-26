use expect_test::expect;

use super::assert_ac_html_angular_parity;

#[test]
fn ac_html_angular_exact_pin_dependency_feature_and_source_directory_match() {
    let elisp_form = r##"(let* ((descriptor
                     (cadr
                      (assq
                       'ac-html-angular
                       package-alist)))
                    (library-directory
                     (file-name-directory
                      (locate-library
                       "ac-html-angular"))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-html-angular
                   web-completion-data))
                (file-relative-name
                 ac-html-angular-source-dir
                 library-directory)
                (file-directory-p
                 ac-html-angular-source-dir)
                web-completion-data-sources))"##;
    let expect = expect![[
        r#"OK (ac-html-angular "20151225.719" ((web-completion-data (0 1))) (t t) "html-stuff" t (("html" . web-completion-data-html-source-dir)))"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_alias_resolution_arity_interactivity_and_documentation_match() {
    let elisp_form = r##"(list
               (eq
                (symbol-function
                 'ac-html-angular+)
                (symbol-function
                 'company-web-angular+))
               (mapcar
                (lambda (function)
                  (list
                   function
                   (functionp function)
                   (help-function-arglist
                    function t)
                   (interactive-form function)
                   (documentation function t)))
                '(ac-html-angular+
                  company-web-angular+)))"##;
    let expect = expect![[
        r#"OK (nil ((ac-html-angular+ t nil (interactive nil) "Enable angular ac-html completion") (company-web-angular+ t nil (interactive nil) "Enable angular ac-html completion")))"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}

#[test]
fn ac_html_angular_registration_uses_a_symbolic_live_directory_reference() {
    let elisp_form = r##"(let ((web-completion-data-sources
                    nil))
               (ac-html-angular+)
               (let* ((entry
                       (assoc
                        "Angular15"
                        web-completion-data-sources))
                      (reference
                       (cdr entry)))
                 (list
                  entry
                  reference
                  (symbolp reference)
                  (boundp reference)
                  (eq
                   (symbol-value reference)
                   ac-html-angular-source-dir)
                  (file-directory-p
                   (symbol-value reference)))))"##;
    let expect = expect![[
        r#"OK (("Angular15" . ac-html-angular-source-dir) ac-html-angular-source-dir t t t t)"#
    ]];

    assert_ac_html_angular_parity(elisp_form, expect);
}
