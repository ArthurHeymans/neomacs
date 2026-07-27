use expect_test::expect;

use super::{assert_afternoon_theme_autoload_parity, assert_afternoon_theme_parity};

#[test]
fn afternoon_theme_registry_metadata_and_complete_setting_kinds_match() {
    let elisp_form = r##"(let ((settings (get 'afternoon 'theme-settings)))
         (list
          (featurep 'afternoon-theme)
          (custom-theme-p 'afternoon)
          (get 'afternoon 'theme-documentation)
          (get 'afternoon 'theme-feature)
          (length settings)
          (mapcar
           (lambda (kind)
             (cons
              kind
              (seq-count
               (lambda (setting)
                 (eq (car setting) kind))
               settings)))
           '(theme-face theme-value))
          (memq 'afternoon custom-known-themes)
          (memq 'afternoon custom-enabled-themes)))"##;
    let expect = expect![[
        r#"OK (t #1=(afternoon user changed) "Dark color theme with a deep blue background" afternoon-theme 417 ((theme-face . 411) (theme-value . 6)) #1# nil)"#
    ]];
    assert_afternoon_theme_parity(elisp_form, expect);
}

#[test]
fn afternoon_theme_source_registers_the_installed_directory_for_real_theme_loading() {
    let elisp_form = r##"(let* ((source (getenv "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-as-directory
                 (file-name-directory source))))
         (list
          (member directory custom-theme-load-path)
          (car custom-theme-load-path)
          directory
          (file-readable-p
           (expand-file-name "afternoon-theme.el" directory))
          (file-readable-p
           (expand-file-name "afternoon-theme-autoloads.el" directory))
          (locate-file
           "afternoon-theme.el"
           custom-theme-load-path)))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/afternoon-theme/20140104.1859/home/.emacs.d/elpa/afternoon-theme-20140104.1859/" custom-theme-directory t) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/afternoon-theme/20140104.1859/home/.emacs.d/elpa/afternoon-theme-20140104.1859/" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/afternoon-theme/20140104.1859/home/.emacs.d/elpa/afternoon-theme-20140104.1859/" t t "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/afternoon-theme/20140104.1859/home/.emacs.d/elpa/afternoon-theme-20140104.1859/afternoon-theme.el")"#
    ]];
    assert_afternoon_theme_parity(elisp_form, expect);
}

#[test]
fn afternoon_theme_autoloads_register_the_theme_without_eagerly_loading_it() {
    let elisp_form = r##"(let* ((source (getenv "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-as-directory
                 (file-name-directory source))))
         (list
          (featurep 'afternoon-theme)
          (custom-theme-p 'afternoon)
          (member directory custom-theme-load-path)
          (car custom-theme-load-path)
          (file-readable-p
           (expand-file-name "afternoon-theme.el" directory))
          (file-name-nondirectory source)))"##;
    let expect = expect![[
        r#"OK (nil nil ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/afternoon-theme/20140104.1859/home/.emacs.d/elpa/afternoon-theme-20140104.1859/" custom-theme-directory t) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/afternoon-theme/20140104.1859/home/.emacs.d/elpa/afternoon-theme-20140104.1859/" t "afternoon-theme-autoloads.el")"#
    ]];
    assert_afternoon_theme_autoload_parity(elisp_form, expect);
}
