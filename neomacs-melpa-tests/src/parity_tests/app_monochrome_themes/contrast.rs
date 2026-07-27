use expect_test::expect;

use super::assert_app_monochrome_parity;

#[test]
fn app_monochrome_dark_and_light_face_sets_have_exact_overlap_and_differences() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (load-theme 'app-monochrome-themes-light-theme t)
         (let* ((less
                 (lambda (left right)
                   (string< (symbol-name left)
                            (symbol-name right))))
                (dark
                 (delete-dups
                  (mapcar
                   #'cadr
                   (get 'app-monochrome-themes-dark-theme
                        'theme-settings))))
                (light
                 (delete-dups
                  (mapcar
                   #'cadr
                   (get 'app-monochrome-themes-light-theme
                        'theme-settings)))))
           (list
            (length dark)
            (length light)
            (length (cl-intersection dark light))
            (sort (cl-set-difference dark light) less)
            (sort (cl-set-difference light dark) less))))"##;
    let expect = expect![
        "OK (172 166 163 (font-latex-bold-face font-latex-italic-face font-latex-math-face font-latex-sectioning-5-face font-latex-string-face font-latex-underline-face font-latex-verbatim-face font-latex-warning-face markdown-inline-code-face) (flycheck-error flycheck-warning org-block))"
    ];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_paired_base_faces_pin_dark_and_light_design_choices() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (load-theme 'app-monochrome-themes-light-theme t)
         (cl-labels
             ((spec
               (theme face)
               (nth
                3
                (cl-find-if
                 (lambda (entry) (eq (cadr entry) face))
                 (get theme 'theme-settings)))))
           (mapcar
            (lambda (face)
              (list
               face
               (spec 'app-monochrome-themes-dark-theme face)
               (spec 'app-monochrome-themes-light-theme face)))
            '(default variable-pitch highlight italic error warning
              success bookmark-face link line-number
              font-lock-comment-face font-lock-builtin-face
              font-lock-constant-face font-lock-string-face
              font-lock-variable-name-face))))"##;
    let expect = expect![[
        r##"OK ((default ((t (:family "UbuntuMono Nerd Font" :foundry "DAMA" :slant normal :weight regular :height 98 :width normal))) ((t (:inherit nil :extend nil :stipple nil :background "white" :foreground "grey12" :inverse-video nil :box nil :strike-through nil :overline nil :underline nil :slant normal :weight regular :height 98 :width normal :foundry "ADBO" :family "Fira Code")))) (variable-pitch ((t (:family "IBM Plex Serif"))) ((t (:family "IBM Plex Serif")))) (highlight ((t (:background "#bcc" :foreground "black"))) ((t (:background "#bcc" :foreground "black")))) (italic ((t (:slant italic :weight normal :family "IBM Plex Sans"))) ((t (:slant italic :weight normal :family "IBM Plex Sans")))) (error ((t (:box (:line-width (2 . 2) :color "Red" :style released-button) :weight bold))) ((t (:background "#fbb" :foreground "black" :box (:line-width (2 . 2) :color "Black" :style flat-button))))) (warning ((t (:foreground "gold"))) ((t (:foreground "red4")))) (success ((t (:foreground "white" :weight bold))) ((t (:foreground "black" :weight bold)))) (bookmark-face ((t (:background "Black" :foreground "Gold"))) ((t (:background "black" :foreground "Gold")))) (link ((t (:underline t :foreground "#5cacac"))) ((t (:underline t :foreground "#3c5c5c")))) (line-number ((t (:inherit shadow :family "VictorMono Nerd Font"))) ((t (:inherit shadow :family "VictorMono Nerd Font")))) (font-lock-comment-face ((t (:foreground "#aaa"))) ((t (:foreground "#888")))) (font-lock-builtin-face ((t (:family "Linux Libertine Mono" :background "grey16"))) ((t (:family "Linux Libertine Mono" :weight bold :foreground "grey47" :background "white")))) (font-lock-constant-face ((t (:foreground "white" :weight bold :inherit font-lock-type-face))) ((t (:foreground "black" :weight bold :inherit font-lock-type-face)))) (font-lock-string-face ((t (:foreground "grey62" :family "IBM Plex Mono"))) ((t (:foreground "grey40" :family "IBM Plex Mono")))) (font-lock-variable-name-face ((t (:foreground "white" :weight thin))) ((t (:foreground "black" :weight thin)))))"##
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_paired_workflow_faces_pin_search_diagnostics_and_magit_contrast() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (load-theme 'app-monochrome-themes-light-theme t)
         (cl-labels
             ((spec
               (theme face)
               (nth
                3
                (cl-find-if
                 (lambda (entry) (eq (cadr entry) face))
                 (get theme 'theme-settings)))))
           (mapcar
            (lambda (face)
              (list
               face
               (spec 'app-monochrome-themes-dark-theme face)
               (spec 'app-monochrome-themes-light-theme face)))
            '(lazy-highlight swiper-line-face ivy-cursor
              orderless-match-face-0 orderless-match-face-1
              show-paren-match flymake-error flycheck-info
              lsp-ui-doc-background lsp-ui-sideline-symbol
              magit-branch-remote magit-section-highlight
              org-code org-done org-todo org-table
              lsp-rust-analyzer-inlay-face
              fancy-compilation-default-face))))"##;
    let expect = expect![[
        r##"OK ((lazy-highlight ((t (:distant-foreground "white" :box (:line-width (2 . 2) :color "grey75" :style released-button)))) ((t (:distant-foreground "black" :box (:line-width (2 . 2) :color "grey25" :style released-button))))) (swiper-line-face ((t (:inherit highlight :box (:line-width (2 . 2) :color "grey75" :style released-button)))) ((t (:inherit highlight :box (:line-width (2 . 2) :color "grey20" :style released-button))))) (ivy-cursor ((((class color) (background light)) (:foreground "white" :background "black")) (((class color) (background dark)) (:foreground "black" :background "white"))) ((((class color) (background light)) (:foreground "white" :background "black")) (((class color) (background dark)) (:foreground "white" :background "white")))) (orderless-match-face-0 ((t (:background "#8cc"))) ((t (:background "#8cc")))) (orderless-match-face-1 ((t (:slant italic :inherit orderless-match-face-0))) ((t (:background "#bbf" :underline t)))) (show-paren-match ((t (:inherit default :background "#8cc"))) ((t (:inherit default :background "#8cc")))) (flymake-error ((t (:underline (:color "Red1" :style dashes :position nil)))) ((t (:underline (:color "Red1" :style dashes :position nil))))) (flycheck-info ((t (:inherit success :underline t))) ((t (:inherit success :underline t)))) (lsp-ui-doc-background ((t (:background "grey90"))) ((t (:background "grey90")))) (lsp-ui-sideline-symbol ((t (:height 0.99 :box (:line-width (1 . -1) :color "grey" :style nil) :foreground "grey"))) ((t (:height 0.99 :box (:line-width (1 . -1) :color "grey20" :style nil) :foreground "grey20")))) (magit-branch-remote ((t (:foreground "white" :weight bold :height 1.25 :family "IBM Plex Serif"))) ((t (:foreground "black" :weight bold :height 1.25 :family "IBM Plex Serif")))) (magit-section-highlight ((((class color) (background light)) (:background "grey95" :extend t)) (((class color) (background dark)) (:background "grey20" :extend t))) ((((class color) (background light)) (:background "grey80" :extend t)) (((class color) (background dark)) (:background "grey44" :extend t)))) (org-code ((t (:family "VictorMono Nerd Font" :foreground "grey75" :background "grey10"))) ((t (:background "grey95" :foreground "grey25" :weight bold :family "IBM Plex Mono")))) (org-done ((t (:inherit org-headline :foreground "white" :background "#048" :box t))) ((t (:inherit org-headline :foreground "black" :background "#0bf" :box t)))) (org-todo ((t (:foreground "white" :background "#800" :box t))) ((t (:foreground "black" :background "#f00" :box t)))) (org-table ((t (:foreground "white" :background "black" :family "VictorMono Nerd Font"))) ((t (:foreground "Blue1" :family "VictorMono Nerd Font")))) (lsp-rust-analyzer-inlay-face ((t (:inherit font-lock-comment-face :foreground "black" :background "white"))) ((t (:inherit font-lock-comment-face :foreground "grey30" :background "white")))) (fancy-compilation-default-face ((t (:inherit font-lock-string-face))) ((t (:inherit default)))))"##
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_loading_light_over_dark_changes_effective_builtin_faces() {
    let elisp_form = r##"(let ((requests
                '((default :family) (default :height)
                  (default :background) (default :foreground)
                  (font-lock-comment-face :foreground)
                  (font-lock-builtin-face :background)
                  (font-lock-string-face :foreground)
                  (error :background) (warning :foreground)
                  (link :foreground))))
         (cl-labels
             ((values
               ()
               (mapcar
                (lambda (request)
                  (list
                   (car request)
                   (cadr request)
                   (face-attribute
                    (car request) (cadr request)
                    nil 'default)))
                requests)))
           (let ((baseline (values)))
             (load-theme
              'app-monochrome-themes-dark-theme t)
             (let ((dark (values)))
               (load-theme
                'app-monochrome-themes-light-theme t)
               (list baseline dark (values)
                     custom-enabled-themes)))))"##;
    let expect = expect![[
        r##"OK (((default :family "default") (default :height 1) (default :background "unspecified-bg") (default :foreground "unspecified-fg") (font-lock-comment-face :foreground "unspecified-fg") (font-lock-builtin-face :background "unspecified-bg") (font-lock-string-face :foreground "unspecified-fg") (error :background "unspecified-bg") (warning :foreground "unspecified-fg") (link :foreground "unspecified-fg")) ((default :family "UbuntuMono Nerd Font") (default :height 98) (default :background "unspecified-bg") (default :foreground "unspecified-fg") (font-lock-comment-face :foreground "#aaa") (font-lock-builtin-face :background "grey16") (font-lock-string-face :foreground "grey62") (error :background "unspecified-bg") (warning :foreground "gold") (link :foreground "#5cacac")) ((default :family "default") (default :height 98) (default :background "white") (default :foreground "grey12") (font-lock-comment-face :foreground "#888") (font-lock-builtin-face :background "white") (font-lock-string-face :foreground "grey40") (error :background "#fbb") (warning :foreground "red4") (link :foreground "#3c5c5c")) (app-monochrome-themes-light-theme app-monochrome-themes-dark-theme))"##
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_disabling_light_reveals_dark_then_restores_baseline() {
    let elisp_form = r##"(let ((requests
                '((default :family) (default :height)
                  (default :background) (default :foreground)
                  (font-lock-comment-face :foreground)
                  (font-lock-string-face :foreground)
                  (link :foreground) (warning :foreground))))
         (cl-labels
             ((values
               ()
               (mapcar
                (lambda (request)
                  (face-attribute
                   (car request) (cadr request)
                   nil 'default))
                requests)))
           (let ((baseline (values)))
             (load-theme
              'app-monochrome-themes-dark-theme t)
             (let ((dark (values)))
               (load-theme
                'app-monochrome-themes-light-theme t)
               (let ((light (values)))
                 (disable-theme
                  'app-monochrome-themes-light-theme)
                 (let ((revealed-dark (values)))
                   (disable-theme
                    'app-monochrome-themes-dark-theme)
                   (let ((restored (values)))
                     (list baseline dark light revealed-dark
                           restored
                           (equal dark revealed-dark)
                           (equal baseline restored)
                           custom-enabled-themes))))))))"##;
    let expect = expect![[
        r##"OK (("default" 1 "unspecified-bg" "unspecified-fg" "unspecified-fg" "unspecified-fg" "unspecified-fg" "unspecified-fg") ("UbuntuMono Nerd Font" 98 "unspecified-bg" "unspecified-fg" "#aaa" "grey62" "#5cacac" "gold") ("default" 98 "white" "grey12" "#888" "grey40" "#3c5c5c" "red4") ("UbuntuMono Nerd Font" 98 "unspecified-bg" "unspecified-fg" "#aaa" "grey62" "#5cacac" "gold") ("default" 1 "unspecified-bg" "unspecified-fg" "unspecified-fg" "unspecified-fg" "unspecified-fg" "unspecified-fg") t t nil)"##
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_external_package_faces_are_stored_without_dependencies() {
    let elisp_form = r##"(let ((faces
                '(magit-filename lsp-ui-sideline-symbol
                  tree-sitter-hl-face:function
                  rainbow-delimiters-depth-9-face
                  jinx-misspelled rust-unsafe)))
         (let ((before (mapcar #'facep faces)))
           (load-theme 'app-monochrome-themes-dark-theme t)
           (let ((settings
                  (get 'app-monochrome-themes-dark-theme
                       'theme-settings)))
             (list
              before
              (mapcar #'facep faces)
              (mapcar
               (lambda (face)
                 (cl-find-if
                  (lambda (entry) (eq (cadr entry) face))
                  settings))
               faces)))))"##;
    let expect = expect![[
        r##"OK ((nil nil nil nil nil nil) (nil nil nil nil nil nil) ((theme-face magit-filename app-monochrome-themes-dark-theme ((t (:weight normal)))) (theme-face lsp-ui-sideline-symbol app-monochrome-themes-dark-theme ((t (:height 0.99 :box (:line-width (1 . -1) :color "grey" :style nil) :foreground "grey")))) (theme-face tree-sitter-hl-face:function app-monochrome-themes-dark-theme ((t (:inherit (font-lock-function-name-face))))) (theme-face rainbow-delimiters-depth-9-face app-monochrome-themes-dark-theme ((t (:weight bold :inherit rainbow-delimiters-base-face :foreground "#afa")))) (theme-face jinx-misspelled app-monochrome-themes-dark-theme ((t (:underline t :inherit warning)))) (theme-face rust-unsafe app-monochrome-themes-dark-theme ((t (:weight bold))))))"##
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_reloading_theme_sources_is_idempotent_for_setting_counts() {
    let elisp_form = r##"(let* ((dark-name
                "app-monochrome-themes-dark-theme-theme")
              (light-name
               "app-monochrome-themes-light-theme-theme")
              (dark-file
               (locate-file dark-name
                            custom-theme-load-path '(".el")))
              (light-file
               (locate-file light-name
                            custom-theme-load-path '(".el"))))
         (load dark-file nil 'nomessage)
         (load light-file nil 'nomessage)
         (let ((first
                (list
                 (length
                  (get 'app-monochrome-themes-dark-theme
                       'theme-settings))
                 (length
                  (get 'app-monochrome-themes-light-theme
                       'theme-settings)))))
           (load dark-file nil 'nomessage)
           (load light-file nil 'nomessage)
           (list
            (mapcar #'file-name-nondirectory
                    (list dark-file light-file))
            first
            (list
             (length
              (get 'app-monochrome-themes-dark-theme
                   'theme-settings))
             (length
              (get 'app-monochrome-themes-light-theme
                   'theme-settings)))
            (cl-count 'app-monochrome-themes-dark-theme
                      features)
            (cl-count 'app-monochrome-themes-light-theme
                      features))))"##;
    let expect = expect![[
        r#"OK (("app-monochrome-themes-dark-theme-theme.el" "app-monochrome-themes-light-theme-theme.el") (176 167) (352 334) 1 1)"#
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}

#[test]
fn app_monochrome_enable_disable_reenable_preserves_theme_order_and_specs() {
    let elisp_form = r##"(progn
         (load-theme 'app-monochrome-themes-dark-theme t)
         (let ((initial
                (list
                 custom-enabled-themes
                 (face-attribute
                  'font-lock-comment-face :foreground
                  nil 'default))))
           (disable-theme
            'app-monochrome-themes-dark-theme)
           (let ((disabled custom-enabled-themes))
             (enable-theme
              'app-monochrome-themes-dark-theme)
             (list
              initial disabled custom-enabled-themes
              (face-attribute
               'font-lock-comment-face :foreground
               nil 'default)
              (length
               (get 'app-monochrome-themes-dark-theme
                    'theme-settings))))))"##;
    let expect = expect![[
        r##"OK (((app-monochrome-themes-dark-theme) "#aaa") nil (app-monochrome-themes-dark-theme) "#aaa" 176)"##
    ]];
    assert_app_monochrome_parity(elisp_form, expect);
}
