use expect_test::expect;

use super::{assert_ahungry_theme_autoload_parity, assert_ahungry_theme_parity};

#[test]
fn ahungry_theme_exact_package_metadata_headers_and_theme_registration_match() {
    let elisp_form = r##"(progn
         (require 'lisp-mnt)
         (let ((descriptor (cadr (assq 'ahungry-theme package-alist))))
           (list
            (package-desc-name descriptor)
            (package-version-join (package-desc-version descriptor))
            (package-desc-summary descriptor)
            (package-desc-kind descriptor)
            (package-desc-reqs descriptor)
            (package-desc-extras descriptor)
            (with-temp-buffer
              (insert-file-contents (getenv "NEOMACS_PACKAGE_SOURCE"))
              (list (lm-header "version")
                    (lm-header "keywords")
                    (lm-header "package-requires")))
            (custom-theme-p 'ahungry)
            (memq 'ahungry custom-known-themes)
            (featurep 'ahungry-theme)
            (featurep 'ahungry)
            (memq 'ahungry custom-enabled-themes))))"##;
    let expect = expect![[
        r#"OK (ahungry-theme "20180131.328" "Ahungry color theme for Emacs.  Make sure to (load-theme 'ahungry)." nil ((emacs (24))) ((:maintainers ("Matthew Carter" . "m@ahungry.com")) (:authors ("Matthew Carter" . "m@ahungry.com")) (:keywords "ahungry" "palette" "color" "theme" "emacs" "color-theme" "deftheme") (:revdesc . "a038d91ec593") (:commit . "a038d91ec593d1f1b19ca66a0576d59bbc24c523") (:url . "https://github.com/ahungry/color-theme-ahungry")) (nil "ahungry palette color theme emacs color-theme deftheme" "((emacs \"24\"))") #1=(ahungry user changed) #1# t nil nil)"#
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_complete_registered_face_inventory_matches() {
    let elisp_form = r##"(let (faces)
         (mapatoms
          (lambda (symbol)
            (when (assq 'ahungry (get symbol 'theme-face))
              (push symbol faces))))
         (sort faces
               (lambda (left right)
                 (string-lessp (symbol-name left)
                               (symbol-name right)))))"##;
    let expect = expect!["OK nil"];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_font_setting_variable_metadata_and_default_are_exact() {
    let elisp_form = r##"(list
         ahungry-theme-font-settings
         (default-value 'ahungry-theme-font-settings)
         (custom-variable-p 'ahungry-theme-font-settings)
         (get 'ahungry-theme-font-settings 'standard-value)
         (get 'ahungry-theme-font-settings 'variable-documentation)
         (local-variable-p 'ahungry-theme-font-settings))"##;
    let expect = expect![[
        r#"OK (#1=(:family "Terminus" :foundry "xos4" :slant normal :weight normal :height 130 :width normal) #1# nil nil "If set to nil, will avoid overriding the user font settings.\nLeave this alone to retain defaults.\n\nDefault value:\n   (:family \"Terminus\" :foundry \"xos4\"\n            :slant normal :weight normal\n            :height 100 :width normal)" nil)"#
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_installed_source_files_match_exact_sizes_and_hashes() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr (assq 'ahungry-theme package-alist)))
                 (directory (package-desc-dir descriptor)))
         (let ((source
                (expand-file-name "ahungry-theme.el" directory))
               (legacy
                (expand-file-name "color-theme-ahungry.el" directory)))
           (list
            (sort
             (seq-filter
              (lambda (file)
                (file-regular-p
                 (expand-file-name file directory)))
              (directory-files directory nil "\\`[^.]"))
             #'string-lessp)
            (file-attribute-size (file-attributes source))
            (secure-hash 'sha256 source)
            (file-exists-p legacy)
            (secure-hash 'sha256 legacy))))"##;
    let expect = expect![[
        r#"OK (("README-elpa" "ahungry-theme-autoloads.el" "ahungry-theme-pkg.el" "ahungry-theme.el" "ahungry-theme.elc") 18101 "57da1be078202d5b4741688f4489e69f30fb3b836649a90c9faf5e7a2acfbf1f" nil "3874512d57b9015b5d9ed4f93602a780f09ae014e823f4249be1dc02f8ba5d8a")"#
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_generated_autoload_only_registers_the_theme_directory() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr (assq 'ahungry-theme package-alist)))
                 (directory
                  (file-name-as-directory
                   (package-desc-dir descriptor))))
         (list
          (featurep 'ahungry-theme-autoloads)
          (custom-theme-p 'ahungry)
          (featurep 'ahungry-theme)
          (member directory custom-theme-load-path)
          (fboundp 'color-theme-ahungry)
          (boundp 'ahungry-theme-font-settings)))"##;
    let expect = expect![[
        r#"OK (t nil nil ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ahungry-theme/20180131.328/home/.emacs.d/elpa/ahungry-theme-20180131.328/" custom-theme-directory t) nil nil)"#
    ]];
    assert_ahungry_theme_autoload_parity(elisp_form, expect);
}
