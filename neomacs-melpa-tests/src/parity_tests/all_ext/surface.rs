use expect_test::expect;

use super::assert_all_ext_parity;

#[test]
fn all_ext_exact_pin_dependency_feature_default_and_source_build_match() {
    let elisp_form = r##"(let ((descriptor
                           (cadr (assq 'all-ext package-alist)))
                          (all-descriptor
                           (cadr (assq 'all package-alist))))
                      (list
                       (package-desc-name descriptor)
                       (package-version-join
                        (package-desc-version descriptor))
                       (package-desc-reqs descriptor)
                       (featurep 'all-ext)
                       (featurep 'all)
                       (package-version-join
                        (package-desc-version all-descriptor))
                       (with-temp-buffer
                         (insert-file-contents-literally
                          (expand-file-name
                           "all.el"
                           (package-desc-dir all-descriptor)))
                         (secure-hash
                          'sha256 (current-buffer)))
                       all-from-occur-select-window-flag
                       (file-name-nondirectory
                        (getenv "NEOMACS_PACKAGE_SOURCE"))))"##;
    let expect = expect![[
        r#"OK (all-ext "20200315.1443" ((emacs (24 4)) (all (1 0))) t t "1.0" "0d12a0c8d1098903a3625cb6b01884c5a1a63d163226d2a6e72d4dfd8f18b8c7" t "all-ext.el")"#
    ]];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_complete_callable_surface_arglists_and_commands_match() {
    let elisp_form = r##"(let* ((source-directory
                               (file-name-directory
                                (file-truename
                                 (getenv "NEOMACS_PACKAGE_SOURCE"))))
                              rows)
                      (mapatoms
                       (lambda (symbol)
                         (when (and
                                (or
                                 (string-prefix-p
                                  "all-from-" (symbol-name symbol))
                                 (eq symbol 'all-next-error)
                                 (eq symbol 'mc/edit-lines-in-all))
                                (fboundp symbol)
                                (when-let
                                    ((file
                                      (symbol-file symbol 'defun)))
                                  (string=
                                   source-directory
                                   (file-name-directory
                                    (file-truename file)))))
                           (push
                            (list
                             symbol
                             (help-function-arglist symbol t)
                             (commandp symbol))
                            rows))))
                      (sort
                       rows
                       (lambda (left right)
                         (string-lessp
                          (symbol-name (car left))
                          (symbol-name (car right))))))"##;
    let expect = expect![
        "OK ((all-from-anything-occur nil t) (all-from-anything-occur-insert (start end lineno content match-beg) nil) (all-from-anything-occur-internal (from anybuf srcbuf) nil) (all-from-helm-occur nil t) (all-next-error (&optional argp reset) nil) (mc/edit-lines-in-all nil t))"
    ];
    assert_all_ext_parity(elisp_form, expect);
}

#[test]
fn all_ext_load_installs_all_mode_advice_and_multiple_cursor_key_contract() {
    let elisp_form = r##"(let ((advice
                           (advice-member-p
                            (lambda (&rest ignore)
                              (setq
                               next-error-function
                               'all-next-error))
                            'all-mode)))
                      (with-temp-buffer
                        (all-mode)
                        (list
                         (key-binding (kbd "C-c C-m"))
                         next-error-function
                         (eq major-mode 'all-mode)
                         (local-variable-p 'next-error-function)
                         (not (null advice)))))"##;
    let expect = expect!["OK (mc/edit-lines-in-all all-next-error t t nil)"];
    assert_all_ext_parity(elisp_form, expect);
}
