use expect_test::expect;

use super::assert_android_mode_autoload_parity;

#[test]
fn generated_autoload_exposes_only_android_minor_mode_without_loading_feature() {
    let elisp_form = r##"(list
                      (featurep 'android-mode)
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
                       '(android-mode
                         android-start-emulator
                         android-logcat
                         android-build-install))
                      (and
                       (member
                        (file-name-directory
                         (getenv "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![[
        r#"OK (nil ((android-mode t t "android-mode" t) (android-start-emulator nil nil nil nil) (android-logcat nil nil nil nil) (android-build-install nil nil nil nil)) nil)"#
    ]];
    assert_android_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn enabling_mode_through_autoload_loads_package_runs_key_hook_and_stays_buffer_local() {
    let elisp_form = r##"(let ((first
                           (generate-new-buffer
                            " *android-autoload-one*"))
                          (second
                           (generate-new-buffer
                            " *android-autoload-two*")))
                      (unwind-protect
                          (let ((before
                                 (featurep
                                  'android-mode)))
                            (with-current-buffer first
                              (android-mode 1))
                            (list
                             before
                             (featurep 'android-mode)
                             (with-current-buffer first
                               (list
                                android-mode
                                (lookup-key
                                 android-mode-map
                                 (kbd
                                  (concat
                                   android-mode-key-prefix
                                   " e")))))
                             (with-current-buffer second
                               (and
                                (boundp 'android-mode)
                                android-mode))
                             (commandp
                              'android-start-emulator)))
                        (kill-buffer first)
                        (kill-buffer second)))"##;
    let expect = expect!["OK (nil t (t android-start-emulator) nil t)"];
    assert_android_mode_autoload_parity(elisp_form, expect);
}
