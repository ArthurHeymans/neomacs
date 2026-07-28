use expect_test::expect;

use super::assert_aggressive_fill_paragraph_parity;

#[test]
fn aggressive_fill_paragraph_loads_the_pinned_package_with_real_dash_support() {
    let elisp_form = r##"(let ((descriptor
                                (cadr
                                 (assq
                                  'aggressive-fill-paragraph
                                  package-alist)))
                               (dash-descriptor
                                (cadr
                                 (assq
                                  'dash
                                  package-alist))))
                           (list
                            (package-version-join
                             (package-desc-version descriptor))
                            (package-desc-reqs descriptor)
                            (featurep
                             'aggressive-fill-paragraph)
                            (featurep
                             'dash)
                            (and
                             dash-descriptor
                             (version-list-<=
                              '(2 10 0)
                              (package-desc-version
                               dash-descriptor)))
                            (file-name-base
                             (symbol-file
                              '-any?
                              'defun))
                            (commandp
                             'aggressive-fill-paragraph-mode)))"##;
    let expect = expect![[r#"OK ("20240213.2320" ((dash (2 10 0))) t t t "dash" t)"#]];

    assert_aggressive_fill_paragraph_parity(elisp_form, expect);
}
