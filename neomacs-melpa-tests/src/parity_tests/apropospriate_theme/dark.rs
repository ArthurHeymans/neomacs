use expect_test::expect;

use super::assert_apropospriate_theme_parity;

#[test]
fn apropospriate_dark_applies_practical_editor_chrome_faces() {
    let elisp_form = r##"(progn
         (require 'hl-line)
         (load-theme 'apropospriate-dark t)
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
fn apropospriate_dark_applies_practical_font_lock_palette() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
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
fn apropospriate_dark_search_completion_and_navigation_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'apropospriate-dark 'theme-settings)))
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
fn apropospriate_dark_diagnostics_and_diff_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'apropospriate-dark 'theme-settings)))
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
fn apropospriate_dark_magit_and_version_control_specs_are_exact() {
    let elisp_form = r##"(let ((settings
                (get 'apropospriate-dark 'theme-settings)))
         (mapcar
          (lambda (face)
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             settings))
          '(magit-process-ok magit-process-ng magit-tag
            magit-branch-local magit-branch-remote
            magit-branch-current magit-section-highlight
            magit-section-heading magit-diff-added
            magit-diff-removed magit-diff-added-highlight
            magit-diff-removed-highlight
            magit-diff-context-highlight
            git-commit-summary blamer-face
            vc-annotate)))"##;
    let expect = expect!["OK (nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil nil)"];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_dark_org_resizing_changes_real_heading_heights() {
    let elisp_form = r##"(progn
         (require 'org)
         (custom-declare-theme
          'apropospriate-dark-resized
          'apropospriate-dark-resized-theme
          "Parity fixture" nil)
         (custom-declare-theme
          'apropospriate-dark-flat
          'apropospriate-dark-flat-theme
          "Parity fixture" nil)
         (let ((apropospriate-org-level-resizing t))
           (create-apropospriate-theme
            'dark 'apropospriate-dark-resized))
         (let ((apropospriate-org-level-resizing nil))
           (create-apropospriate-theme
            'dark 'apropospriate-dark-flat))
         (mapcar
          (lambda (theme)
            (let ((settings (get theme 'theme-settings)))
              (list
               theme
               (mapcar
                (lambda (face)
                  (cl-find-if
                   (lambda (entry)
                     (and (eq (car entry) 'theme-face)
                          (eq (cadr entry) face)))
                   settings))
                '(org-document-title org-level-1
                  org-level-2 org-level-3 org-level-4)))))
          '(apropospriate-dark-resized
            apropospriate-dark-flat)))"##;
    let expect = expect![[
        r##"OK ((apropospriate-dark-resized ((theme-face org-document-title apropospriate-dark-resized ((#1=((class color) (min-colors 89)) (:weight bold :foreground "#FFCC80" . #2=(:height 1.44))))) (theme-face org-level-1 apropospriate-dark-resized ((#1# (:inherit header-line :height 1.3)))) (theme-face org-level-2 apropospriate-dark-resized ((#1# (:inherit header-line :height 1.2)))) (theme-face org-level-3 apropospriate-dark-resized ((#1# (:inherit header-line :height 1.1)))) (theme-face org-level-4 apropospriate-dark-resized ((#1# . #3=((:inherit header-line))))))) (apropospriate-dark-flat ((theme-face org-document-title apropospriate-dark-flat ((#1# (:weight bold :foreground "#FFCC80" . #2#)))) (theme-face org-level-1 apropospriate-dark-flat ((#1# (:inherit header-line :height 1.0)))) (theme-face org-level-2 apropospriate-dark-flat ((#1# (:inherit header-line :height 1.0)))) (theme-face org-level-3 apropospriate-dark-flat ((#1# (:inherit header-line :height 1.0)))) (theme-face org-level-4 apropospriate-dark-flat ((#1# . #3#))))))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_dark_mode_line_height_customization_rebuilds_specs() {
    let elisp_form = r##"(mapcar
         (lambda (height)
           (let* ((apropospriate-mode-line-height height)
                  (theme
                   (intern
                    (format "apropospriate-dark-height-%s"
                            height))))
             (custom-declare-theme
              theme
              (intern (format "%s-theme" theme))
              "Parity fixture" nil)
             (create-apropospriate-theme 'dark theme)
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
        r##"OK ((nil ((theme-face mode-line apropospriate-dark-height-nil ((#1=((class color) (min-colors 89)) (:box (:line-width 4 :color "#2A2A2A" . #2=(:style nil)) :background "#323232" :foreground "#E0E0E0" :height 1.0)))) (theme-face mode-line-inactive apropospriate-dark-height-nil ((#1# (:box (:line-width 4 :color "#494949" . #3=(:style nil)) :background "#494949" :foreground "#9E9E9E" :height 1.0)))) (theme-face powerline-active1 apropospriate-dark-height-nil ((#1# (:background "#424242" :height 1.0)))) (theme-face powerline-inactive1 apropospriate-dark-height-nil ((#1# (:background "#494949" :height 1.0)))))) (1.0 ((theme-face mode-line apropospriate-dark-height-1.0 ((#1# (:box (:line-width 4 :color "#2A2A2A" . #2#) :background "#323232" :foreground "#E0E0E0" :height 1.0)))) (theme-face mode-line-inactive apropospriate-dark-height-1.0 ((#1# (:box (:line-width 4 :color "#494949" . #3#) :background "#494949" :foreground "#9E9E9E" :height 1.0)))) (theme-face powerline-active1 apropospriate-dark-height-1.0 ((#1# (:background "#424242" :height 1.0)))) (theme-face powerline-inactive1 apropospriate-dark-height-1.0 ((#1# (:background "#494949" :height 1.0)))))) (1.25 ((theme-face mode-line apropospriate-dark-height-1.25 ((#1# (:box (:line-width 4 :color "#2A2A2A" . #2#) :background "#323232" :foreground "#E0E0E0" :height 1.25)))) (theme-face mode-line-inactive apropospriate-dark-height-1.25 ((#1# (:box (:line-width 4 :color "#494949" . #3#) :background "#494949" :foreground "#9E9E9E" :height 1.25)))) (theme-face powerline-active1 apropospriate-dark-height-1.25 ((#1# (:background "#424242" :height 1.25)))) (theme-face powerline-inactive1 apropospriate-dark-height-1.25 ((#1# (:background "#494949" :height 1.25)))))))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_dark_terminal_palette_and_theme_variables_are_practical() {
    let elisp_form = r##"(progn
         (require 'ansi-color)
         (load-theme 'apropospriate-dark t)
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
        r##"OK (["#424242" "#EF9A9A" "#C5E1A5" "#FFEE58" "#64B5F6" "#E1BEE7" "#80DEEA" "#E0E0E0"] ((pos-tip-foreground-color nil nil) (pos-tip-background-color nil nil) (beacon-color nil nil) (highlight-symbol-colors nil nil) (vc-annotate-color-map nil nil) (vc-annotate-very-old-color nil nil)) ((ansi-color-red "red3") (ansi-color-bright-red "red2") (ansi-color-green "green3") (ansi-color-bright-green "green2") (ansi-color-blue "blue2") (ansi-color-bright-blue "blue1") (ansi-color-cyan "cyan3") (ansi-color-bright-cyan "cyan2")))"##
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_dark_theme_enable_disable_restores_default_face() {
    let elisp_form = r##"(let ((before
                (list
                 (face-attribute
                  'default :foreground nil 'default)
                 (face-attribute
                  'default :background nil 'default))))
         (load-theme 'apropospriate-dark t)
         (let ((enabled
                (list
                 custom-enabled-themes
                 (face-attribute
                  'default :foreground nil 'default)
                 (face-attribute
                  'default :background nil 'default))))
           (disable-theme 'apropospriate-dark)
           (list
            before enabled custom-enabled-themes
            (list
             (face-attribute
              'default :foreground nil 'default)
             (face-attribute
              'default :background nil 'default)))))"##;
    let expect = expect![[
        r#"OK (("unspecified-fg" "unspecified-bg") ((apropospriate-dark) "unspecified-fg" "unspecified-bg") nil ("unspecified-fg" "unspecified-bg"))"#
    ]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}

#[test]
fn apropospriate_dark_theme_reload_is_idempotent_in_real_lifecycle() {
    let elisp_form = r##"(progn
         (load-theme 'apropospriate-dark t)
         (load-theme 'apropospriate-dark t)
         (load-theme 'apropospriate-dark t)
         (list
          custom-enabled-themes
          (cl-count 'apropospriate-dark
                    custom-enabled-themes)
          (cl-count 'apropospriate-dark
                    custom-known-themes)
          (face-attribute
           'default :background nil 'default)
          (face-attribute
           'font-lock-keyword-face
           :foreground nil 'default)))"##;
    let expect = expect![[r#"OK ((apropospriate-dark) 1 1 "unspecified-bg" "unspecified-fg")"#]];
    assert_apropospriate_theme_parity(elisp_form, expect);
}
