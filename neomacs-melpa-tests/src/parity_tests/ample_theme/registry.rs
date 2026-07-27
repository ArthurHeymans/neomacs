use expect_test::expect;

use super::{
    assert_ample_flat_theme_parity, assert_ample_light_theme_parity,
    assert_ample_theme_autoload_parity, assert_ample_theme_parity,
};

#[test]
fn ample_theme_registers_dark_identity_feature_and_interactive_loader() {
    let elisp_form = r##"(list
         (custom-theme-p 'ample)
         (get 'ample 'theme-documentation)
         (featurep 'ample-theme)
         (fboundp 'ample-theme)
         (help-function-arglist 'ample-theme t)
         (commandp 'ample-theme)
         (custom-theme-enabled-p 'ample)
         (memq 'ample custom-known-themes))"##;
    let expect =
        expect![[r#"OK (#1=(ample user changed) "A smooth dark theme." t t nil t nil #1#)"#]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_registers_flat_identity_feature_and_interactive_loader() {
    let elisp_form = r##"(list
         (custom-theme-p 'ample-flat)
         (get 'ample-flat 'theme-documentation)
         (featurep 'ample-flat-theme)
         (fboundp 'ample-flat-theme)
         (help-function-arglist 'ample-flat-theme t)
         (commandp 'ample-flat-theme)
         (custom-theme-enabled-p 'ample-flat)
         (memq 'ample-flat custom-known-themes))"##;
    let expect =
        expect![[r#"OK (#1=(ample-flat user changed) "A flat, dark theme." t t nil t nil #1#)"#]];
    assert_ample_flat_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_registers_light_identity_feature_and_interactive_loader() {
    let elisp_form = r##"(list
         (custom-theme-p 'ample-light)
         (get 'ample-light 'theme-documentation)
         (featurep 'ample-light-theme)
         (fboundp 'ample-light-theme)
         (help-function-arglist 'ample-light-theme t)
         (commandp 'ample-light-theme)
         (custom-theme-enabled-p 'ample-light)
         (memq 'ample-light custom-known-themes))"##;
    let expect = expect![[
        r#"OK (#1=(ample-light user changed) "A smooth light theme to pair with ample-dark." t t nil t nil #1#)"#
    ]];
    assert_ample_light_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_package_descriptor_records_pin_kind_summary_and_payload_files() {
    let elisp_form = r##"(let* ((description
                          (cadr
                           (assq 'ample-theme
                                 package-alist)))
               (directory
                (package-desc-dir description)))
         (list
          (package-version-join
           (package-desc-version description))
          (package-desc-reqs description)
          (package-desc-summary description)
          (package-desc-kind description)
          (sort
           (mapcar
            #'file-name-nondirectory
            (directory-files
             directory t
             "\\.el\\'"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20260611.1532" nil "Calm Dark Theme for Emacs." nil ("ample-flat-theme.el" "ample-light-theme.el" "ample-theme-autoloads.el" "ample-theme-pkg.el" "ample-theme.el"))"#
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_autoloads_publish_all_three_loaders_without_defining_themes() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (autoloadp
              (and (fboundp symbol)
                   (symbol-function symbol)))
             (commandp symbol)
             (help-function-arglist symbol t)))
          '(ample-theme
            ample-flat-theme
            ample-light-theme))
         (mapcar
          #'custom-theme-p
          '(ample ample-flat ample-light))
         (mapcar
          #'featurep
          '(ample-theme
            ample-flat-theme
            ample-light-theme)))"##;
    let expect = expect![[
        r#"OK (((ample-theme t t t "[Arg list not available until function definition is loaded.]") (ample-flat-theme t t t "[Arg list not available until function definition is loaded.]") (ample-light-theme t t t "[Arg list not available until function definition is loaded.]")) (nil nil nil) (nil nil nil))"#
    ]];
    assert_ample_theme_autoload_parity(elisp_form, expect);
}

#[test]
fn ample_theme_each_source_deduplicates_the_installed_theme_load_path() {
    let elisp_form = r##"(let* ((source
                          (getenv
                           "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-as-directory
                 (file-name-directory source))))
         (load source nil t t)
         (load source nil t t)
         (list
          (car custom-theme-load-path)
          (member directory
                  custom-theme-load-path)
          (cl-count directory
                    custom-theme-load-path
                    :test #'equal)))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ample-theme/20260611.1532/home/.emacs.d/elpa/ample-theme-20260611.1532/" ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ample-theme/20260611.1532/home/.emacs.d/elpa/ample-theme-20260611.1532/" custom-theme-directory t) 1)"#
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}
