use expect_test::expect;

use super::assert_ac_html_bootstrap_parity;

#[test]
fn ac_html_bootstrap_exact_pin_dependency_features_and_source_directories_match() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (let* ((descriptor
                       (cadr
                        (assq
                         'ac-html-bootstrap
                         package-alist)))
                      (library-directory
                       (file-name-directory
                        (locate-library
                         "ac-html-bootstrap"))))
                 (list
                  (package-desc-name descriptor)
                  (package-version-join
                   (package-desc-version descriptor))
                  (package-desc-reqs descriptor)
                  (mapcar
                   #'featurep
                   '(ac-html-bootstrap
                     ac-html-fa
                     web-completion-data))
                  (mapcar
                   (lambda (directory)
                     (list
                      (file-relative-name
                       directory
                       library-directory)
                      (file-directory-p
                       directory)))
                   (list
                    ac-html-bootstrap-source-dir
                    ac-html-fa-source-dir))
                  web-completion-data-sources)))"##;
    let expect = expect![[
        r#"OK (ac-html-bootstrap "20160302.1701" ((web-completion-data (0 1))) (t t t) (("html-stuff" t) ("fa-html-stuff" t)) (("html" . web-completion-data-html-source-dir)))"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_and_fa_alias_resolution_arity_interactivity_and_docs_match() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (mapcar
                (lambda (function)
                  (list
                   function
                   (functionp function)
                   (help-function-arglist
                    function t)
                   (interactive-form function)
                   (documentation function t)
                   (let ((definition
                          (symbol-function
                           function)))
                     (cond
                      ((symbolp definition)
                       definition)
                      ((byte-code-function-p
                        definition)
                       'byte-code)
                      (t 'interpreted)))))
                '(ac-html-bootstrap+
                  company-web-bootstrap+
                  ac-html-fa+
                  company-web-fa+)))"##;
    let expect = expect![[
        r#"OK ((ac-html-bootstrap+ t nil (interactive nil) "Enable bootstrap ac-html completion" interpreted) (company-web-bootstrap+ t nil (interactive nil) "Enable bootstrap ac-html completion" ac-html-bootstrap+) (ac-html-fa+ t nil (interactive nil) "Enable Font Awesome completion for `ac-html' or `company-web'" interpreted) (company-web-fa+ t nil (interactive nil) "Enable Font Awesome completion for `ac-html' or `company-web'" ac-html-fa+))"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}

#[test]
fn ac_html_bootstrap_and_fa_register_symbolic_live_directory_references() {
    let elisp_form = r##"(progn
               (require 'ac-html-fa)
               (let ((web-completion-data-sources
                      nil))
                 (ac-html-bootstrap+)
                 (ac-html-fa+)
                 (mapcar
                  (lambda (name)
                    (let* ((entry
                            (assoc
                             name
                             web-completion-data-sources))
                           (reference
                            (cdr entry)))
                      (list
                       entry
                       reference
                       (symbolp reference)
                       (boundp reference)
                       (file-directory-p
                        (symbol-value
                         reference)))))
                  '("Bootstrap"
                    "Font Aws"))))"##;
    let expect = expect![[
        r#"OK ((("Bootstrap" . ac-html-bootstrap-source-dir) ac-html-bootstrap-source-dir t t t) (("Font Aws" . ac-html-fa-source-dir) ac-html-fa-source-dir t t t))"#
    ]];

    assert_ac_html_bootstrap_parity(elisp_form, expect);
}
