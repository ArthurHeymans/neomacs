use expect_test::expect;

use super::assert_apropospriate_theme_parity;

#[test]
fn apropospriate_light_applies_practical_editor_chrome_faces() {
    let elisp_form = r##"(progn
         (require 'hl-line)
         (load-theme 'apropospriate-light t)
         (mapcar
          (lambda (face)
            (list face
                  (face-attribute face :foreground nil 'default)
                  (face-attribute face :background nil 'default)
                  (face-attribute face :weight nil 'default)
                  (face-attribute face :height nil 'default)
                  (face-attribute face :box nil 'default)))
          '(default cursor fringe header-line mode-line
            mode-line-inactive region highlight hl-line)))"##;
    let expect = expect![[
        r#"OK ((default "unspecified-fg" "unspecified-bg" normal 1 nil) (cursor "unspecified-fg" "white" normal 1 nil) (fringe "unspecified-fg" "gray" normal 1 nil) (header-line "unspecified-fg" "unspecified-bg" normal 1 nil) (mode-line "unspecified-fg" "unspecified-bg" normal 1 nil) (mode-line-inactive "unspecified-fg" "unspecified-bg" normal 1 nil) (region "unspecified-fg" "unspecified-bg" normal 1 nil) (highlight "unspecified-fg" "unspecified-bg" normal 1 nil) (hl-line "unspecified-fg" "unspecified-bg" normal 1 nil))"#
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_applies_practical_font_lock_palette() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-light t)
         (mapcar
          (lambda (face)
            (list face
                  (face-attribute face :foreground nil 'default)
                  (face-attribute face :background nil 'default)
                  (face-attribute face :inherit nil 'default)))
          '(font-lock-builtin-face
            font-lock-comment-face
            font-lock-constant-face
            font-lock-doc-face
            font-lock-function-name-face
            font-lock-keyword-face
            font-lock-preprocessor-face
            font-lock-string-face
            font-lock-type-face
            font-lock-variable-name-face
            font-lock-warning-face)))"##;
    let expect = expect![[
        r#"OK ((font-lock-builtin-face "unspecified-fg" "unspecified-bg" nil) (font-lock-comment-face "unspecified-fg" "unspecified-bg" nil) (font-lock-constant-face "unspecified-fg" "unspecified-bg" nil) (font-lock-doc-face "unspecified-fg" "unspecified-bg" font-lock-string-face) (font-lock-function-name-face "unspecified-fg" "unspecified-bg" nil) (font-lock-keyword-face "unspecified-fg" "unspecified-bg" nil) (font-lock-preprocessor-face "unspecified-fg" "unspecified-bg" font-lock-builtin-face) (font-lock-string-face "unspecified-fg" "unspecified-bg" nil) (font-lock-type-face "unspecified-fg" "unspecified-bg" nil) (font-lock-variable-name-face "unspecified-fg" "unspecified-bg" nil) (font-lock-warning-face "unspecified-fg" "unspecified-bg" error))"#
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_search_completion_and_navigation_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'apropospriate-light 'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             settings))
          '(match isearch query-replace lazy-highlight
            isearch-fail vertico-current
            orderless-match-face-0 orderless-match-face-1
            orderless-match-face-2 orderless-match-face-3
            ivy-current-match ivy-minibuffer-match-face-1
            helm-match helm-selection avy-lead-face
            corfu-current company-tooltip-selection)))"##;
    let expect =
        expect!["OK (nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil)"];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_diagnostics_and_diff_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'apropospriate-light 'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             settings))
          '(error warning success
            flycheck-error flycheck-warning flycheck-info
            flymake-error flymake-warning flymake-note
            diff-added diff-changed diff-removed
            diff-refine-added diff-refine-changed
            diff-refine-removed
            smerge-upper smerge-lower
            smerge-refined-added smerge-refined-removed)))"##;
    let expect =
        expect!["OK (nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil)"];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_org_and_markdown_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'apropospriate-light 'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             settings))
          '(org-document-title org-document-info
            org-document-info-keyword org-code org-block
            org-block-background org-table org-todo org-done
            org-level-1 org-level-2 org-level-3 org-level-8
            markdown-url-face markdown-link-face)))"##;
    let expect = expect!["OK (nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil)"];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_eshell_and_terminal_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'apropospriate-light 'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             settings))
          '(eshell-prompt eshell-ls-archive
            eshell-ls-directory eshell-ls-executable
            eshell-ls-symlink
            ansi-color-red ansi-color-bright-red
            ansi-color-green ansi-color-bright-green
            ansi-color-yellow ansi-color-bright-yellow
            ansi-color-blue ansi-color-bright-blue
            ansi-color-cyan ansi-color-bright-cyan
            term-color-white term-color-bright-white)))"##;
    let expect =
        expect!["OK (nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil)"];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_mode_line_height_customization_rebuilds_specs() {
    let elisp_form = r##"(mapcar
         (lambda (height)
           (let* ((apropospriate-mode-line-height height)
                  (theme
                   (intern
                    (format "apropospriate-light-height-%s"
                            height))))
             (custom-declare-theme
              theme
              (intern (format "%s-theme" theme))
              "Parity fixture" nil)
             (create-apropospriate-theme 'light theme)
             (list
              height
              (mapcar
               (lambda (face)
                 (cl-find-if
                  (lambda (entry)
                    (and (eq (car entry) 'theme-face)
                         (eq (cadr entry) face)))
                  (get theme 'theme-settings)))
               '(mode-line mode-line-inactive
                 powerline-active1 powerline-inactive1)))))
         '(nil 1.0 1.25))"##;
    let expect = expect![[
        r##"OK ((nil ((theme-face mode-line apropospriate-light-height-nil ((#1=((class color) (min-colors 89)) (:box (:line-width 4 :color "#E6E6E6" . #2=(:style nil)) :background "#FDFDFD" :foreground "#546E7A" :height 1.0)))) (theme-face mode-line-inactive apropospriate-light-height-nil ((#1# (:box (:line-width 4 :color "#F0F0F0" . #3=(:style nil)) :background "#F0F0F0" :foreground "#78909C" :height 1.0)))) (theme-face powerline-active1 apropospriate-light-height-nil ((#1# (:background "#F5F5F5" :height 1.0)))) (theme-face powerline-inactive1 apropospriate-light-height-nil ((#1# (:background "#F0F0F0" :height 1.0)))))) (1.0 ((theme-face mode-line apropospriate-light-height-1.0 ((#1# (:box (:line-width 4 :color "#E6E6E6" . #2#) :background "#FDFDFD" :foreground "#546E7A" :height 1.0)))) (theme-face mode-line-inactive apropospriate-light-height-1.0 ((#1# (:box (:line-width 4 :color "#F0F0F0" . #3#) :background "#F0F0F0" :foreground "#78909C" :height 1.0)))) (theme-face powerline-active1 apropospriate-light-height-1.0 ((#1# (:background "#F5F5F5" :height 1.0)))) (theme-face powerline-inactive1 apropospriate-light-height-1.0 ((#1# (:background "#F0F0F0" :height 1.0)))))) (1.25 ((theme-face mode-line apropospriate-light-height-1.25 ((#1# (:box (:line-width 4 :color "#E6E6E6" . #2#) :background "#FDFDFD" :foreground "#546E7A" :height 1.25)))) (theme-face mode-line-inactive apropospriate-light-height-1.25 ((#1# (:box (:line-width 4 :color "#F0F0F0" . #3#) :background "#F0F0F0" :foreground "#78909C" :height 1.25)))) (theme-face powerline-active1 apropospriate-light-height-1.25 ((#1# (:background "#F5F5F5" :height 1.25)))) (theme-face powerline-inactive1 apropospriate-light-height-1.25 ((#1# (:background "#F0F0F0" :height 1.25)))))))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_terminal_palette_and_theme_variables_are_practical() {
    let elisp_form = r##"(progn
         (require 'ansi-color)
         (load-theme 'apropospriate-light t)
         (list
          ansi-color-names-vector
          (mapcar
           (lambda (symbol)
             (list symbol
                   (boundp symbol)
                   (and (boundp symbol)
                        (symbol-value symbol))))
           '(pos-tip-foreground-color
             pos-tip-background-color
             beacon-color
             highlight-symbol-colors
             vc-annotate-color-map
             vc-annotate-very-old-color))
          (mapcar
           (lambda (face)
             (list face
                   (face-attribute
                    face :foreground nil 'default)))
           '(ansi-color-red ansi-color-bright-red
             ansi-color-green ansi-color-bright-green
             ansi-color-blue ansi-color-bright-blue
             ansi-color-cyan ansi-color-bright-cyan))))"##;
    let expect = expect![[
        r##"OK (["#F5F5F5" "#FF1744" "#66BB6A" "#F57F17" "#42A5F5" "#7E57C2" "#0097A7" "#546E7A"] ((pos-tip-foreground-color nil nil) (pos-tip-background-color nil nil) (beacon-color nil nil) (highlight-symbol-colors nil nil) (vc-annotate-color-map nil nil) (vc-annotate-very-old-color nil nil)) ((ansi-color-red "red3") (ansi-color-bright-red "red2") (ansi-color-green "green3") (ansi-color-bright-green "green2") (ansi-color-blue "blue2") (ansi-color-bright-blue "blue1") (ansi-color-cyan "cyan3") (ansi-color-bright-cyan "cyan2")))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_theme_enable_disable_restores_default_face() {
    let elisp_form = r##"(let ((before
                (list
                 (face-attribute
                  'default :foreground nil 'default)
                 (face-attribute
                  'default :background nil 'default))))
         (load-theme 'apropospriate-light t)
         (let ((enabled
                (list
                 custom-enabled-themes
                 (face-attribute
                  'default :foreground nil 'default)
                 (face-attribute
                  'default :background nil 'default))))
           (disable-theme 'apropospriate-light)
           (list
            before enabled custom-enabled-themes
            (list
             (face-attribute
              'default :foreground nil 'default)
             (face-attribute
              'default :background nil 'default)))))"##;
    let expect = expect![[
        r#"OK (("unspecified-fg" "unspecified-bg") ((apropospriate-light) "unspecified-fg" "unspecified-bg") nil ("unspecified-fg" "unspecified-bg"))"#
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_light_theme_reload_is_idempotent_in_real_lifecycle() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-light t)
         (load-theme 'apropospriate-light t)
         (load-theme 'apropospriate-light t)
         (list
          custom-enabled-themes
          (cl-count 'apropospriate-light
                    custom-enabled-themes)
          (cl-count 'apropospriate-light
                    custom-known-themes)
          (face-attribute
           'default :background nil 'default)
          (face-attribute
           'font-lock-keyword-face
           :foreground nil 'default)))"##;
    let expect = expect![[r#"OK ((apropospriate-light) 1 1 "unspecified-bg" "unspecified-fg")"#]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}
