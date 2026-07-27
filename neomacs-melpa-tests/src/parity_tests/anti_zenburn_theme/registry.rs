use expect_test::expect;

use super::{assert_anti_zenburn_theme_autoload_parity, assert_anti_zenburn_theme_parity};

#[test]
fn anti_zenburn_theme_exact_pin_metadata_and_theme_registration_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'anti-zenburn-theme
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
          (featurep 'anti-zenburn-theme)
          (and
           (custom-theme-p 'anti-zenburn)
           t)
          (custom-theme-name-valid-p
           'anti-zenburn)
          (custom-theme-enabled-p
           'anti-zenburn)
          (get
           'anti-zenburn
           'theme-feature)
          (get
           'anti-zenburn
           'theme-documentation)))"##;
    let expect = expect![[
        r#"OK (anti-zenburn-theme "20180712.1838" nil "Low-contrast Zenburn-inverted theme." ((:maintainers ("Andrey Kotlarski" . "m00naticus@gmail.com")) (:authors ("Andrey Kotlarski" . "m00naticus@gmail.com")) (:revdesc . "dbafbaa86be6") (:commit . "dbafbaa86be67c1d409873f57a5c0bbe1e7ca158") (:url . "https://github.com/m00natic/anti-zenburn-theme")) nil nil t t t nil anti-zenburn-theme "Reversed Zenburn color theme.")"#
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_complete_setting_inventory_has_exact_shape_and_digest() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'anti-zenburn
                        'theme-settings))))
                    (face-settings
                     (seq-filter
                      (lambda (setting)
                        (eq
                         (car setting)
                         'theme-face))
                      settings))
                    (value-settings
                     (seq-filter
                      (lambda (setting)
                        (eq
                         (car setting)
                         'theme-value))
                      settings))
                    (faces
                     (mapcar #'cadr
                             face-settings))
                    (duplicates nil)
                    (seen nil))
         (dolist (face faces)
           (if (memq face seen)
               (push face duplicates)
             (push face seen)))
         (list
          (length settings)
          (length face-settings)
          (length value-settings)
          (length
           (delete-dups
            (copy-sequence faces)))
          (nreverse duplicates)
          (mapcar #'cadr
                  value-settings)
          (seq-take faces 12)
          (seq-subseq faces 512 524)
          (last faces 12)
          (secure-hash
           'sha256
           (mapconcat
            #'prin1-to-string
            settings
            "\n"))))"##;
    let expect = expect![[
        r#"OK (1058 1049 9 1049 nil (ansi-color-names-vector company-quickhelp-color-background company-quickhelp-color-foreground fci-rule-color nrepl-message-colors pdf-view-midnight-colors vc-annotate-color-map vc-annotate-very-old-color vc-annotate-background) (button link link-visited default cursor escape-glyph widget-field fringe header-line highlight success warning) (ido-indicator iedit-occurrence js2-warning js2-error js2-jsdoc-tag js2-jsdoc-type js2-jsdoc-value js2-function-param js2-external-variable js2-instance-member js2-jsdoc-html-tag-delimiter js2-jsdoc-html-tag-name) (wl-highlight-summary-thread-top-face wl-highlight-thread-indent-face wl-highlight-summary-refiled-face wl-highlight-summary-displaying-face which-func cscope-file-face cscope-function-face cscope-line-number-face cscope-mouse-face cscope-separator-face yascroll:thumb-text-area yascroll:thumb-fringe) "c8f1a7f04798e41851be42cb7b50e5576ff49e058ab5fb118903bae17064d2c6")"#
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_installed_payload_is_exact_and_kept_in_runtime_cache() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'anti-zenburn-theme
                    package-alist)))
                 (directory
                  (package-desc-dir descriptor)))
         (mapcar
          (lambda (name)
            (let ((path
                   (expand-file-name
                    name
                    directory)))
              (if
                  (string-suffix-p ".elc" name)
                  (list
                   name
                   (file-regular-p path)
                   (>
                    (nth
                     7
                     (file-attributes path))
                    0))
                (with-temp-buffer
                  (set-buffer-multibyte nil)
                  (insert-file-contents-literally path)
                  (list
                   name
                   (buffer-size)
                   (secure-hash
                    'sha256
                    (current-buffer)))))))
          (sort
           (directory-files
            directory
            nil
            "\\`[^.]")
           #'string<)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 111 "cf26c6c7df7d106e3261b88752d490aa9d459e3be6c1c0b36d0be25fade05512") ("anti-zenburn-theme-autoloads.el" 890 "6f72bace3f735f9d37eb644be57ef7639d983dbbd72b3a4c3615168ea5620ebc") ("anti-zenburn-theme-pkg.el" 411 "0bfe23f7fd89735fa77550010ecbf40f1c4fef386bcc75c0bf96abdc5511f899") ("anti-zenburn-theme.el" 88635 "54a07e4250791390837b3b30289c49b4972cdf350fb12e6430715fc97087caf4") ("anti-zenburn-theme.elc" t t))"#
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_source_and_autoload_each_register_one_theme_path() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source))))
         (list
          (featurep
           'anti-zenburn-theme)
          (custom-theme-p
           'anti-zenburn)
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
                  (equal entry directory)
                (setq count
                      (1+ count)))))
          (file-name-nondirectory
           (locate-library
            "anti-zenburn-theme"))))"##;
    let expect = expect![[r#"OK (t (anti-zenburn user changed) t 1 "anti-zenburn-theme.el")"#]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_generated_autoload_registers_path_without_loading_theme() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source))))
         (list
          (featurep
           'anti-zenburn-theme)
          (custom-theme-p
           'anti-zenburn)
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
                  (equal entry directory)
                (setq count
                      (1+ count)))))))"##;
    let expect = expect!["OK (nil nil t 1)"];

    assert_anti_zenburn_theme_autoload_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_source_reloads_accumulate_settings_but_not_load_paths() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source)))
                    observations)
         (dolist (_ '(first second))
           (load source nil t t)
           (push
            (list
             (length
              (get
               'anti-zenburn
               'theme-settings))
             (let ((count 0))
               (dolist
                   (entry
                    custom-theme-load-path
                    count)
                 (when
                     (equal entry directory)
                   (setq count
                         (1+ count))))))
            observations))
         (nreverse observations))"##;
    let expect = expect!["OK ((2116 1) (3174 1))"];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}
