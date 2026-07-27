use expect_test::expect;

use super::{assert_amaranth_dark_theme_autoload_parity, assert_amaranth_dark_theme_parity};

#[test]
fn pinned_package_metadata_source_headers_and_installed_payload_match_the_frozen_archive() {
    let elisp_form = r##"(let* ((description
                     (cadr
                      (assq
                       'amaranth-dark-theme
                       package-alist)))
                    (directory
                     (and description
                          (package-desc-dir description))))
               (list
                (featurep 'amaranth-dark-theme)
                (custom-theme-p 'amaranth-dark)
                (memq 'amaranth-dark custom-known-themes)
                (and description
                     (package-version-join
                      (package-desc-version description)))
                (and description
                     (package-desc-archive description))
                (and description
                     (package-desc-summary description))
                (and directory
                     (sort
                      (mapcar
                       #'file-name-nondirectory
                       (directory-files
                        directory
                        t
                        "\\`amaranth-dark-theme.*\\.elc?\\'"))
                      #'string<))
                (with-temp-buffer
                  (insert-file-contents
                   (getenv "NEOMACS_PACKAGE_SOURCE"))
                  (list
                   (re-search-forward
                    "^;; Package-Version: 20251228\\.1916$"
                    nil t)
                   (re-search-forward
                    "^;; Package-Revision: 624e0b5ef632$"
                    nil t)
                   (re-search-forward
                    "^;; Package-Requires: ((emacs \"24\\.1\"))$"
                    nil t)))))"##;
    let expect = expect![[
        r#"OK (t #1=(amaranth-dark user changed) #1# "20251228.1916" nil "Amaranth Dark theme." ("amaranth-dark-theme-autoloads.el" "amaranth-dark-theme-pkg.el" "amaranth-dark-theme.el" "amaranth-dark-theme.elc") (277 311 349))"#
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn theme_registry_contains_one_variable_and_every_face_once_without_duplicate_settings() {
    let elisp_form = r##"(let* ((settings
                     (get 'amaranth-dark 'theme-settings))
                    (faces
                     (mapcar
                      #'cadr
                      (seq-filter
                       (lambda (setting)
                         (eq (car setting) 'theme-face))
                       settings)))
                    (variables
                     (seq-filter
                      (lambda (setting)
                        (eq (car setting) 'theme-value))
                      settings))
                    (duplicates nil))
               (dolist (face faces)
                 (when
                     (> (cl-count face faces :test #'eq) 1)
                   (cl-pushnew face duplicates)))
               (list
                (length settings)
                (length faces)
                (length
                 (delete-dups (copy-sequence faces)))
                (sort duplicates
                      (lambda (left right)
                        (string<
                         (symbol-name left)
                         (symbol-name right))))
                variables
                (car faces)
                (car (last faces))))"##;
    let expect = expect![
        "OK (129 128 128 nil ((theme-value frame-background-mode amaranth-dark 'dark)) orderless-match-face-3 border)"
    ];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn loaded_theme_source_adds_its_installed_directory_once_to_custom_theme_load_path() {
    let elisp_form = r##"(let* ((source
                     (file-truename
                      (getenv "NEOMACS_PACKAGE_SOURCE")))
                    (directory
                     (file-name-as-directory
                      (file-name-directory source)))
                    (matches
                     (lambda ()
                       (cl-count-if
                        (lambda (entry)
                          (and
                           (stringp entry)
                           (equal
                            (file-truename entry)
                            directory)))
                        custom-theme-load-path))))
               (list
                directory
                (funcall matches)
                (progn
                  (load source nil t t)
                  (funcall matches))
                (progn
                  (load source nil t t)
                  (funcall matches))
                (member directory custom-theme-load-path)))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amaranth-dark-theme/20251228.1916/home/.emacs.d/elpa/amaranth-dark-theme-20251228.1916/" 1 1 1 ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amaranth-dark-theme/20251228.1916/home/.emacs.d/elpa/amaranth-dark-theme-20251228.1916/" custom-theme-directory t))"#
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn nil_load_file_name_branch_registers_theme_settings_without_mutating_theme_search_path() {
    let elisp_form = r##"(let ((source
                    (getenv "NEOMACS_PACKAGE_SOURCE"))
                   (custom-theme-load-path
                    '("fixture-theme-directory"))
                   (before
                    (length
                     (get 'amaranth-dark 'theme-settings))))
               (with-temp-buffer
                 (insert-file-contents source)
                 (let ((load-file-name nil))
                   (eval-buffer)))
               (list
                custom-theme-load-path
                (custom-theme-p 'amaranth-dark)
                (memq 'amaranth-dark custom-known-themes)
                before
                (length
                 (get 'amaranth-dark 'theme-settings))))"##;
    let expect = expect![[
        r#"OK (("fixture-theme-directory") #1=(amaranth-dark user changed) #1# 129 258)"#
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_file_registers_only_the_installed_theme_directory_and_metadata() {
    let elisp_form = r##"(let* ((source
                     (file-truename
                      (getenv "NEOMACS_PACKAGE_SOURCE")))
                    (directory
                     (file-name-as-directory
                      (file-name-directory source)))
                    (description
                     (cadr
                      (assq
                       'amaranth-dark-theme
                       package-alist))))
               (list
                (file-name-nondirectory source)
                (cl-count-if
                 (lambda (entry)
                   (and
                    (stringp entry)
                    (equal
                     (file-truename entry)
                     directory)))
                 custom-theme-load-path)
                (member directory custom-theme-load-path)
                (and description
                     (package-version-join
                      (package-desc-version description)))
                (custom-theme-p 'amaranth-dark)
                (featurep 'amaranth-dark-theme)
                (locate-library "amaranth-dark-theme")))"##;
    let expect = expect![[
        r#"OK ("amaranth-dark-theme-autoloads.el" 1 ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amaranth-dark-theme/20251228.1916/home/.emacs.d/elpa/amaranth-dark-theme-20251228.1916/" custom-theme-directory t) "20251228.1916" nil nil "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/amaranth-dark-theme/20251228.1916/home/.emacs.d/elpa/amaranth-dark-theme-20251228.1916/amaranth-dark-theme.el")"#
    ]];

    assert_amaranth_dark_theme_autoload_parity(elisp_form, expect);
}

#[test]
fn resolved_palette_uses_only_the_documented_high_contrast_colors_and_nil_sentinels() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings))
                   colors)
               (dolist (setting settings)
                 (when
                     (eq (car setting) 'theme-face)
                   (let ((tree (nth 3 setting)))
                     (while (consp tree)
                       (let ((item (pop tree)))
                         (cond
                          ((and
                            (stringp item)
                            (string-match-p
                             "\\`#[[:xdigit:]]\\{6\\}\\'"
                             item))
                           (push item colors))
                          ((consp item)
                           (setq tree
                                 (append item tree)))))))))
               (mapcar
                (lambda (color)
                  (cons
                   color
                   (cl-count color colors :test #'equal)))
                (sort
                 (delete-dups colors)
                 #'string<)))"##;
    let expect = expect![[
        r##"OK (("#000000" . 1) ("#080808" . 1) ("#101010" . 1) ("#302d2d" . 1) ("#303540" . 1) ("#4f4949" . 1) ("#598b43" . 1) ("#616775" . 1) ("#7b7171" . 1) ("#959da3" . 1) ("#97a1b5" . 1) ("#a02e2e" . 1) ("#a64d79" . 1) ("#c73c3f" . 1) ("#c81a1a" . 1) ("#e4e4ef" . 1) ("#f4f4ff" . 1) ("#f5f5f5" . 1) ("#ffd966" . 1) ("#ffffff" . 1))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}
