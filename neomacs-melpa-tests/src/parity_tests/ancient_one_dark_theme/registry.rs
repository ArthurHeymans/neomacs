use expect_test::expect;

use super::{assert_ancient_one_dark_theme_autoload_parity, assert_ancient_one_dark_theme_parity};

#[test]
fn ancient_one_dark_theme_exact_pin_metadata_and_registration_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ancient-one-dark-theme
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
          (and
           (custom-theme-p
            'ancient-one-dark)
           t)
          (custom-theme-name-valid-p
           'ancient-one-dark)
          (custom-theme-enabled-p
           'ancient-one-dark)
          (get
           'ancient-one-dark
           'theme-feature)
          (get
           'ancient-one-dark
           'theme-documentation)))"##;
    let expect = expect![[
        r#"OK (ancient-one-dark-theme "20211030.1358" ((emacs (24 1))) "A color theme based off uetchy's Ancient One Dark Theme." ((:revdesc . "a0eaa8bce0ff") (:commit . "a0eaa8bce0ffc25d1469af48a74e80f820bab0ab") (:url . "https://github.com/DaniruKun/ancient-one-dark-emacs-theme")) nil nil t t nil ancient-one-dark-theme nil)"#
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_setting_inventory_preserves_order_duplicates_and_shape() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'ancient-one-dark
                        'theme-settings))))
                    (faces
                     (mapcar #'cadr settings))
                    (duplicates nil)
                    (seen nil))
         (dolist (face faces)
           (if (memq face seen)
               (push face duplicates)
             (push face seen)))
         (list
          (length settings)
          (length
           (delete-dups
            (copy-sequence faces)))
          (nreverse duplicates)
          (delete-dups
           (mapcar #'car settings))
          (delete-dups
           (mapcar #'caddr settings))
          (mapcar
           (lambda (setting)
             (length setting))
           settings)))"##;
    let expect = expect![
        "OK (202 199 (term-color-black line-number line-number-current-line) (theme-face) (ancient-one-dark) (4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4))"
    ];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_complete_face_inventory_matches_source_order() {
    let elisp_form = r##"(mapcar
         #'cadr
         (reverse
          (copy-sequence
           (get
            'ancient-one-dark
            'theme-settings))))"##;
    let expect = expect![
        "OK (default font-lock-builtin-face font-lock-comment-face font-lock-negation-char-face font-lock-reference-face font-lock-constant-face font-lock-doc-face font-lock-function-name-face font-lock-keyword-face font-lock-string-face font-lock-type-face font-lock-variable-name-face font-lock-warning-face term-color-black region highlight hl-line centaur-tabs-selected centaur-tabs-unselected fringe cursor show-paren-match-face isearch mode-line mode-line-inactive mode-line-buffer-id mode-line-highlight mode-line-emphasis vertical-border minibuffer-prompt default-italic link org-code org-hide org-level-1 org-level-2 org-level-3 org-level-4 org-footnote org-link org-special-keyword org-quote org-verse org-todo org-done org-block org-date org-warning org-agenda-structure org-agenda-date org-agenda-date-weekend org-agenda-date-today org-agenda-done org-scheduled org-scheduled-today org-ellipsis org-verbatim org-document-info-keyword font-latex-bold-face font-latex-italic-face font-latex-string-face font-latex-match-reference-keywords font-latex-match-variable-keywords ido-only-match org-sexp-date ido-first-match ivy-current-match gnus-header-content gnus-header-from gnus-header-name gnus-header-subject mu4e-view-url-number-face mu4e-cited-1-face mu4e-cited-7-face mu4e-header-marks-face ffap js2-private-function-call js2-jsdoc-html-tag-delimiter js2-jsdoc-html-tag-name js2-external-variable js2-function-param js2-jsdoc-value js2-private-member js3-warning-face js3-error-face js3-external-variable-face js3-function-param-face js3-jsdoc-tag-face js3-instance-member-face warning ac-completion-face info-quoted-name info-string icompletep-determined undo-tree-visualizer-current-face undo-tree-visualizer-default-face undo-tree-visualizer-unmodified-face undo-tree-visualizer-register-face slime-repl-inputed-output-face trailing-whitespace rainbow-delimiters-depth-1-face rainbow-delimiters-depth-2-face rainbow-delimiters-depth-3-face rainbow-delimiters-depth-4-face rainbow-delimiters-depth-5-face rainbow-delimiters-depth-6-face rainbow-delimiters-depth-7-face rainbow-delimiters-depth-8-face magit-item-highlight magit-section-heading magit-hunk-heading magit-section-highlight magit-hunk-heading-highlight magit-diff-context-highlight magit-diffstat-added magit-diffstat-removed magit-process-ok magit-process-ng magit-branch magit-log-author magit-hash magit-diff-file-header lazy-highlight term term-color-black term-color-blue term-color-red term-color-green term-color-yellow term-color-magenta term-color-cyan term-color-white rainbow-delimiters-unmatched-face helm-header helm-source-header helm-selection helm-selection-line helm-visible-mark helm-candidate-number helm-separator helm-time-zone-current helm-time-zone-home helm-buffer-not-saved helm-buffer-process helm-buffer-saved-out helm-buffer-size helm-ff-directory helm-ff-file helm-ff-executable helm-ff-invalid-symlink helm-ff-symlink helm-ff-prefix helm-grep-cmd-line helm-grep-file helm-grep-finish helm-grep-lineno helm-grep-match helm-grep-running helm-moccur-buffer helm-source-go-package-godoc-description helm-bookmark-w3m company-echo-common company-preview company-preview-common company-preview-search company-scrollbar-bg company-scrollbar-fg company-tooltip company-tooltop-annotation company-tooltip-common company-tooltip-common-selection company-tooltip-mouse company-tooltip-selection company-template-field web-mode-builtin-face web-mode-comment-face web-mode-constant-face web-mode-keyword-face web-mode-doctype-face web-mode-function-name-face web-mode-string-face web-mode-type-face web-mode-html-attr-name-face web-mode-html-attr-value-face web-mode-warning-face web-mode-html-tag-face jde-java-font-lock-package-face jde-java-font-lock-public-face jde-java-font-lock-private-face jde-java-font-lock-constant-face jde-java-font-lock-modifier-face jde-jave-font-lock-protected-face jde-java-font-lock-number-face line-number line-number-current-line line-number line-number-current-line tab-line tab-line-tab tab-line-tab-inactive tab-line-tab-current tab-line-highlight)"
    ];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_source_registers_one_exact_load_path_entry() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source))))
         (list
          (and
           (memq
            'ancient-one-dark
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
                  (equal entry directory)
                (setq count
                      (1+ count)))))
          (file-name-nondirectory
           (locate-library
            "ancient-one-dark-theme"))))"##;
    let expect = expect![[r#"OK (t t 1 "ancient-one-dark-theme.el")"#]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_installed_payload_is_exact_and_unvendored() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'ancient-one-dark-theme
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
                  (string-suffix-p
                   ".elc"
                   name)
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
                  (insert-file-contents-literally
                   path)
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
        r#"OK (("README-elpa" 248 "5c15b55cd6b605211d56e02cd51a3e47cdaeb669479f4714d45e047821f07ecd") ("ancient-one-dark-theme-autoloads.el" 804 "0a3a4beac6c613739fec1de7dc1b1abcdff15710be4c053c00c6ecdd155f4449") ("ancient-one-dark-theme-pkg.el" 337 "f6e15461321f1d3ff11141dc533581e2b265fbec2ae57cbfd2629a9d8da2444a") ("ancient-one-dark-theme.el" 16854 "004f174754c688f358fa2afc4f8699b5db647fbfaa0d6b55ff39f63e05bfbbf5"))"#
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_autoload_registers_path_without_loading_theme_source() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source))))
         (list
          (featurep
           'ancient-one-dark-theme)
          (custom-theme-p
           'ancient-one-dark)
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

    assert_ancient_one_dark_theme_autoload_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_reloads_accumulate_exact_settings_but_not_load_paths() {
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
               'ancient-one-dark
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
    let expect = expect!["OK ((404 1) (606 1))"];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}
