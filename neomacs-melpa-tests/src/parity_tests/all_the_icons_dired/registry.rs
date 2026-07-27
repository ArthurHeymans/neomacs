use expect_test::expect;

use super::{assert_all_the_icons_dired_autoload_parity, assert_all_the_icons_dired_parity};

#[test]
fn all_the_icons_dired_registry_loads_exact_dependency_and_public_surface() {
    let elisp_form = r##"(list
         (featurep 'all-the-icons-dired)
         (featurep 'all-the-icons)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (help-function-arglist symbol t)
                  (commandp symbol)))
          '(all-the-icons-dired--icon
            all-the-icons-dired--put-icon
            all-the-icons-dired--fontify-region
            all-the-icons-dired--setup
            all-the-icons-dired--teardown
            all-the-icons-dired-mode))
         (mapcar
          (lambda (symbol)
            (list symbol
                  (default-value symbol)
                  (get symbol 'custom-type)
                  (get symbol 'custom-group)))
          '(all-the-icons-dired-lighter
            all-the-icons-dired-v-adjust
            all-the-icons-dired-monochrome)))"##;
    let expect = expect![[
        r#"OK (t t ((all-the-icons-dired--icon t (file) nil) (all-the-icons-dired--put-icon t (pos) nil) (all-the-icons-dired--fontify-region t (start end &optional loudly) nil) (all-the-icons-dired--setup t nil nil) (all-the-icons-dired--teardown t nil nil) (all-the-icons-dired-mode t (&optional arg) t)) ((all-the-icons-dired-lighter " all-the-icons-dired-mode" string nil) (all-the-icons-dired-v-adjust 0.01 number nil) (all-the-icons-dired-monochrome t boolean nil)))"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_face_inherits_dired_directory_contract() {
    let elisp_form = r##"(list
         (facep 'all-the-icons-dired-dir-face)
         (get 'all-the-icons-dired-dir-face
              'face-documentation)
         (get 'all-the-icons-dired-dir-face
              'face-defface-spec)
         (get 'all-the-icons-dired-dir-face
              'custom-group)
         (face-attribute
          'all-the-icons-dired-dir-face :inherit nil t))"##;
    let expect = expect![[
        r#"OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "Face for the directory icon." ((t (:inherit dired-directory))) nil dired-directory)"#
    ]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_autoload_publishes_mode_without_loading_runtime() {
    let elisp_form = r##"(list
         (featurep 'all-the-icons-dired)
         (fboundp 'all-the-icons-dired-mode)
         (autoloadp
          (and (fboundp 'all-the-icons-dired-mode)
               (symbol-function
                'all-the-icons-dired-mode)))
         (commandp 'all-the-icons-dired-mode)
         (help-function-arglist
          'all-the-icons-dired-mode t)
         (boundp 'all-the-icons-dired-mode)
         (file-name-nondirectory
          (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (nil t t t "[Arg list not available until function definition is loaded.]" nil "all-the-icons-dired-autoloads.el")"#
    ]];
    assert_all_the_icons_dired_autoload_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_dired_minor_mode_metadata_uses_custom_lighter_and_group() {
    let elisp_form = r##"(list
         (get 'all-the-icons-dired-mode 'custom-type)
         (get 'all-the-icons-dired-mode 'custom-group)
         (assq 'all-the-icons-dired-mode minor-mode-alist)
         (assq 'all-the-icons-dired-mode minor-mode-map-alist)
         (get 'all-the-icons-dired-mode
              'function-documentation))"##;
    let expect =
        expect![["OK (nil nil (all-the-icons-dired-mode all-the-icons-dired-lighter) nil nil)"]];
    assert_all_the_icons_dired_parity(elisp_form, expect);
}
