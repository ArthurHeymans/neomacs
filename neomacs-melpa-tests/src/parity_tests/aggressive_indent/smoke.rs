use expect_test::expect;

use super::assert_aggressive_indent_parity;

#[test]
fn aggressive_indent_loads_the_pinned_package_and_user_commands() {
    let elisp_form = r##"(let ((descriptor
                                (cadr
                                 (assq
                                  'aggressive-indent
                                  package-alist))))
                           (list
                            (package-version-join
                             (package-desc-version descriptor))
                            (package-desc-reqs descriptor)
                            (featurep
                             'aggressive-indent)
                            (file-name-base
                             (locate-library
                              "aggressive-indent"))
                            (mapcar
                             (lambda (command)
                               (list
                                command
                                (commandp
                                 command)))
                             '(aggressive-indent-mode
                               global-aggressive-indent-mode
                               aggressive-indent-indent-defun
                               aggressive-indent-indent-region-and-on))))"##;
    let expect = expect![[
        r#"OK ("20230112.1300" ((emacs (24 3))) t "aggressive-indent" ((aggressive-indent-mode t) (global-aggressive-indent-mode t) (aggressive-indent-indent-defun t) (aggressive-indent-indent-region-and-on t)))"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}
