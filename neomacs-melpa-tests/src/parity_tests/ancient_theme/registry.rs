use expect_test::expect;

use super::{assert_ancient_theme_autoload_parity, assert_ancient_theme_parity};

#[test]
fn ancient_theme_registers_exact_identity_feature_and_documentation() {
    let elisp_form = r##"(list
         (custom-theme-p 'ancient)
         (get 'ancient 'theme-feature)
         (get 'ancient 'theme-documentation)
         (featurep 'ancient-theme)
         (featurep 'ancient)
         (memq 'ancient custom-known-themes)
         (memq 'ancient custom-enabled-themes))"##;
    let expect = expect![[
        r#"OK (#1=(ancient user changed) ancient-theme "A theme about ruins." t nil #1# nil)"#
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_descriptor_records_exact_pin_dependencies_and_payload() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq
                            'ancient-theme
                            package-alist)))
               (directory
                (package-desc-dir description)))
         (list
          (package-desc-name description)
          (package-version-join
           (package-desc-version description))
          (package-desc-kind description)
          (package-desc-summary description)
          (package-desc-reqs description)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files
                    directory t
                    "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (ancient-theme "20260322.1856" nil "A theme about ruins." ((emacs (29 1))) ("README-elpa" "ancient-theme-autoloads.el" "ancient-theme-pkg.el" "ancient-theme.el" "ancient-theme.elc"))"#
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_autoload_registers_load_path_without_loading_theme() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-directory source)))
         (list
          (member directory
                  custom-theme-load-path)
          (custom-theme-p 'ancient)
          (featurep 'ancient-theme)
          (locate-file
           "ancient-theme.el"
           custom-theme-load-path)
          (cl-count directory
                    custom-theme-load-path
                    :test #'equal)))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ancient-theme/20260322.1856/home/.emacs.d/elpa/ancient-theme-20260322.1856/" custom-theme-directory t) nil nil "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ancient-theme/20260322.1856/home/.emacs.d/elpa/ancient-theme-20260322.1856/ancient-theme.el" 1)"#
    ]];
    assert_ancient_theme_autoload_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_source_reload_deduplicates_path_and_appends_settings_exactly() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-directory source))
               (before
                (secure-hash
                 'sha256
                 (prin1-to-string
                  (get 'ancient
                       'theme-settings)))))
         (load source nil t t)
         (load source nil t t)
         (list
          (cl-count directory
                    custom-theme-load-path
                    :test #'equal)
          (length
           (get 'ancient
                'theme-settings))
          before
          (secure-hash
           'sha256
           (prin1-to-string
            (get 'ancient
                 'theme-settings)))))"##;
    let expect = expect![[
        r#"OK (1 708 "12efc48e191a89413d085774abaceccf2b0d2e726bb9649c276569cebd200617" "535ccb5cf1eba90d826e448aee35a3cd65d401234a3b1672e6c9d521c769f92e")"#
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}
