use expect_test::expect;

use super::{assert_arjen_grey_theme_autoload_parity, assert_arjen_grey_theme_parity};

#[test]
fn arjen_grey_theme_descriptor_pins_exact_release_metadata_and_dependencies() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'arjen-grey-theme package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-summary descriptor)
                (package-desc-reqs descriptor)
                (package-desc-kind descriptor)
                (package-desc-archive descriptor)
                (package-desc-extras descriptor)))"##;
    let expect = expect![[
        r#"OK (arjen-grey-theme "20170522.2047" "A soothing dark grey theme." nil nil nil ((:maintainers ("Arjen Wiersma" . "arjen@wiersma.org")) (:authors ("Arjen Wiersma" . "arjen@wiersma.org")) (:keywords "faces") (:revdesc . "4cd0be72b65d") (:commit . "4cd0be72b65d42390e2105cfdaa408a1ead8d8d1") (:url . "https://github.com/credmp/arjen-grey")))"#
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_installed_payload_has_exact_files_sizes_and_source_hashes() {
    let elisp_form = r##"(let* ((descriptor
                     (cadr
                      (assq 'arjen-grey-theme package-alist)))
                    (directory (package-desc-dir descriptor))
                    (files
                     (sort
                      (directory-files
                       directory t "^[^.].*")
                      #'string<)))
               (mapcar
                (lambda (file)
                  (list
                   (file-name-nondirectory file)
                   (file-attribute-size
                    (file-attributes file))
                   (and
                    (member
                     (file-name-nondirectory file)
                     '("README-elpa"
                       "arjen-grey-theme-autoloads.el"
                       "arjen-grey-theme-pkg.el"
                       "arjen-grey-theme.el"))
                    (with-temp-buffer
                      (insert-file-contents-literally file)
                      (secure-hash
                       'sha256
                       (current-buffer))))))
                files))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 36 "b1c421dfad81450e708abf4080e0e83f5ae70305ccf84688e56f9f1d54975f24") ("arjen-grey-theme-autoloads.el" 878 "212acbbe16f5e3706417963c9650dff44d6e4a8e052c02ca1d424f504a5affa3") ("arjen-grey-theme-pkg.el" 401 "4423e5685459901397bd468deb910c3918fd3b09eee3823cee63a9e8fea11b26") ("arjen-grey-theme.el" 4323 "8b4af9cacdaaf0a85e968abb2111f563cf82a16a005b7fac6f6026cc5d13bd10") ("arjen-grey-theme.elc" 2905 nil))"#
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_registry_has_exact_documentation_feature_and_setting_shape() {
    let elisp_form = r##"(let ((settings
                    (get 'arjen-grey 'theme-settings)))
               (list
                (featurep 'arjen-grey-theme)
                (custom-theme-p 'arjen-grey)
                (custom-theme-name-valid-p 'arjen-grey)
                (custom-theme-enabled-p 'arjen-grey)
                (get 'arjen-grey 'theme-feature)
                (get 'arjen-grey 'theme-documentation)
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
                (memq 'arjen-grey custom-known-themes)
                custom-enabled-themes))"##;
    let expect = expect![[
        r#"OK (t #1=(arjen-grey user changed) t nil arjen-grey-theme "A soothing dark grey theme by Arjen" 38 ((theme-face . 37) (theme-value . 1)) #1# nil)"#
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_registry_preserves_complete_ordered_setting_inventory() {
    let elisp_form = r##"(let ((settings
                    (get 'arjen-grey 'theme-settings)))
               (list
                (mapcar
                 (lambda (setting)
                   (list
                    (car setting)
                    (cadr setting)
                    (caddr setting)))
                 settings)
                (length
                 (delete-dups
                  (mapcar #'cadr settings)))
                (seq-every-p
                 (lambda (setting)
                   (and
                    (= (length setting) 4)
                    (eq (caddr setting) 'arjen-grey)))
                 settings)))"##;
    let expect = expect![
        "OK (((theme-value hl-paren-colors arjen-grey) (theme-face gnus-summary-normal-read arjen-grey) (theme-face widget-button arjen-grey) (theme-face gnus-header-subject arjen-grey) (theme-face gnus-header-content arjen-grey) (theme-face gnus-header-name arjen-grey) (theme-face company-preview-common arjen-grey) (theme-face company-preview arjen-grey) (theme-face company-scrollbar-bg arjen-grey) (theme-face company-scrollbar-fg arjen-grey) (theme-face company-tooltip-common arjen-grey) (theme-face company-tooltip-mouse arjen-grey) (theme-face company-tooltip-selection arjen-grey) (theme-face company-tooltip-annotation arjen-grey) (theme-face company-tooltip arjen-grey) (theme-face persp-selected-face arjen-grey) (theme-face helm-selection-line arjen-grey) (theme-face helm-selection arjen-grey) (theme-face helm-ff-directory arjen-grey) (theme-face helm-source-header arjen-grey) (theme-face helm-header arjen-grey) (theme-face font-lock-warning-face arjen-grey) (theme-face minibuffer-prompt arjen-grey) (theme-face font-lock-variable-name-face arjen-grey) (theme-face font-lock-constant-face arjen-grey) (theme-face font-lock-type-face arjen-grey) (theme-face font-lock-string-face arjen-grey) (theme-face font-lock-keyword-face arjen-grey) (theme-face font-lock-function-name-face arjen-grey) (theme-face font-lock-comment-face arjen-grey) (theme-face font-lock-builtin-face arjen-grey) (theme-face secondary-selection arjen-grey) (theme-face linum arjen-grey) (theme-face region arjen-grey) (theme-face mode-line arjen-grey) (theme-face fringe arjen-grey) (theme-face cursor arjen-grey) (theme-face default arjen-grey)) 38 t)"
    ];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_source_registers_one_real_theme_load_path_entry() {
    let elisp_form = r##"(let* ((source
                     (getenv "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory source))))
               (list
                (member directory custom-theme-load-path)
                (car custom-theme-load-path)
                (seq-count
                 (lambda (entry)
                   (equal entry directory))
                 custom-theme-load-path)
                (locate-file
                 "arjen-grey-theme.el"
                 custom-theme-load-path)
                (file-name-nondirectory
                 (or
                  (symbol-file
                   'arjen-grey-theme
                   'feature)
                  ""))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/arjen-grey-theme/20170522.2047/home/.emacs.d/elpa/arjen-grey-theme-20170522.2047/" custom-theme-directory t) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/arjen-grey-theme/20170522.2047/home/.emacs.d/elpa/arjen-grey-theme-20170522.2047/" 1 "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/arjen-grey-theme/20170522.2047/home/.emacs.d/elpa/arjen-grey-theme-20170522.2047/arjen-grey-theme.el" "")"#
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_autoload_registers_path_without_eager_theme_loading() {
    let elisp_form = r##"(let* ((source
                     (getenv "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory source))))
               (list
                (featurep 'arjen-grey-theme)
                (custom-theme-p 'arjen-grey)
                (member directory custom-theme-load-path)
                (seq-count
                 (lambda (entry)
                   (equal entry directory))
                 custom-theme-load-path)
                (file-readable-p
                 (expand-file-name
                  "arjen-grey-theme.el"
                  directory))
                (file-name-nondirectory source)))"##;
    let expect = expect![[
        r#"OK (nil nil ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/arjen-grey-theme/20170522.2047/home/.emacs.d/elpa/arjen-grey-theme-20170522.2047/" custom-theme-directory t) 1 t "arjen-grey-theme-autoloads.el")"#
    ]];
    assert_arjen_grey_theme_autoload_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_source_reload_accumulates_settings_but_deduplicates_load_path() {
    let elisp_form = r##"(let* ((source
                     (getenv "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory source)))
                    observations)
               (dolist (_ '(first second))
                 (load source nil t t)
                 (push
                  (list
                   (length
                    (get 'arjen-grey 'theme-settings))
                   (seq-count
                    (lambda (entry)
                      (equal entry directory))
                    custom-theme-load-path)
                   (seq-count
                    (lambda (feature)
                      (eq feature 'arjen-grey-theme))
                    features))
                  observations))
               (nreverse observations))"##;
    let expect = expect!["OK ((76 1 1) (114 1 1))"];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}
