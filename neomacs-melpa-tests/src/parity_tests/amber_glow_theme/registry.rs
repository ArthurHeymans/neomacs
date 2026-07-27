use expect_test::expect;

use super::{assert_amber_glow_theme_autoload_parity, assert_amber_glow_theme_parity};

#[test]
fn amber_glow_theme_registers_exact_theme_identity_documentation_and_feature() {
    let elisp_form = r##"(list
         (custom-theme-p 'amber-glow)
         (get 'amber-glow 'theme-documentation)
         (featurep 'amber-glow-theme)
         (memq 'amber-glow custom-known-themes)
         (custom-theme-enabled-p 'amber-glow)
         (file-name-nondirectory
          (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (#1=(amber-glow user changed) "A warm and inviting amber-themed Emacs theme." t #1# nil "amber-glow-theme.el")"#
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_package_descriptor_records_pin_summary_and_requirement() {
    let elisp_form = r##"(let ((description
                        (cadr
                         (assq
                          'amber-glow-theme
                          package-alist))))
         (list
          (package-version-join
           (package-desc-version description))
          (package-desc-reqs description)
          (package-desc-summary description)
          (package-desc-kind description)
          (file-name-nondirectory
           (directory-file-name
            (package-desc-dir description)))))"##;
    let expect = expect![[
        r#"OK ("20250305.936" ((emacs (24 1))) "A warm and inviting theme." nil "amber-glow-theme-20250305.936")"#
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_source_adds_its_installed_directory_to_theme_load_path() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-as-directory
                 (file-name-directory source))))
         (list
          directory
          (car custom-theme-load-path)
          (member directory
                  custom-theme-load-path)
          (cl-count directory
                    custom-theme-load-path
                    :test #'equal)))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amber-glow-theme/20250305.936/home/.emacs.d/elpa/amber-glow-theme-20250305.936/" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amber-glow-theme/20250305.936/home/.emacs.d/elpa/amber-glow-theme-20250305.936/" ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amber-glow-theme/20250305.936/home/.emacs.d/elpa/amber-glow-theme-20250305.936/" custom-theme-directory t) 1)"#
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_autoload_adds_directory_without_defining_theme() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-as-directory
                 (file-name-directory source))))
         (list
          (custom-theme-p 'amber-glow)
          (featurep 'amber-glow-theme)
          (member directory
                  custom-theme-load-path)
          (cl-count directory
                    custom-theme-load-path
                    :test #'equal)))"##;
    let expect = expect![[
        r#"OK (nil nil ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amber-glow-theme/20250305.936/home/.emacs.d/elpa/amber-glow-theme-20250305.936/" custom-theme-directory t) 1)"#
    ]];
    assert_amber_glow_theme_autoload_parity(elisp_form, expect);
}
