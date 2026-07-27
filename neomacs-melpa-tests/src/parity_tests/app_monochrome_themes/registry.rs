use expect_test::expect;

use super::assert_app_monochrome_parity;

#[test]
fn app_monochrome_descriptor_records_exact_pin_dependency_and_payload() {
    let elisp_form = r##"(let* ((desc
                (cadr (assq 'app-monochrome-themes package-alist)))
              (dir (package-desc-dir desc)))
         (list
          (package-version-join (package-desc-version desc))
          (package-desc-reqs desc)
          (package-desc-kind desc)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files dir t "^[^.].*"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20250710.2315" ((emacs (26 1))) nil ("README-elpa" "app-monochrome-themes-autoloads.el" "app-monochrome-themes-dark-theme-theme.el" "app-monochrome-themes-dark-theme-theme.elc" "app-monochrome-themes-light-theme-theme.el" "app-monochrome-themes-light-theme-theme.elc" "app-monochrome-themes-pkg.el" "app-monochrome-themes.el" "app-monochrome-themes.elc"))"#
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_main_library_registers_feature_group_and_theme_path() {
    let elisp_form = r##"(let* ((source
                (locate-library "app-monochrome-themes"))
              (directory (file-name-directory source)))
         (list
          (featurep 'app-monochrome-themes)
          (get 'app-monochrome-themes 'custom-group)
          (get 'app-monochrome-themes 'group-documentation)
          (file-name-nondirectory source)
          (member directory custom-theme-load-path)
          (mapcar
           (lambda (entry)
             (cond
              ((stringp entry)
               (file-name-nondirectory
                (directory-file-name entry)))
              (t entry)))
           custom-theme-load-path)))"##;
    let expect = expect![[
        r#"OK (t nil "App Monochrome themes." "app-monochrome-themes.el" ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/app-monochrome-themes/20250710.2315/home/.emacs.d/elpa/app-monochrome-themes-20250710.2315/" custom-theme-directory t) ("app-monochrome-themes-20250710.2315" custom-theme-directory t))"#
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_main_reload_keeps_single_package_theme_path() {
    let elisp_form = r##"(let* ((source
                (locate-library "app-monochrome-themes"))
              (directory (file-name-directory source))
              (before
               (cl-count directory custom-theme-load-path
                         :test #'equal)))
         (load source nil 'nomessage)
         (load source nil 'nomessage)
         (list before
               (cl-count directory custom-theme-load-path
                         :test #'equal)
               (cl-count 'app-monochrome-themes features)
               (car custom-theme-load-path)))"##;
    let expect = expect![[
        r#"OK (1 1 1 "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/app-monochrome-themes/20250710.2315/home/.emacs.d/elpa/app-monochrome-themes-20250710.2315/")"#
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_both_themes_load_from_registered_path_with_exact_counts() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (load-theme 'app-monochrome-themes-light-theme t)
         (let ((dark
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings))
               (light
                (get 'app-monochrome-themes-light-theme
                     'theme-settings)))
           (list
            (length dark)
            (length (delete-dups (mapcar #'cadr dark)))
            (length light)
            (length (delete-dups (mapcar #'cadr light)))
            custom-enabled-themes
            (memq 'app-monochrome-themes-dark-theme
                  custom-known-themes)
            (memq 'app-monochrome-themes-light-theme
                  custom-known-themes))))"##;
    let expect = expect![
        "OK (176 172 167 166 (app-monochrome-themes-light-theme app-monochrome-themes-dark-theme) #1=(app-monochrome-themes-dark-theme user changed) (app-monochrome-themes-light-theme . #1#))"
    ];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_theme_exact_sorted_face_registry_is_stable() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (sort
          (delete-dups
           (mapcar
            #'cadr
            (get 'app-monochrome-themes-dark-theme
                 'theme-settings)))
          (lambda (left right)
            (string< (symbol-name left)
                     (symbol-name right)))))"##;
    let expect = expect![
        "OK (bookmark-face compilation-line-number completions-common-part counsel-active-mode counsel-application-name counsel-outline-1 counsel-variable-documentation custom-link default dired-directory dired-flagged dired-set-id dirvish-media-info-property-key doom-modeline-buffer-file doom-modeline-lsp-success error fancy-compilation-default-face flycheck-fringe-warning flycheck-info flymake-error flyspell-incorrect font-latex-bold-face font-latex-italic-face font-latex-math-face font-latex-sectioning-5-face font-latex-string-face font-latex-underline-face font-latex-verbatim-face font-latex-warning-face font-lock-builtin-face font-lock-comment-face font-lock-constant-face font-lock-doc-face font-lock-doc-markup-face font-lock-function-name-face font-lock-keyword-face font-lock-negation-char-face font-lock-preprocessor-face font-lock-string-face font-lock-type-face font-lock-variable-name-face git-commit-keyword git-commit-summary highlight isearch italic ivy-action ivy-cursor ivy-grep-line-number ivy-match-required-face ivy-prompt-match ivy-virtual jinx-highlight jinx-misspelled lazy-highlight line-number link lsp-face-highlight-textual lsp-face-highlight-write lsp-face-semhl-function lsp-face-semhl-implementation lsp-face-semhl-type lsp-rust-analyzer-inlay-face lsp-rust-analyzer-inlay-param-face lsp-rust-analyzer-inlay-type-face lsp-ui-doc-background lsp-ui-doc-highlight-hover lsp-ui-peek-line-number lsp-ui-peek-list lsp-ui-sideline-global lsp-ui-sideline-symbol magit-blame-name magit-branch-current magit-branch-local magit-branch-remote magit-branch-upstream magit-diff-base-highlight magit-diff-file-heading magit-filename magit-keyword magit-keyword-squash magit-section-child-count magit-section-heading magit-section-heading-selection magit-section-highlight magit-section-secondary-heading magit-tag markdown-header-face markdown-header-face-1 markdown-header-face-2 markdown-header-face-3 markdown-header-face-4 markdown-header-face-5 markdown-header-face-6 markdown-inline-code-face minibuffer-prompt mode-line mode-line-buffer-id mode-line-highlight orderless-match-face-0 orderless-match-face-1 orderless-match-face-2 orderless-match-face-3 org-code org-default org-done org-headline-done org-headline-todo org-level-1 org-level-2 org-level-3 org-level-4 org-level-5 org-level-6 org-level-7 org-level-8 org-list-dt org-modern-done org-modern-todo org-table org-todo org-verbatim rainbow-delimiters-depth-1-face rainbow-delimiters-depth-2-face rainbow-delimiters-depth-3-face rainbow-delimiters-depth-4-face rainbow-delimiters-depth-5-face rainbow-delimiters-depth-6-face rainbow-delimiters-depth-7-face rainbow-delimiters-depth-8-face rainbow-delimiters-depth-9-face rust-unsafe show-paren-match show-paren-mismatch sp-show-pair-match-content-face success swiper-background-match-face-1 swiper-background-match-face-3 swiper-line-face swiper-match-face-1 trailing-whitespace tree-sitter-hl-face:attribute tree-sitter-hl-face:constant tree-sitter-hl-face:constant.builtin tree-sitter-hl-face:constructor tree-sitter-hl-face:doc tree-sitter-hl-face:embedded tree-sitter-hl-face:escape tree-sitter-hl-face:function tree-sitter-hl-face:function.call tree-sitter-hl-face:function.special tree-sitter-hl-face:label tree-sitter-hl-face:method tree-sitter-hl-face:method.call tree-sitter-hl-face:number tree-sitter-hl-face:operator tree-sitter-hl-face:property tree-sitter-hl-face:property.definition tree-sitter-hl-face:punctuation tree-sitter-hl-face:punctuation.delimiter tree-sitter-hl-face:string.special tree-sitter-hl-face:type tree-sitter-hl-face:type.argument tree-sitter-hl-face:type.builtin tree-sitter-hl-face:type.parameter tree-sitter-hl-face:type.super tree-sitter-hl-face:variable tree-sitter-hl-face:variable.builtin tree-sitter-hl-face:variable.parameter tree-sitter-hl-face:variable.special variable-pitch warning)"
    ];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_theme_exact_sorted_face_registry_is_stable() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-light-theme t)
         (sort
          (delete-dups
           (mapcar
            #'cadr
            (get 'app-monochrome-themes-light-theme
                 'theme-settings)))
          (lambda (left right)
            (string< (symbol-name left)
                     (symbol-name right)))))"##;
    let expect = expect![
        "OK (bookmark-face compilation-line-number completions-common-part counsel-active-mode counsel-application-name counsel-outline-1 counsel-variable-documentation custom-link default dired-directory dired-flagged dired-set-id dirvish-media-info-property-key doom-modeline-buffer-file doom-modeline-lsp-success error fancy-compilation-default-face flycheck-error flycheck-fringe-warning flycheck-info flycheck-warning flymake-error flyspell-incorrect font-lock-builtin-face font-lock-comment-face font-lock-constant-face font-lock-doc-face font-lock-doc-markup-face font-lock-function-name-face font-lock-keyword-face font-lock-negation-char-face font-lock-preprocessor-face font-lock-string-face font-lock-type-face font-lock-variable-name-face git-commit-keyword git-commit-summary highlight isearch italic ivy-action ivy-cursor ivy-grep-line-number ivy-match-required-face ivy-prompt-match ivy-virtual jinx-highlight jinx-misspelled lazy-highlight line-number link lsp-face-highlight-textual lsp-face-highlight-write lsp-face-semhl-function lsp-face-semhl-implementation lsp-face-semhl-type lsp-rust-analyzer-inlay-face lsp-rust-analyzer-inlay-param-face lsp-rust-analyzer-inlay-type-face lsp-ui-doc-background lsp-ui-doc-highlight-hover lsp-ui-peek-line-number lsp-ui-peek-list lsp-ui-sideline-global lsp-ui-sideline-symbol magit-blame-name magit-branch-current magit-branch-local magit-branch-remote magit-branch-upstream magit-diff-base-highlight magit-diff-file-heading magit-filename magit-keyword magit-keyword-squash magit-section-child-count magit-section-heading magit-section-heading-selection magit-section-highlight magit-section-secondary-heading magit-tag markdown-header-face markdown-header-face-1 markdown-header-face-2 markdown-header-face-3 markdown-header-face-4 markdown-header-face-5 markdown-header-face-6 minibuffer-prompt mode-line mode-line-buffer-id mode-line-highlight orderless-match-face-0 orderless-match-face-1 orderless-match-face-2 orderless-match-face-3 org-block org-code org-default org-done org-headline-done org-headline-todo org-level-1 org-level-2 org-level-3 org-level-4 org-level-5 org-level-6 org-level-7 org-level-8 org-list-dt org-modern-done org-modern-todo org-table org-todo org-verbatim rainbow-delimiters-depth-1-face rainbow-delimiters-depth-2-face rainbow-delimiters-depth-3-face rainbow-delimiters-depth-4-face rainbow-delimiters-depth-5-face rainbow-delimiters-depth-6-face rainbow-delimiters-depth-7-face rainbow-delimiters-depth-8-face rainbow-delimiters-depth-9-face rust-unsafe show-paren-match show-paren-mismatch sp-show-pair-match-content-face success swiper-background-match-face-1 swiper-background-match-face-3 swiper-line-face swiper-match-face-1 trailing-whitespace tree-sitter-hl-face:attribute tree-sitter-hl-face:constant tree-sitter-hl-face:constant.builtin tree-sitter-hl-face:constructor tree-sitter-hl-face:doc tree-sitter-hl-face:embedded tree-sitter-hl-face:escape tree-sitter-hl-face:function tree-sitter-hl-face:function.call tree-sitter-hl-face:function.special tree-sitter-hl-face:label tree-sitter-hl-face:method tree-sitter-hl-face:method.call tree-sitter-hl-face:number tree-sitter-hl-face:operator tree-sitter-hl-face:property tree-sitter-hl-face:property.definition tree-sitter-hl-face:punctuation tree-sitter-hl-face:punctuation.delimiter tree-sitter-hl-face:string.special tree-sitter-hl-face:type tree-sitter-hl-face:type.argument tree-sitter-hl-face:type.builtin tree-sitter-hl-face:type.parameter tree-sitter-hl-face:type.super tree-sitter-hl-face:variable tree-sitter-hl-face:variable.builtin tree-sitter-hl-face:variable.parameter tree-sitter-hl-face:variable.special variable-pitch warning)"
    ];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_theme_setting_entries_have_exact_structural_contract() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (load-theme 'app-monochrome-themes-light-theme t)
         (mapcar
          (lambda (theme)
            (let ((settings (get theme 'theme-settings)))
              (list
               theme
               (cl-count-if
                (lambda (entry)
                  (eq (car entry) 'theme-face))
                settings)
               (cl-count-if
                (lambda (entry)
                  (eq (nth 2 entry) theme))
                settings)
               (cl-count-if
                (lambda (entry)
                  (and (= (length entry) 4)
                       (symbolp (cadr entry))
                       (listp (nth 3 entry))))
                settings))))
          '(app-monochrome-themes-dark-theme
            app-monochrome-themes-light-theme)))"##;
    let expect = expect![
        "OK ((app-monochrome-themes-dark-theme 176 176 176) (app-monochrome-themes-light-theme 167 167 167))"
    ];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_feature_and_theme_provides_are_distinct_and_complete() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (load-theme 'app-monochrome-themes-light-theme t)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (featurep symbol)
                  (custom-theme-p symbol)
                  (file-name-nondirectory
                   (or (symbol-file symbol 'feature) ""))))
          '(app-monochrome-themes
            app-monochrome-themes-dark-theme
            app-monochrome-themes-light-theme)))"##;
    let expect = expect![[
        r#"OK ((app-monochrome-themes t nil "") (app-monochrome-themes-dark-theme t #1=(app-monochrome-themes-dark-theme user changed) "") (app-monochrome-themes-light-theme t (app-monochrome-themes-light-theme . #1#) ""))"#
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}
