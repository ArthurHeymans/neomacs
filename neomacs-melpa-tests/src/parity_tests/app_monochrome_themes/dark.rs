use expect_test::expect;

use super::assert_app_monochrome_dark_parity;

#[test]
fn app_monochrome_dark_base_status_and_navigation_face_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(default dired-directory dired-flagged dired-set-id
            variable-pitch highlight italic error warning success
            bookmark-face isearch custom-link link line-number)))"##;
    let expect = expect![[
        r##"OK ((theme-face default app-monochrome-themes-dark-theme ((t (:family "UbuntuMono Nerd Font" :foundry "DAMA" :slant normal :weight regular :height 98 :width normal)))) (theme-face dired-directory app-monochrome-themes-dark-theme ((t (:weight bold)))) (theme-face dired-flagged app-monochrome-themes-dark-theme ((t (:foreground "Red" :box (:line-width (2 . 2) :color "Red" :style released-button) :weight bold)))) (theme-face dired-set-id app-monochrome-themes-dark-theme ((t (:underline t)))) (theme-face variable-pitch app-monochrome-themes-dark-theme ((t (:family "IBM Plex Serif")))) (theme-face highlight app-monochrome-themes-dark-theme ((t (:background "#bcc" :foreground "black")))) (theme-face italic app-monochrome-themes-dark-theme ((t (:slant italic :weight normal :family "IBM Plex Sans")))) (theme-face error app-monochrome-themes-dark-theme ((t (:box (:line-width (2 . 2) :color "Red" :style released-button) :weight bold)))) (theme-face warning app-monochrome-themes-dark-theme ((t (:foreground "gold")))) (theme-face success app-monochrome-themes-dark-theme ((t (:foreground "white" :weight bold)))) (theme-face bookmark-face app-monochrome-themes-dark-theme ((t (:background "Black" :foreground "Gold")))) (theme-face isearch app-monochrome-themes-dark-theme ((t (:inherit link)))) (theme-face custom-link app-monochrome-themes-dark-theme ((t (:inherit link :box (:line-width (2 . 2) :color "grey75" :style released-button))))) (theme-face link app-monochrome-themes-dark-theme ((t (:underline t :foreground "#5cacac")))) (theme-face line-number app-monochrome-themes-dark-theme ((t (:inherit shadow :family "VictorMono Nerd Font")))))"##
    ]];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_font_lock_latex_and_mode_line_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(font-lock-comment-face font-lock-type-face
            font-lock-builtin-face font-lock-function-name-face
            font-lock-keyword-face font-lock-constant-face
            font-lock-string-face font-lock-negation-char-face
            font-lock-doc-face font-lock-doc-markup-face
            font-lock-variable-name-face font-lock-preprocessor-face
            font-latex-bold-face font-latex-italic-face
            font-latex-math-face font-latex-sectioning-5-face
            font-latex-string-face font-latex-underline-face
            font-latex-verbatim-face font-latex-warning-face
            mode-line mode-line-highlight mode-line-buffer-id)))"##;
    let expect = expect![[
        r##"OK ((theme-face font-lock-comment-face app-monochrome-themes-dark-theme ((t (:foreground "#aaa")))) (theme-face font-lock-type-face app-monochrome-themes-dark-theme ((t (:weight bold :family "VictorMono Nerd Font")))) (theme-face font-lock-builtin-face app-monochrome-themes-dark-theme ((t (:family "Linux Libertine Mono" :background "grey16")))) (theme-face font-lock-function-name-face app-monochrome-themes-dark-theme ((t (:slant italic :family "IBM Plex Mono")))) (theme-face font-lock-keyword-face app-monochrome-themes-dark-theme ((t (:weight bold :family "Ubuntu Mono")))) (theme-face font-lock-constant-face app-monochrome-themes-dark-theme ((t (:foreground "white" :weight bold :inherit font-lock-type-face)))) (theme-face font-lock-string-face app-monochrome-themes-dark-theme ((t (:foreground "grey62" :family "IBM Plex Mono")))) (theme-face font-lock-negation-char-face app-monochrome-themes-dark-theme ((t (:weight bold)))) (theme-face font-lock-doc-face app-monochrome-themes-dark-theme ((t (:slant italic :inherit font-lock-string-face)))) (theme-face font-lock-doc-markup-face app-monochrome-themes-dark-theme ((t (:inherit (font-lock-constant-face))))) (theme-face font-lock-variable-name-face app-monochrome-themes-dark-theme ((t (:foreground "white" :weight thin)))) (theme-face font-lock-preprocessor-face app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face font-latex-bold-face app-monochrome-themes-dark-theme ((t (:inherit bold)))) (theme-face font-latex-italic-face app-monochrome-themes-dark-theme ((t (:inherit italic)))) (theme-face font-latex-math-face app-monochrome-themes-dark-theme ((t (:family "IBM Plex Mono")))) (theme-face font-latex-sectioning-5-face app-monochrome-themes-dark-theme ((t (:inherit org-level-5 :weight bold)))) (theme-face font-latex-string-face app-monochrome-themes-dark-theme ((t (:inherit font-lock-string-face)))) (theme-face font-latex-underline-face app-monochrome-themes-dark-theme ((t (:inherit underline)))) (theme-face font-latex-verbatim-face app-monochrome-themes-dark-theme ((t (:inherit fixed-pitch)))) (theme-face font-latex-warning-face app-monochrome-themes-dark-theme ((t (:inherit warning)))) (theme-face mode-line app-monochrome-themes-dark-theme ((t (:inherit line-number :box t)))) (theme-face mode-line-highlight app-monochrome-themes-dark-theme ((t (:family "Linux Libertine Mono" :inherit mode-line :weight bold)))) (theme-face mode-line-buffer-id app-monochrome-themes-dark-theme ((t (:inherit mode-line-highlight)))))"##
    ]];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_search_completion_and_pairing_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(counsel-outline-1 counsel-application-name
            counsel-active-mode counsel-variable-documentation
            swiper-background-match-face-1 swiper-match-face-1
            lazy-highlight swiper-background-match-face-3
            swiper-line-face ivy-match-required-face ivy-cursor
            ivy-virtual ivy-action ivy-prompt-match
            ivy-grep-line-number minibuffer-prompt
            orderless-match-face-0 orderless-match-face-1
            orderless-match-face-2 orderless-match-face-3
            completions-common-part sp-show-pair-match-content-face
            show-paren-match show-paren-mismatch
            trailing-whitespace)))"##;
    let expect = expect![[
        r##"OK ((theme-face counsel-outline-1 app-monochrome-themes-dark-theme ((t (:inherit (org-level-1))))) (theme-face counsel-application-name app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face counsel-active-mode app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face counsel-variable-documentation app-monochrome-themes-dark-theme ((t (:inherit (font-lock-comment-face))))) (theme-face swiper-background-match-face-1 app-monochrome-themes-dark-theme ((t (:inherit (swiper-match-face-1))))) (theme-face swiper-match-face-1 app-monochrome-themes-dark-theme ((t (:inherit (lazy-highlight))))) (theme-face lazy-highlight app-monochrome-themes-dark-theme ((t (:distant-foreground "white" :box (:line-width (2 . 2) :color "grey75" :style released-button))))) (theme-face swiper-background-match-face-3 app-monochrome-themes-dark-theme ((t (:inherit (swiper-match-face-3))))) (theme-face swiper-line-face app-monochrome-themes-dark-theme ((t (:inherit highlight :box (:line-width (2 . 2) :color "grey75" :style released-button))))) (theme-face ivy-match-required-face app-monochrome-themes-dark-theme ((t (:foreground "red" :inherit (minibuffer-prompt))))) (theme-face ivy-cursor app-monochrome-themes-dark-theme ((((class color) (background light)) (:foreground "white" :background "black")) (((class color) (background dark)) (:foreground "black" :background "white")))) (theme-face ivy-virtual app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face ivy-action app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face ivy-prompt-match app-monochrome-themes-dark-theme ((t (:inherit (ivy-current-match))))) (theme-face ivy-grep-line-number app-monochrome-themes-dark-theme ((t (:inherit (compilation-line-number))))) (theme-face minibuffer-prompt app-monochrome-themes-dark-theme ((t (:inherit default :weight bold :slant italic :box t)))) (theme-face orderless-match-face-0 app-monochrome-themes-dark-theme ((t (:background "#8cc")))) (theme-face orderless-match-face-1 app-monochrome-themes-dark-theme ((t (:slant italic :inherit orderless-match-face-0)))) (theme-face orderless-match-face-2 app-monochrome-themes-dark-theme ((t (:inherit link :weight bold)))) (theme-face orderless-match-face-3 app-monochrome-themes-dark-theme ((t (:inherit link :underline t)))) (theme-face completions-common-part app-monochrome-themes-dark-theme ((t (:inherit error)))) (theme-face sp-show-pair-match-content-face app-monochrome-themes-dark-theme ((t nil))) (theme-face show-paren-match app-monochrome-themes-dark-theme ((t (:inherit default :background "#8cc")))) (theme-face show-paren-mismatch app-monochrome-themes-dark-theme ((t (:inherit show-paren-match :inverse-video t)))) (theme-face trailing-whitespace app-monochrome-themes-dark-theme ((t (:underline "blue")))))"##
    ]];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_lsp_and_diagnostics_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(lsp-face-semhl-function lsp-face-highlight-textual
            lsp-face-highlight-write lsp-face-semhl-type
            lsp-face-semhl-implementation lsp-ui-doc-background
            lsp-ui-doc-highlight-hover lsp-ui-sideline-global
            lsp-ui-sideline-symbol lsp-ui-peek-list
            lsp-ui-peek-line-number flymake-error
            flycheck-fringe-warning flycheck-info
            doom-modeline-lsp-success doom-modeline-buffer-file
            lsp-rust-analyzer-inlay-param-face
            lsp-rust-analyzer-inlay-type-face
            lsp-rust-analyzer-inlay-face rust-unsafe)))"##;
    let expect = expect![[
        r##"OK ((theme-face lsp-face-semhl-function app-monochrome-themes-dark-theme ((t (:inherit (font-lock-function-name-face))))) (theme-face lsp-face-highlight-textual app-monochrome-themes-dark-theme ((t (:inherit (highlight))))) (theme-face lsp-face-highlight-write app-monochrome-themes-dark-theme ((t (:weight bold :inherit (highlight))))) (theme-face lsp-face-semhl-type app-monochrome-themes-dark-theme ((t (:inherit (font-lock-type-face))))) (theme-face lsp-face-semhl-implementation app-monochrome-themes-dark-theme ((t (:weight bold :inherit (font-lock-function-name-face))))) (theme-face lsp-ui-doc-background app-monochrome-themes-dark-theme ((t (:background "grey90")))) (theme-face lsp-ui-doc-highlight-hover app-monochrome-themes-dark-theme ((t (:inherit highlight)))) (theme-face lsp-ui-sideline-global app-monochrome-themes-dark-theme ((t nil))) (theme-face lsp-ui-sideline-symbol app-monochrome-themes-dark-theme ((t (:height 0.99 :box (:line-width (1 . -1) :color "grey" :style nil) :foreground "grey")))) (theme-face lsp-ui-peek-list app-monochrome-themes-dark-theme ((((background light)) (:background "light gray")) (t (:background "#181818")))) (theme-face lsp-ui-peek-line-number app-monochrome-themes-dark-theme ((t (:foreground "grey25")))) (theme-face flymake-error app-monochrome-themes-dark-theme ((t (:underline (:color "Red1" :style dashes :position nil))))) (theme-face flycheck-fringe-warning app-monochrome-themes-dark-theme ((t (:inherit (warning))))) (theme-face flycheck-info app-monochrome-themes-dark-theme ((t (:inherit success :underline t)))) (theme-face doom-modeline-lsp-success app-monochrome-themes-dark-theme ((t (:inherit nil :weight bold)))) (theme-face doom-modeline-buffer-file app-monochrome-themes-dark-theme ((t (:inherit (mode-line-buffer-id bold))))) (theme-face lsp-rust-analyzer-inlay-param-face app-monochrome-themes-dark-theme ((t (:inherit (lsp-rust-analyzer-inlay-face))))) (theme-face lsp-rust-analyzer-inlay-type-face app-monochrome-themes-dark-theme ((t (:inherit (lsp-rust-analyzer-inlay-face))))) (theme-face lsp-rust-analyzer-inlay-face app-monochrome-themes-dark-theme ((t (:inherit font-lock-comment-face :foreground "black" :background "white")))) (theme-face rust-unsafe app-monochrome-themes-dark-theme ((t (:weight bold)))))"##
    ]];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_magit_git_and_compilation_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(compilation-line-number fancy-compilation-default-face
            magit-filename magit-keyword-squash magit-blame-name
            magit-diff-base-highlight magit-keyword magit-tag
            magit-branch-upstream magit-branch-remote
            magit-branch-local magit-branch-current
            magit-section-heading-selection
            magit-section-secondary-heading magit-section-heading
            magit-section-highlight magit-section-child-count
            magit-diff-file-heading git-commit-keyword
            git-commit-summary)))"##;
    let expect = expect![[
        r##"OK ((theme-face compilation-line-number app-monochrome-themes-dark-theme ((t (:inherit (font-lock-keyword-face))))) (theme-face fancy-compilation-default-face app-monochrome-themes-dark-theme ((t (:inherit font-lock-string-face)))) (theme-face magit-filename app-monochrome-themes-dark-theme ((t (:weight normal)))) (theme-face magit-keyword-squash app-monochrome-themes-dark-theme ((t (:inherit (font-lock-warning-face))))) (theme-face magit-blame-name app-monochrome-themes-dark-theme ((t nil))) (theme-face magit-diff-base-highlight app-monochrome-themes-dark-theme ((((class color) (background light)) (:foreground "#aaaa11" :background "#eeeebb" :extend t)) (((class color) (background dark)) (:foreground "#eeeebb" :background "#666622" :extend t)))) (theme-face magit-keyword app-monochrome-themes-dark-theme ((t (:inherit (font-lock-string-face))))) (theme-face magit-tag app-monochrome-themes-dark-theme ((t (:inherit font-lock-constant-face)))) (theme-face magit-branch-upstream app-monochrome-themes-dark-theme ((t (:slant italic)))) (theme-face magit-branch-remote app-monochrome-themes-dark-theme ((t (:foreground "white" :weight bold :height 1.25 :family "IBM Plex Serif")))) (theme-face magit-branch-local app-monochrome-themes-dark-theme ((t (:weight bold :height 1.25 :family "IBM Plex Serif")))) (theme-face magit-branch-current app-monochrome-themes-dark-theme ((t (:box (:line-width (1 . 1) :color nil :style nil) :inherit (magit-branch-local))))) (theme-face magit-section-heading-selection app-monochrome-themes-dark-theme ((t (:extend t :weight bold :height 1.25)))) (theme-face magit-section-secondary-heading app-monochrome-themes-dark-theme ((t (:weight bold :extend t)))) (theme-face magit-section-heading app-monochrome-themes-dark-theme ((t (:extend t :weight bold :height 2.0 :inherit font-lock-constant-face)))) (theme-face magit-section-highlight app-monochrome-themes-dark-theme ((((class color) (background light)) (:background "grey95" :extend t)) (((class color) (background dark)) (:background "grey20" :extend t)))) (theme-face magit-section-child-count app-monochrome-themes-dark-theme ((t nil))) (theme-face magit-diff-file-heading app-monochrome-themes-dark-theme ((t (:extend t :weight bold :family "IBM Plex Serif")))) (theme-face git-commit-keyword app-monochrome-themes-dark-theme ((t (:inherit font-lock-keyword-face)))) (theme-face git-commit-summary app-monochrome-themes-dark-theme ((t (:inherit default)))))"##
    ]];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_tree_sitter_compatibility_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(tree-sitter-hl-face:constructor
            tree-sitter-hl-face:property.definition
            tree-sitter-hl-face:number tree-sitter-hl-face:method
            tree-sitter-hl-face:function.call
            tree-sitter-hl-face:operator tree-sitter-hl-face:type
            tree-sitter-hl-face:label tree-sitter-hl-face:function
            tree-sitter-hl-face:type.builtin
            tree-sitter-hl-face:method.call
            tree-sitter-hl-face:variable.parameter
            tree-sitter-hl-face:function.special
            tree-sitter-hl-face:doc tree-sitter-hl-face:embedded
            tree-sitter-hl-face:variable
            tree-sitter-hl-face:variable.special
            tree-sitter-hl-face:variable.builtin
            tree-sitter-hl-face:constant tree-sitter-hl-face:escape
            tree-sitter-hl-face:punctuation.delimiter
            tree-sitter-hl-face:string.special
            tree-sitter-hl-face:punctuation
            tree-sitter-hl-face:constant.builtin
            tree-sitter-hl-face:type.parameter
            tree-sitter-hl-face:type.super
            tree-sitter-hl-face:type.argument
            tree-sitter-hl-face:property
            tree-sitter-hl-face:attribute)))"##;
    let expect = expect![
        "OK ((theme-face tree-sitter-hl-face:constructor app-monochrome-themes-dark-theme ((t (:inherit tree-sitter-hl-face:constant)))) (theme-face tree-sitter-hl-face:property.definition app-monochrome-themes-dark-theme ((t (:inherit tree-sitter-hl-face:variable.parameter)))) (theme-face tree-sitter-hl-face:number app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:constant))))) (theme-face tree-sitter-hl-face:method app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:function))))) (theme-face tree-sitter-hl-face:function.call app-monochrome-themes-dark-theme ((t (:inherit font-lock-function-name-face)))) (theme-face tree-sitter-hl-face:operator app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:keyword))))) (theme-face tree-sitter-hl-face:type app-monochrome-themes-dark-theme ((t (:inherit font-lock-type-face)))) (theme-face tree-sitter-hl-face:label app-monochrome-themes-dark-theme ((t (:inherit (font-lock-preprocessor-face))))) (theme-face tree-sitter-hl-face:function app-monochrome-themes-dark-theme ((t (:inherit (font-lock-function-name-face))))) (theme-face tree-sitter-hl-face:type.builtin app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face tree-sitter-hl-face:method.call app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:function.call))))) (theme-face tree-sitter-hl-face:variable.parameter app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:variable))))) (theme-face tree-sitter-hl-face:function.special app-monochrome-themes-dark-theme ((t (:inherit (font-lock-preprocessor-face))))) (theme-face tree-sitter-hl-face:doc app-monochrome-themes-dark-theme ((t (:inherit (font-lock-doc-face))))) (theme-face tree-sitter-hl-face:embedded app-monochrome-themes-dark-theme ((t (:inherit (default))))) (theme-face tree-sitter-hl-face:variable app-monochrome-themes-dark-theme ((t (:inherit (font-lock-variable-name-face))))) (theme-face tree-sitter-hl-face:variable.special app-monochrome-themes-dark-theme ((t (:inherit (font-lock-warning-face))))) (theme-face tree-sitter-hl-face:variable.builtin app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face tree-sitter-hl-face:constant app-monochrome-themes-dark-theme ((t (:inherit font-lock-constant-face)))) (theme-face tree-sitter-hl-face:escape app-monochrome-themes-dark-theme ((t (:inherit (font-lock-keyword-face))))) (theme-face tree-sitter-hl-face:punctuation.delimiter app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:punctuation))))) (theme-face tree-sitter-hl-face:string.special app-monochrome-themes-dark-theme ((t (:weight bold :inherit (tree-sitter-hl-face:string))))) (theme-face tree-sitter-hl-face:punctuation app-monochrome-themes-dark-theme ((t (:inherit (default))))) (theme-face tree-sitter-hl-face:constant.builtin app-monochrome-themes-dark-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face tree-sitter-hl-face:type.parameter app-monochrome-themes-dark-theme ((t (:inherit (font-lock-variable-name-face font-lock-type-face) :slant italic :weight bold)))) (theme-face tree-sitter-hl-face:type.super app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:type))))) (theme-face tree-sitter-hl-face:type.argument app-monochrome-themes-dark-theme ((t (:inherit (tree-sitter-hl-face:type))))) (theme-face tree-sitter-hl-face:property app-monochrome-themes-dark-theme ((t (:inherit nil :slant italic)))) (theme-face tree-sitter-hl-face:attribute app-monochrome-themes-dark-theme ((t (:inherit default)))))"
    ];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_org_markdown_and_delimiter_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-dark-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(markdown-header-face markdown-header-face-1
            markdown-header-face-2 markdown-header-face-3
            markdown-header-face-4 markdown-header-face-5
            markdown-header-face-6 markdown-inline-code-face
            org-default org-level-1 org-level-2 org-level-3
            org-level-4 org-level-5 org-level-6 org-level-7
            org-level-8 org-code org-verbatim org-done org-todo
            org-headline-todo org-headline-done org-table
            org-modern-done org-modern-todo
            rainbow-delimiters-depth-1-face
            rainbow-delimiters-depth-2-face
            rainbow-delimiters-depth-3-face
            rainbow-delimiters-depth-4-face
            rainbow-delimiters-depth-5-face
            rainbow-delimiters-depth-6-face
            rainbow-delimiters-depth-7-face
            rainbow-delimiters-depth-8-face
            rainbow-delimiters-depth-9-face)))"##;
    let expect = expect![[
        r##"OK ((theme-face markdown-header-face app-monochrome-themes-dark-theme ((t (:weight bold :height 1.2 :family "Ubuntu")))) (theme-face markdown-header-face-1 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face :height 0.8)))) (theme-face markdown-header-face-2 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-1 :height 0.8)))) (theme-face markdown-header-face-3 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-2 :height 0.8)))) (theme-face markdown-header-face-4 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-3 :height 0.8)))) (theme-face markdown-header-face-5 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-4 :height 0.8)))) (theme-face markdown-header-face-6 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-4 :height 0.8)))) (theme-face markdown-inline-code-face app-monochrome-themes-dark-theme ((t (:inherit org-code)))) (theme-face org-default app-monochrome-themes-dark-theme ((t (:inherit (default))))) (theme-face org-level-1 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face)))) (theme-face org-level-2 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-1)))) (theme-face org-level-3 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-2)))) (theme-face org-level-4 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-3)))) (theme-face org-level-5 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-4)))) (theme-face org-level-6 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-4 :height 0.85)))) (theme-face org-level-7 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-4 :height 0.75)))) (theme-face org-level-8 app-monochrome-themes-dark-theme ((t (:inherit markdown-header-face-4 :height 0.65)))) (theme-face org-code app-monochrome-themes-dark-theme ((t (:family "VictorMono Nerd Font" :foreground "grey75" :background "grey10")))) (theme-face org-verbatim app-monochrome-themes-dark-theme ((t (:inherit org-code :weight light)))) (theme-face org-done app-monochrome-themes-dark-theme ((t (:inherit org-headline :foreground "white" :background "#048" :box t)))) (theme-face org-todo app-monochrome-themes-dark-theme ((t (:foreground "white" :background "#800" :box t)))) (theme-face org-headline-todo app-monochrome-themes-dark-theme ((t (:inherit org-headline)))) (theme-face org-headline-done app-monochrome-themes-dark-theme ((t (:inherit org-headline)))) (theme-face org-table app-monochrome-themes-dark-theme ((t (:foreground "white" :background "black" :family "VictorMono Nerd Font")))) (theme-face org-modern-done app-monochrome-themes-dark-theme ((t (:inherit org-done :height 2.0)))) (theme-face org-modern-todo app-monochrome-themes-dark-theme ((t (:inherit org-todo :height 1.0)))) (theme-face rainbow-delimiters-depth-1-face app-monochrome-themes-dark-theme ((t (:inherit rainbow-delimiters-base-face)))) (theme-face rainbow-delimiters-depth-2-face app-monochrome-themes-dark-theme ((t (:weight bold :inherit rainbow-delimiters-base-face)))) (theme-face rainbow-delimiters-depth-3-face app-monochrome-themes-dark-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#fda")))) (theme-face rainbow-delimiters-depth-4-face app-monochrome-themes-dark-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#7fd")))) (theme-face rainbow-delimiters-depth-5-face app-monochrome-themes-dark-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#f0a")))) (theme-face rainbow-delimiters-depth-6-face app-monochrome-themes-dark-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#acf")))) (theme-face rainbow-delimiters-depth-7-face app-monochrome-themes-dark-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#faa")))) (theme-face rainbow-delimiters-depth-8-face app-monochrome-themes-dark-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#0af")))) (theme-face rainbow-delimiters-depth-9-face app-monochrome-themes-dark-theme ((t (:weight bold :inherit rainbow-delimiters-base-face :foreground "#afa")))))"##
    ]];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_dark_duplicate_precedence_and_enabled_faces_are_exact() {
    let elisp_form = r##"(let* ((theme
                'app-monochrome-themes-dark-theme)
              (settings (get theme 'theme-settings))
              (duplicates
               (mapcar
                (lambda (face)
                  (list
                   face
                   (mapcar
                    (lambda (entry) (nth 3 entry))
                    (cl-remove-if-not
                     (lambda (entry) (eq (cadr entry) face))
                     settings))))
                '(lsp-ui-doc-background markdown-header-face
                  orderless-match-face-0
                  orderless-match-face-1))))
         (enable-theme theme)
         (list
          duplicates
          custom-enabled-themes
          (mapcar
           (lambda (request)
             (list (car request)
                   (cadr request)
                   (face-attribute
                    (car request) (cadr request)
                    nil 'default)))
           '((default :family) (default :height)
             (font-lock-comment-face :foreground)
             (font-lock-keyword-face :weight)
             (link :foreground) (link :underline)
             (error :weight) (mode-line :box)))))"##;
    let expect = expect![[
        r##"OK (((lsp-ui-doc-background (((t (:background "grey90"))) ((t (:background "grey10"))))) (markdown-header-face (((t (:weight bold :height 1.2 :family "Ubuntu"))) ((t (:weight bold :inherit magit-section-heading :height 1.5))))) (orderless-match-face-0 (((t (:background "#8cc"))) ((t (:inherit error :weight bold))))) (orderless-match-face-1 (((t (:slant italic :inherit orderless-match-face-0))) ((t (:inherit error :underline t)))))) (app-monochrome-themes-dark-theme) ((default :family "UbuntuMono Nerd Font") (default :height 98) (font-lock-comment-face :foreground "#aaa") (font-lock-keyword-face :weight bold) (link :foreground "#5cacac") (link :underline t) (error :weight bold) (mode-line :box 1)))"##
    ]];
    assert_app_monochrome_dark_parity(elisp_form, expect);
}
