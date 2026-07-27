use expect_test::expect;

use super::assert_app_monochrome_light_parity;

#[test]
fn app_monochrome_light_base_status_and_navigation_face_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-light-theme
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
        r##"OK ((theme-face default app-monochrome-themes-light-theme ((t (:inherit nil :extend nil :stipple nil :background "white" :foreground "grey12" :inverse-video nil :box nil :strike-through nil :overline nil :underline nil :slant normal :weight regular :height 98 :width normal :foundry "ADBO" :family "Fira Code")))) (theme-face dired-directory app-monochrome-themes-light-theme ((t (:weight bold)))) (theme-face dired-flagged app-monochrome-themes-light-theme ((t (:foreground "Red" :box (:line-width (2 . 2) :color "Red" :style released-button) :weight bold)))) (theme-face dired-set-id app-monochrome-themes-light-theme ((t (:underline t)))) (theme-face variable-pitch app-monochrome-themes-light-theme ((t (:family "IBM Plex Serif")))) (theme-face highlight app-monochrome-themes-light-theme ((t (:background "#bcc" :foreground "black")))) (theme-face italic app-monochrome-themes-light-theme ((t (:slant italic :weight normal :family "IBM Plex Sans")))) (theme-face error app-monochrome-themes-light-theme ((t (:background "#fbb" :foreground "black" :box (:line-width (2 . 2) :color "Black" :style flat-button))))) (theme-face warning app-monochrome-themes-light-theme ((t (:foreground "red4")))) (theme-face success app-monochrome-themes-light-theme ((t (:foreground "black" :weight bold)))) (theme-face bookmark-face app-monochrome-themes-light-theme ((t (:background "black" :foreground "Gold")))) (theme-face isearch app-monochrome-themes-light-theme ((t (:inherit link)))) (theme-face custom-link app-monochrome-themes-light-theme ((t (:inherit link :box (:line-width (2 . 2) :color "grey75" :style released-button))))) (theme-face link app-monochrome-themes-light-theme ((t (:underline t :foreground "#3c5c5c")))) (theme-face line-number app-monochrome-themes-light-theme ((t (:inherit shadow :family "VictorMono Nerd Font")))))"##
    ]];
    assert_app_monochrome_light_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_font_lock_and_mode_line_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-light-theme
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
            mode-line mode-line-highlight mode-line-buffer-id)))"##;
    let expect = expect![[
        r##"OK ((theme-face font-lock-comment-face app-monochrome-themes-light-theme ((t (:foreground "#888")))) (theme-face font-lock-type-face app-monochrome-themes-light-theme ((t (:weight bold :family "VictorMono Nerd Font")))) (theme-face font-lock-builtin-face app-monochrome-themes-light-theme ((t (:family "Linux Libertine Mono" :weight bold :foreground "grey47" :background "white")))) (theme-face font-lock-function-name-face app-monochrome-themes-light-theme ((t (:slant italic :family "IBM Plex Mono")))) (theme-face font-lock-keyword-face app-monochrome-themes-light-theme ((t (:weight bold :family "Ubuntu Mono")))) (theme-face font-lock-constant-face app-monochrome-themes-light-theme ((t (:foreground "black" :weight bold :inherit font-lock-type-face)))) (theme-face font-lock-string-face app-monochrome-themes-light-theme ((t (:foreground "grey40" :family "IBM Plex Mono")))) (theme-face font-lock-negation-char-face app-monochrome-themes-light-theme ((t (:weight bold)))) (theme-face font-lock-doc-face app-monochrome-themes-light-theme ((t (:slant italic :inherit font-lock-string-face)))) (theme-face font-lock-doc-markup-face app-monochrome-themes-light-theme ((t (:inherit (font-lock-constant-face))))) (theme-face font-lock-variable-name-face app-monochrome-themes-light-theme ((t (:foreground "black" :weight thin)))) (theme-face font-lock-preprocessor-face app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face mode-line app-monochrome-themes-light-theme ((t (:inherit line-number :box t)))) (theme-face mode-line-highlight app-monochrome-themes-light-theme ((t (:family "Linux Libertine Mono" :inherit mode-line :weight bold)))) (theme-face mode-line-buffer-id app-monochrome-themes-light-theme ((t (:inherit mode-line-highlight)))))"##
    ]];
    assert_app_monochrome_light_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_search_completion_and_pairing_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-light-theme
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
        r##"OK ((theme-face counsel-outline-1 app-monochrome-themes-light-theme ((t (:inherit (org-level-1))))) (theme-face counsel-application-name app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face counsel-active-mode app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face counsel-variable-documentation app-monochrome-themes-light-theme ((t (:inherit (font-lock-comment-face))))) (theme-face swiper-background-match-face-1 app-monochrome-themes-light-theme ((t (:inherit (swiper-match-face-1))))) (theme-face swiper-match-face-1 app-monochrome-themes-light-theme ((t (:inherit (lazy-highlight))))) (theme-face lazy-highlight app-monochrome-themes-light-theme ((t (:distant-foreground "black" :box (:line-width (2 . 2) :color "grey25" :style released-button))))) (theme-face swiper-background-match-face-3 app-monochrome-themes-light-theme ((t (:inherit (swiper-match-face-3))))) (theme-face swiper-line-face app-monochrome-themes-light-theme ((t (:inherit highlight :box (:line-width (2 . 2) :color "grey20" :style released-button))))) (theme-face ivy-match-required-face app-monochrome-themes-light-theme ((t (:foreground "red" :inherit (minibuffer-prompt))))) (theme-face ivy-cursor app-monochrome-themes-light-theme ((((class color) (background light)) (:foreground "white" :background "black")) (((class color) (background dark)) (:foreground "white" :background "white")))) (theme-face ivy-virtual app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face ivy-action app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face ivy-prompt-match app-monochrome-themes-light-theme ((t (:inherit (ivy-current-match))))) (theme-face ivy-grep-line-number app-monochrome-themes-light-theme ((t (:inherit (compilation-line-number))))) (theme-face minibuffer-prompt app-monochrome-themes-light-theme ((t (:inherit default :weight bold :slant italic :box t)))) (theme-face orderless-match-face-0 app-monochrome-themes-light-theme ((t (:background "#8cc")))) (theme-face orderless-match-face-1 app-monochrome-themes-light-theme ((t (:background "#bbf" :underline t)))) (theme-face orderless-match-face-2 app-monochrome-themes-light-theme ((t (:inherit link :weight bold)))) (theme-face orderless-match-face-3 app-monochrome-themes-light-theme ((t (:inherit link :underline t)))) (theme-face completions-common-part app-monochrome-themes-light-theme ((t (:inherit error)))) (theme-face sp-show-pair-match-content-face app-monochrome-themes-light-theme ((t nil))) (theme-face show-paren-match app-monochrome-themes-light-theme ((t (:inherit default :background "#8cc")))) (theme-face show-paren-mismatch app-monochrome-themes-light-theme ((t (:inherit show-paren-match :inverse-video t)))) (theme-face trailing-whitespace app-monochrome-themes-light-theme ((t (:underline "blue")))))"##
    ]];
    assert_app_monochrome_light_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_lsp_and_diagnostics_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-light-theme
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
            lsp-ui-peek-line-number flymake-error flycheck-info
            flycheck-warning flycheck-error
            flycheck-fringe-warning doom-modeline-lsp-success
            doom-modeline-buffer-file
            lsp-rust-analyzer-inlay-param-face
            lsp-rust-analyzer-inlay-type-face
            lsp-rust-analyzer-inlay-face rust-unsafe)))"##;
    let expect = expect![[
        r##"OK ((theme-face lsp-face-semhl-function app-monochrome-themes-light-theme ((t (:inherit (font-lock-function-name-face))))) (theme-face lsp-face-highlight-textual app-monochrome-themes-light-theme ((t (:inherit (highlight))))) (theme-face lsp-face-highlight-write app-monochrome-themes-light-theme ((t (:weight bold :inherit (highlight))))) (theme-face lsp-face-semhl-type app-monochrome-themes-light-theme ((t (:inherit (font-lock-type-face))))) (theme-face lsp-face-semhl-implementation app-monochrome-themes-light-theme ((t (:weight bold :inherit (font-lock-function-name-face))))) (theme-face lsp-ui-doc-background app-monochrome-themes-light-theme ((t (:background "grey90")))) (theme-face lsp-ui-doc-highlight-hover app-monochrome-themes-light-theme ((t (:inherit highlight)))) (theme-face lsp-ui-sideline-global app-monochrome-themes-light-theme ((t nil))) (theme-face lsp-ui-sideline-symbol app-monochrome-themes-light-theme ((t (:height 0.99 :box (:line-width (1 . -1) :color "grey20" :style nil) :foreground "grey20")))) (theme-face lsp-ui-peek-list app-monochrome-themes-light-theme ((((background light)) (:background "dark grey")) (t (:background "#f8f8f8")))) (theme-face lsp-ui-peek-line-number app-monochrome-themes-light-theme ((t (:foreground "grey25")))) (theme-face flymake-error app-monochrome-themes-light-theme ((t (:underline (:color "Red1" :style dashes :position nil))))) (theme-face flycheck-info app-monochrome-themes-light-theme ((t (:inherit success :underline t)))) (theme-face flycheck-warning app-monochrome-themes-light-theme ((t (:underline "dark red")))) (theme-face flycheck-error app-monochrome-themes-light-theme ((t (:background "#fbb" :box (:line-width (2 . 2) :color "Red" :style flat-button))))) (theme-face flycheck-fringe-warning app-monochrome-themes-light-theme ((t (:inherit (warning))))) (theme-face doom-modeline-lsp-success app-monochrome-themes-light-theme ((t (:inherit nil :weight bold)))) (theme-face doom-modeline-buffer-file app-monochrome-themes-light-theme ((t (:inherit (mode-line-buffer-id bold))))) (theme-face lsp-rust-analyzer-inlay-param-face app-monochrome-themes-light-theme ((t (:inherit (lsp-rust-analyzer-inlay-face))))) (theme-face lsp-rust-analyzer-inlay-type-face app-monochrome-themes-light-theme ((t (:inherit (lsp-rust-analyzer-inlay-face))))) (theme-face lsp-rust-analyzer-inlay-face app-monochrome-themes-light-theme ((t (:inherit font-lock-comment-face :foreground "grey30" :background "white")))) (theme-face rust-unsafe app-monochrome-themes-light-theme ((t (:weight bold)))))"##
    ]];
    assert_app_monochrome_light_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_magit_git_and_compilation_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-light-theme
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
        r##"OK ((theme-face compilation-line-number app-monochrome-themes-light-theme ((t (:inherit (font-lock-keyword-face))))) (theme-face fancy-compilation-default-face app-monochrome-themes-light-theme ((t (:inherit default)))) (theme-face magit-filename app-monochrome-themes-light-theme ((t (:weight normal)))) (theme-face magit-keyword-squash app-monochrome-themes-light-theme ((t (:inherit (font-lock-warning-face))))) (theme-face magit-blame-name app-monochrome-themes-light-theme ((t nil))) (theme-face magit-diff-base-highlight app-monochrome-themes-light-theme ((((class color) (background light)) (:foreground "#aaaa11" :background "#eeeebb" :extend t)) (((class color) (background dark)) (:foreground "#eeeebb" :background "#666622" :extend t)))) (theme-face magit-keyword app-monochrome-themes-light-theme ((t (:inherit (font-lock-string-face))))) (theme-face magit-tag app-monochrome-themes-light-theme ((t (:inherit font-lock-constant-face)))) (theme-face magit-branch-upstream app-monochrome-themes-light-theme ((t (:slant italic)))) (theme-face magit-branch-remote app-monochrome-themes-light-theme ((t (:foreground "black" :weight bold :height 1.25 :family "IBM Plex Serif")))) (theme-face magit-branch-local app-monochrome-themes-light-theme ((t (:weight bold :height 1.25 :family "IBM Plex Serif")))) (theme-face magit-branch-current app-monochrome-themes-light-theme ((t (:box (:line-width (1 . 1) :color nil :style nil) :inherit (magit-branch-local))))) (theme-face magit-section-heading-selection app-monochrome-themes-light-theme ((t (:extend t :weight bold :height 1.25)))) (theme-face magit-section-secondary-heading app-monochrome-themes-light-theme ((t (:weight bold :extend t)))) (theme-face magit-section-heading app-monochrome-themes-light-theme ((t (:extend t :weight bold :height 2.0 :inherit font-lock-constant-face)))) (theme-face magit-section-highlight app-monochrome-themes-light-theme ((((class color) (background light)) (:background "grey80" :extend t)) (((class color) (background dark)) (:background "grey44" :extend t)))) (theme-face magit-section-child-count app-monochrome-themes-light-theme ((t nil))) (theme-face magit-diff-file-heading app-monochrome-themes-light-theme ((t (:extend t :weight bold :family "IBM Plex Serif")))) (theme-face git-commit-keyword app-monochrome-themes-light-theme ((t (:inherit font-lock-keyword-face)))) (theme-face git-commit-summary app-monochrome-themes-light-theme ((t (:inherit default)))))"##
    ]];
    assert_app_monochrome_light_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_tree_sitter_compatibility_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-light-theme
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
        "OK ((theme-face tree-sitter-hl-face:constructor app-monochrome-themes-light-theme ((t (:inherit tree-sitter-hl-face:constant)))) (theme-face tree-sitter-hl-face:property.definition app-monochrome-themes-light-theme ((t (:inherit tree-sitter-hl-face:variable.parameter)))) (theme-face tree-sitter-hl-face:number app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:constant))))) (theme-face tree-sitter-hl-face:method app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:function))))) (theme-face tree-sitter-hl-face:function.call app-monochrome-themes-light-theme ((t (:inherit font-lock-function-name-face)))) (theme-face tree-sitter-hl-face:operator app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:keyword))))) (theme-face tree-sitter-hl-face:type app-monochrome-themes-light-theme ((t (:inherit font-lock-type-face)))) (theme-face tree-sitter-hl-face:label app-monochrome-themes-light-theme ((t (:inherit (font-lock-preprocessor-face))))) (theme-face tree-sitter-hl-face:function app-monochrome-themes-light-theme ((t (:inherit (font-lock-function-name-face))))) (theme-face tree-sitter-hl-face:type.builtin app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face tree-sitter-hl-face:method.call app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:function.call))))) (theme-face tree-sitter-hl-face:variable.parameter app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:variable))))) (theme-face tree-sitter-hl-face:function.special app-monochrome-themes-light-theme ((t (:inherit (font-lock-preprocessor-face))))) (theme-face tree-sitter-hl-face:doc app-monochrome-themes-light-theme ((t (:inherit (font-lock-doc-face))))) (theme-face tree-sitter-hl-face:embedded app-monochrome-themes-light-theme ((t (:inherit (default))))) (theme-face tree-sitter-hl-face:variable app-monochrome-themes-light-theme ((t (:inherit (font-lock-variable-name-face))))) (theme-face tree-sitter-hl-face:variable.special app-monochrome-themes-light-theme ((t (:inherit (font-lock-warning-face))))) (theme-face tree-sitter-hl-face:variable.builtin app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face tree-sitter-hl-face:constant app-monochrome-themes-light-theme ((t (:inherit font-lock-constant-face)))) (theme-face tree-sitter-hl-face:escape app-monochrome-themes-light-theme ((t (:inherit (font-lock-keyword-face))))) (theme-face tree-sitter-hl-face:punctuation.delimiter app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:punctuation))))) (theme-face tree-sitter-hl-face:string.special app-monochrome-themes-light-theme ((t (:weight bold :inherit (tree-sitter-hl-face:string))))) (theme-face tree-sitter-hl-face:punctuation app-monochrome-themes-light-theme ((t (:inherit (default))))) (theme-face tree-sitter-hl-face:constant.builtin app-monochrome-themes-light-theme ((t (:inherit (font-lock-builtin-face))))) (theme-face tree-sitter-hl-face:type.parameter app-monochrome-themes-light-theme ((t (:inherit (font-lock-variable-name-face font-lock-type-face) :slant italic :weight bold)))) (theme-face tree-sitter-hl-face:type.super app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:type))))) (theme-face tree-sitter-hl-face:type.argument app-monochrome-themes-light-theme ((t (:inherit (tree-sitter-hl-face:type))))) (theme-face tree-sitter-hl-face:property app-monochrome-themes-light-theme ((t (:inherit nil :slant italic)))) (theme-face tree-sitter-hl-face:attribute app-monochrome-themes-light-theme ((t (:inherit default)))))"
    ];
    assert_app_monochrome_light_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_org_markdown_and_delimiter_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'app-monochrome-themes-light-theme
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry) (eq (cadr entry) face))
             settings))
          '(markdown-header-face markdown-header-face-1
            markdown-header-face-2 markdown-header-face-3
            markdown-header-face-4 markdown-header-face-5
            markdown-header-face-6 org-default org-level-1 org-level-2
            org-level-3 org-level-4 org-level-5 org-level-6
            org-level-7 org-level-8 org-code org-verbatim org-block
            org-done org-todo org-headline-todo org-headline-done
            org-modern-done org-modern-todo org-table
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
        r##"OK ((theme-face markdown-header-face app-monochrome-themes-light-theme ((t (:weight bold :inherit magit-section-heading :height 1.5)))) (theme-face markdown-header-face-1 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face :height 0.65 :family "IBM Plex Sans")))) (theme-face markdown-header-face-2 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face :height 0.4 :family "IBM Plex Serif")))) (theme-face markdown-header-face-3 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face :height 0.3 :family "IBM Plex Serif")))) (theme-face markdown-header-face-4 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face :height 0.55)))) (theme-face markdown-header-face-5 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face :height 0.55)))) (theme-face markdown-header-face-6 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face :height 0.55)))) (theme-face org-default app-monochrome-themes-light-theme ((t (:inherit (default))))) (theme-face org-level-1 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face)))) (theme-face org-level-2 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face-1)))) (theme-face org-level-3 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face-2)))) (theme-face org-level-4 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face-3)))) (theme-face org-level-5 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face-4)))) (theme-face org-level-6 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face-4 :height 0.85)))) (theme-face org-level-7 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face-4 :height 0.75)))) (theme-face org-level-8 app-monochrome-themes-light-theme ((t (:inherit markdown-header-face-4 :height 0.65)))) (theme-face org-code app-monochrome-themes-light-theme ((t (:background "grey95" :foreground "grey25" :weight bold :family "IBM Plex Mono")))) (theme-face org-verbatim app-monochrome-themes-light-theme ((t (:inherit org-code :weight light)))) (theme-face org-block app-monochrome-themes-light-theme ((t (:inherit shadow :extend t :family "VictorMono Nerd Font")))) (theme-face org-done app-monochrome-themes-light-theme ((t (:inherit org-headline :foreground "black" :background "#0bf" :box t)))) (theme-face org-todo app-monochrome-themes-light-theme ((t (:foreground "black" :background "#f00" :box t)))) (theme-face org-headline-todo app-monochrome-themes-light-theme ((t (:inherit org-headline)))) (theme-face org-headline-done app-monochrome-themes-light-theme ((t (:inherit org-headline)))) (theme-face org-modern-done app-monochrome-themes-light-theme ((t (:inherit org-done)))) (theme-face org-modern-todo app-monochrome-themes-light-theme ((t (:inherit org-todo)))) (theme-face org-table app-monochrome-themes-light-theme ((t (:foreground "Blue1" :family "VictorMono Nerd Font")))) (theme-face rainbow-delimiters-depth-1-face app-monochrome-themes-light-theme ((t (:inherit rainbow-delimiters-base-face :weight bold)))) (theme-face rainbow-delimiters-depth-2-face app-monochrome-themes-light-theme ((t (:weight bold :inherit rainbow-delimiters-base-face :foreground "#900")))) (theme-face rainbow-delimiters-depth-3-face app-monochrome-themes-light-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#5aa")))) (theme-face rainbow-delimiters-depth-4-face app-monochrome-themes-light-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#04f")))) (theme-face rainbow-delimiters-depth-5-face app-monochrome-themes-light-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#707")))) (theme-face rainbow-delimiters-depth-6-face app-monochrome-themes-light-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#700")))) (theme-face rainbow-delimiters-depth-7-face app-monochrome-themes-light-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#070")))) (theme-face rainbow-delimiters-depth-8-face app-monochrome-themes-light-theme ((t (:inherit rainbow-delimiters-base-face :foreground "#007")))) (theme-face rainbow-delimiters-depth-9-face app-monochrome-themes-light-theme ((t (:weight bold :inherit rainbow-delimiters-base-face :foreground "#070")))))"##
    ]];
    assert_app_monochrome_light_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_light_duplicate_precedence_and_enabled_faces_are_exact() {
    let elisp_form = r##"(let* ((theme
                'app-monochrome-themes-light-theme)
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
                '(org-verbatim))))
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
             (default :background) (default :foreground)
             (font-lock-comment-face :foreground)
             (font-lock-keyword-face :weight)
             (link :foreground) (link :underline)
             (error :background) (mode-line :box)))))"##;
    let expect = expect![[
        r##"OK (((org-verbatim (((t (:inherit org-code :weight light))) ((t (:inherit org-code)))))) (app-monochrome-themes-light-theme) ((default :family "default") (default :height 98) (default :background "white") (default :foreground "grey12") (font-lock-comment-face :foreground "#888") (font-lock-keyword-face :weight bold) (link :foreground "#3c5c5c") (link :underline t) (error :background "#fbb") (mode-line :box 1)))"##
    ]];
    assert_app_monochrome_light_parity(elisp_form, expect);
}
