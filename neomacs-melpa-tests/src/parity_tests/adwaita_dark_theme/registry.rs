use expect_test::expect;

use super::{assert_adwaita_dark_theme_autoload_parity, assert_adwaita_dark_theme_parity};

#[test]
fn adwaita_dark_theme_registry_defaults_and_custom_metadata_match() {
    let elisp_form = r##"(list
         (featurep 'adwaita-dark-theme)
         (custom-theme-p 'adwaita-dark)
         (get 'adwaita-dark 'theme-documentation)
         (get 'adwaita-dark 'theme-feature)
         (length (get 'adwaita-dark 'theme-settings))
         (mapcar
          (lambda (symbol)
            (list symbol
                  (symbol-value symbol)
                  (get symbol 'custom-type)
                  (get symbol 'custom-group)))
          '(adwaita-dark-theme-pad-mode-line
            adwaita-dark-theme-pad-tab-line
            adwaita-dark-theme-pad-tab-bar
            adwaita-dark-theme-no-completions-first-difference
            adwaita-dark-theme-bold-vertico-current
            adwaita-dark-theme-gray-rainbow-delimiters
            adwaita-dark-theme-gray-outlines)))"##;
    let expect = expect![[
        r#"OK (t (adwaita-dark user changed) "A dark color scheme inspired by Adwaita." adwaita-dark-theme 520 ((adwaita-dark-theme-pad-mode-line nil boolean nil) (adwaita-dark-theme-pad-tab-line nil boolean nil) (adwaita-dark-theme-pad-tab-bar nil boolean nil) (adwaita-dark-theme-no-completions-first-difference nil boolean nil) (adwaita-dark-theme-bold-vertico-current nil boolean nil) (adwaita-dark-theme-gray-rainbow-delimiters nil boolean nil) (adwaita-dark-theme-gray-outlines nil boolean nil)))"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_callable_surface_arglists_and_command_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (subrp (symbol-function symbol))))
         '(adwaita-dark-theme--neotree-insert-root
           adwaita-dark-theme--neotree-insert-dir
           adwaita-dark-theme--neotree-insert-file
           adwaita-dark-theme-neotree-configuration-enable
           adwaita-dark-theme-eldoc-frame-configuration-enable
           adwaita-dark-theme-arrow-fringe-bmp-enable
           adwaita-dark-theme--diff-hl-fringe-bmp-function
           adwaita-dark-theme-diff-hl-fringe-bmp-enable
           adwaita-dark-theme-flycheck-fringe-bmp-enable
           adwaita-dark-theme-flymake-fringe-bmp-enable))"##;
    let expect = expect![
        "OK ((adwaita-dark-theme--neotree-insert-root (node) nil nil) (adwaita-dark-theme--neotree-insert-dir (node depth expanded) nil nil) (adwaita-dark-theme--neotree-insert-file (node depth) nil nil) (adwaita-dark-theme-neotree-configuration-enable nil nil nil) (adwaita-dark-theme-eldoc-frame-configuration-enable nil nil nil) (adwaita-dark-theme-arrow-fringe-bmp-enable nil nil nil) (adwaita-dark-theme--diff-hl-fringe-bmp-function (_type _pos) nil nil) (adwaita-dark-theme-diff-hl-fringe-bmp-enable nil nil nil) (adwaita-dark-theme-flycheck-fringe-bmp-enable nil nil nil) (adwaita-dark-theme-flymake-fringe-bmp-enable nil nil nil))"
    ];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_source_registers_its_real_package_directory_for_theme_loading() {
    let elisp_form = r##"(let* ((source (getenv "NEOMACS_PACKAGE_SOURCE"))
               (directory
                (file-name-as-directory
                 (file-name-directory source))))
         (list
          (member directory custom-theme-load-path)
          (car custom-theme-load-path)
          directory
          (file-readable-p
           (expand-file-name
            "adwaita-dark-theme.el"
            directory))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/adwaita-dark-theme/20231209.1033/home/.emacs.d/elpa/adwaita-dark-theme-20231209.1033/" custom-theme-directory t) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/adwaita-dark-theme/20231209.1033/home/.emacs.d/elpa/adwaita-dark-theme-20231209.1033/" "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/adwaita-dark-theme/20231209.1033/home/.emacs.d/elpa/adwaita-dark-theme-20231209.1033/" t)"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_autoloads_register_theme_path_and_all_setup_commands() {
    let elisp_form = r##"(let ((source (getenv "NEOMACS_PACKAGE_SOURCE")))
         (list
          (mapcar
           (lambda (symbol)
             (list symbol
                   (autoloadp (symbol-function symbol))
                   (nth 1 (symbol-function symbol))
                   (commandp symbol)))
           '(adwaita-dark-theme-neotree-configuration-enable
             adwaita-dark-theme-eldoc-frame-configuration-enable
             adwaita-dark-theme-arrow-fringe-bmp-enable
             adwaita-dark-theme-diff-hl-fringe-bmp-enable
             adwaita-dark-theme-flycheck-fringe-bmp-enable
             adwaita-dark-theme-flymake-fringe-bmp-enable))
          (mapcar
           (lambda (entry)
             (if (stringp entry)
                 (file-name-nondirectory
                  (directory-file-name entry))
               entry))
           custom-theme-load-path)
          (featurep 'adwaita-dark-theme)
          (file-name-nondirectory source)))"##;
    let expect = expect![[
        r#"OK (((adwaita-dark-theme-neotree-configuration-enable t "adwaita-dark-theme" nil) (adwaita-dark-theme-eldoc-frame-configuration-enable t "adwaita-dark-theme" nil) (adwaita-dark-theme-arrow-fringe-bmp-enable t "adwaita-dark-theme" nil) (adwaita-dark-theme-diff-hl-fringe-bmp-enable t "adwaita-dark-theme" nil) (adwaita-dark-theme-flycheck-fringe-bmp-enable t "adwaita-dark-theme" nil) (adwaita-dark-theme-flymake-fringe-bmp-enable t "adwaita-dark-theme" nil)) ("adwaita-dark-theme-20231209.1033" custom-theme-directory t) nil "adwaita-dark-theme-autoloads.el")"#
    ]];
    assert_adwaita_dark_theme_autoload_parity(elisp_form, expect);
}
