use expect_test::expect;

use super::assert_anakondo_autoload_parity;

#[test]
fn generated_autoload_exposes_only_the_documented_minor_mode_without_loading_feature() {
    let elisp_form = r##"(list
                      (featurep 'anakondo)
                      (mapcar
                       (lambda (symbol)
                         (let ((definition
                                (symbol-function symbol)))
                           (list
                            symbol
                            (fboundp symbol)
                            (autoloadp definition)
                            (and
                             (autoloadp definition)
                             (nth 1 definition))
                            (commandp symbol))))
                       '(anakondo-minor-mode
                         anakondo-refresh-project-cache
                         anakondo-completion-at-point))
                      (and
                       (member
                        (file-name-directory
                         (getenv "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![[
        r#"OK (nil ((anakondo-minor-mode t t "anakondo" t) (anakondo-refresh-project-cache nil nil nil nil) (anakondo-completion-at-point nil nil nil nil)) nil)"#
    ]];
    assert_anakondo_autoload_parity(elisp_form, expect);
}

#[test]
fn disabling_minor_mode_through_autoload_loads_package_and_leaves_buffer_clean() {
    let elisp_form = r##"(with-temp-buffer
                      (let ((before
                             (featurep 'anakondo)))
                        (anakondo-minor-mode -1)
                        (list
                         before
                         (featurep 'anakondo)
                         anakondo-minor-mode
                         (memq
                          #'anakondo-completion-at-point
                          completion-at-point-functions)
                         anakondo--completion-candidates-cache
                         anakondo--cache
                         (commandp 'anakondo-minor-mode)
                         (commandp
                          'anakondo-refresh-project-cache))))"##;
    let expect = expect!["OK (nil t nil nil nil nil t t)"];
    assert_anakondo_autoload_parity(elisp_form, expect);
}
