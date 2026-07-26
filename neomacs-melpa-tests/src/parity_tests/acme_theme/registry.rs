use expect_test::expect;

use super::{assert_acme_theme_autoload_parity, assert_acme_theme_parity};

#[test]
fn acme_theme_exact_pin_metadata_features_and_theme_registration_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'acme-theme
                      package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join
           (package-desc-version descriptor))
          (package-desc-reqs descriptor)
          (package-desc-summary descriptor)
          (copy-tree
           (package-desc-extras descriptor))
          (package-desc-kind descriptor)
          (package-desc-archive descriptor)
          (featurep
           'acme-theme)
          (and
           (custom-theme-p
            'acme)
           t)
          (custom-theme-name-valid-p
           'acme)
          (custom-theme-enabled-p
           'acme)
          (get
           'acme
           'theme-feature)
          (get
           'acme
           'theme-documentation)))"##;
    let expect = expect![[
        r#"OK (acme-theme "20210430.302" nil "A color theme based on Acme & Sam from Plan 9." ((:revdesc . "ae8788b5851e") (:commit . "ae8788b5851ea353fbb80ab586a3bbd5dc8e91aa") (:url . "https://github.com/ianpan870102/acme-emacs-theme")) nil nil t t t nil acme-theme "A color theme based on Acme & Sam")"#
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_group_and_black_foreground_custom_option_match_exact_metadata() {
    let elisp_form = r##"(list
         (get
          'acme-theme
          'group-documentation)
         (copy-tree
          (get
           'acme-theme
           'custom-group))
         (and
          (member
           '(acme-theme custom-group)
           (get
            'faces
            'custom-group))
          t)
         acme-theme-black-fg
         (default-boundp
          'acme-theme-black-fg)
         (default-value
          'acme-theme-black-fg)
         (let ((standard
                (get
                 'acme-theme-black-fg
                 'standard-value)))
           (list
            (and standard t)
            (eval
             (car standard)
             t)))
         (get
          'acme-theme-black-fg
          'custom-type)
         (get
          'acme-theme-black-fg
          'custom-group)
         (documentation-property
          'acme-theme-black-fg
          'variable-documentation
          t)
         (let ((file
                (symbol-file
                 'acme-theme-black-fg
                 'defvar)))
           (and file
                (file-name-nondirectory
                 file))))"##;
    let expect = expect![[
        r#"OK ("Options for acme theme." ((acme-theme-black-fg custom-variable)) t nil t nil (t nil) boolean nil "If non-nil, foreground will be pure black instead of the default dark grey." "acme-theme.el")"#
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_setting_inventory_has_exact_source_order_shape_and_duplicate() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (faces
                     (mapcar
                      #'cadr
                      settings))
                    (duplicates
                     (let (seen repeated)
                       (dolist
                           (face faces)
                         (if
                             (memq face seen)
                             (push face repeated)
                           (push face seen)))
                       (nreverse repeated))))
               (list
                (length settings)
                (length
                 (delete-dups
                  (copy-sequence faces)))
                duplicates
                (delete-dups
                 (mapcar
                  #'car
                  settings))
                (delete-dups
                 (mapcar
                  #'caddr
                  settings))))"##;
    let expect = expect!["OK (314 313 (ivy-minibuffer-match-face-3) (theme-face) (acme))"];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_source_registers_one_exact_theme_path_and_both_providers() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source))))
               (list
                (featurep
                 'acme-theme)
                (and
                 (memq
                  'acme
                  custom-known-themes)
                 t)
                (and
                 (member
                  directory
                  custom-theme-load-path)
                 t)
                (let ((count 0))
                  (dolist
                      (entry
                       custom-theme-load-path
                       count)
                    (when
                        (equal
                         entry
                         directory)
                      (setq count
                            (1+ count)))))
                (file-name-nondirectory
                 (locate-library
                  "acme-theme"))
                (let ((file
                       (symbol-file
                        'acme-theme-black-fg
                        'defvar)))
                  (and file
                       (file-name-nondirectory
                        file)))))"##;
    let expect = expect![[r#"OK (t t t 1 "acme-theme.el" "acme-theme.el")"#]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_packaged_source_descriptor_autoload_readme_and_bytecode_assets_match() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'acme-theme
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor)))
         (mapcar
          (lambda (name)
            (let ((path
                   (expand-file-name
                    name
                    directory)))
              (if
                  (equal
                   name
                   "acme-theme.elc")
                  (list
                   name
                   (file-exists-p path)
                   (file-regular-p path)
                   (and
                    (>
                     (nth
                      7
                      (file-attributes
                       path))
                     0)
                    t))
                (with-temp-buffer
                  (set-buffer-multibyte nil)
                  (insert-file-contents-literally
                   path)
                  (list
                   name
                   (file-exists-p path)
                   (file-regular-p path)
                   (buffer-size)
                   (secure-hash
                    'sha256
                    (current-buffer)))))))
          '("acme-theme.el"
            "acme-theme.elc"
            "acme-theme-autoloads.el"
            "acme-theme-pkg.el"
            "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("acme-theme.el" t t 35251 "f1c8202c772d1de83eda4765fe21429a528a4fb350a28394d3705fe9678ed1f9") ("acme-theme.elc" t t t) ("acme-theme-autoloads.el" t t 849 "2a8b1e56b5b25871234dfda951da419a1c931e03d671b48f399e2837cc77b7ef") ("acme-theme-pkg.el" t t 290 "9761f97a75d2643ecaf8d6794148796c4a2288e9959e973507048adeb4a0f0e7") ("README-elpa" t t 56 "f0ee18aaf6eeeeda62380c9a901b5bd1ddf749c6a96275d9e89123fbcb3bd52c"))"#
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_installed_package_contains_only_the_exact_melpa_assets() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'acme-theme
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor)))
         (sort
          (directory-files
           directory
           nil
           "\\`[^.]")
          #'string<))"##;
    let expect = expect![[
        "OK (\"README-elpa\" \"acme-theme-autoloads.el\" \"acme-theme-pkg.el\" \"acme-theme.el\" \"acme-theme.elc\")"
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_autoload_file_adds_theme_path_without_loading_runtime() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source))))
               (list
                (featurep
                 'acme-theme)
                (featurep
                 'acme-theme-autoloads)
                (and
                 (custom-theme-p
                  'acme)
                 t)
                (and
                 (member
                  directory
                 custom-theme-load-path)
                 t)
                (let ((count 0))
                  (dolist
                      (entry
                       custom-theme-load-path
                       count)
                    (when
                        (equal
                         entry
                         directory)
                      (setq count
                            (1+ count)))))
                (let ((count 0))
                  (dolist
                      (entry
                       load-path
                       count)
                    (when
                        (equal
                         (file-name-as-directory
                          entry)
                         directory)
                      (setq count
                            (1+ count)))))
                (file-name-nondirectory
                 (locate-library
                  "acme-theme"))
                (file-name-nondirectory
                 (locate-library
                  "acme-theme-autoloads"))))"##;
    let expect = expect![[r#"OK (nil t nil t 1 1 "acme-theme.el" "acme-theme-autoloads.el")"#]];
    assert_acme_theme_autoload_parity(elisp_form, expect);
}
