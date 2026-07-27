use expect_test::expect;

use super::{assert_anti_zenburn_theme_parity, assert_anti_zenburn_theme_with_prelude_parity};

#[test]
fn anti_zenburn_theme_core_editor_face_specs_match_every_literal_attribute() {
    let elisp_form = r##"(let ((settings
                    (get
                     'anti-zenburn
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((setting
                   (seq-find
                    (lambda (candidate)
                      (and
                       (eq
                        (car candidate)
                        'theme-face)
                       (eq
                        (cadr candidate)
                        face)))
                    settings)))
              (list
               face
               (copy-tree
                (nth 3 setting)))))
          '(button
            link
            link-visited
            default
            cursor
            escape-glyph
            widget-field
            fringe
            header-line
            highlight
            success
            warning
            tooltip
            minibuffer-prompt
            mode-line
            mode-line-buffer-id
            mode-line-inactive
            region
            secondary-selection
            trailing-whitespace
            vertical-border)))"##;
    let expect = expect![[
        r##"OK ((button ((t (:underline t)))) (link ((t (:foreground "#0f2050" :underline t :weight bold)))) (link-visited ((t (:foreground "#2f4070" :underline t :weight normal)))) (default ((t (:foreground "#232333" :background "#c0c0c0")))) (cursor ((t (:foreground "#232333" :background "#000010")))) (escape-glyph ((t (:foreground "#0f2050" :weight bold)))) (widget-field ((t (:foreground "#232333" :background "#909090")))) (fringe ((t (:foreground "#232333" :background "#b0b0b0")))) (header-line ((t (:foreground "#0f2050" :background "#d4d4d4" :box (:line-width -1 :style released-button))))) (highlight ((t (:background "#c7c7c7")))) (success ((t (:foreground "#806080" :weight bold)))) (warning ((t (:foreground "#205070" :weight bold)))) (tooltip ((t (:foreground "#232333" :background "#b0b0b0")))) (minibuffer-prompt ((t (:foreground "#0f2050")))) (mode-line ((((class color) (min-colors 89)) (:foreground "#704d70" :background "#d4d4d4" :box (:line-width -1 :style released-button))) (t (:inverse-video t)))) (mode-line-buffer-id ((t (:foreground "#0f2050" :weight bold)))) (mode-line-inactive ((t (:foreground "#a080a0" :background "#c7c7c7" :box (:line-width -1 :style released-button))))) (region ((((class color) (min-colors 89)) (:background "#d4d4d4")) (t (:inverse-video t)))) (secondary-selection ((t (:background "#a0a0a0")))) (trailing-whitespace ((t (:background "#336c6c")))) (vertical-border ((t (:foreground "#232333")))))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_language_and_search_face_specs_match_real_editor_semantics() {
    let elisp_form = r##"(let ((settings
                    (get
                     'anti-zenburn
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((setting
                   (seq-find
                    (lambda (candidate)
                      (eq
                       (cadr candidate)
                       face))
                    settings)))
              (list
               face
               (copy-tree
                (nth 3 setting)))))
          '(font-lock-builtin-face
            font-lock-comment-face
            font-lock-comment-delimiter-face
            font-lock-constant-face
            font-lock-doc-face
            font-lock-function-name-face
            font-lock-keyword-face
            font-lock-negation-char-face
            font-lock-preprocessor-face
            font-lock-regexp-grouping-construct
            font-lock-regexp-grouping-backslash
            font-lock-string-face
            font-lock-type-face
            font-lock-variable-name-face
            font-lock-warning-face
            isearch
            isearch-fail
            lazy-highlight
            match
            show-paren-match
            show-paren-mismatch)))"##;
    let expect = expect![[
        r##"OK ((font-lock-builtin-face ((t (:foreground "#232333" :weight bold)))) (font-lock-comment-face ((t (:foreground "#806080")))) (font-lock-comment-delimiter-face ((t (:foreground "#a080a0")))) (font-lock-constant-face ((t (:foreground "#401440")))) (font-lock-doc-face ((t (:foreground "#603a60")))) (font-lock-function-name-face ((t (:foreground "#6c1f1c")))) (font-lock-keyword-face ((t (:foreground "#0f2050" :weight bold)))) (font-lock-negation-char-face ((t (:foreground "#0f2050" :weight bold)))) (font-lock-preprocessor-face ((t (:foreground "#6b400c")))) (font-lock-regexp-grouping-construct ((t (:foreground "#0f2050" :weight bold)))) (font-lock-regexp-grouping-backslash ((t (:foreground "#806080" :weight bold)))) (font-lock-string-face ((t (:foreground "#336c6c")))) (font-lock-type-face ((t (:foreground "#834744")))) (font-lock-variable-name-face ((t (:foreground "#205070")))) (font-lock-warning-face ((t (:foreground "#2f4070" :weight bold)))) (isearch ((t (:foreground "#2f4070" :weight bold :background "#a0a0a0")))) (isearch-fail ((t (:foreground "#232333" :background "#73acac")))) (lazy-highlight ((t (:foreground "#2f4070" :weight bold :background "#c7c7c7")))) (match ((t (:background "#d4d4d4" :foreground "#205070" :weight bold)))) (show-paren-match ((t (:background "#909090" :weight bold)))) (show-paren-mismatch ((t (:foreground "#235c5c" :background "#909090" :weight bold)))))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_workflow_and_diagnostic_face_specs_match_exactly() {
    let elisp_form = r##"(let ((settings
                    (get
                     'anti-zenburn
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((setting
                   (seq-find
                    (lambda (candidate)
                      (eq
                       (cadr candidate)
                       face))
                    settings)))
              (list
               face
               (copy-tree
                (nth 3 setting)))))
          '(compilation-info
            compilation-line-number
            compilation-mode-line-exit
            compilation-mode-line-fail
            compilation-mode-line-run
            diff-added
            diff-changed
            diff-removed
            diff-refine-added
            diff-refine-changed
            diff-refine-removed
            diff-header
            diff-file-header
            flycheck-error
            flycheck-warning
            flycheck-info
            flymake-errline
            flymake-warnline
            flymake-infoline
            flyspell-duplicate
            flyspell-incorrect
            term-color-red
            term-color-green
            term-color-blue
            term-default-fg-color
            term-default-bg-color)))"##;
    let expect = expect![[
        r##"OK ((compilation-info ((t (:foreground "#401440" :underline t)))) (compilation-line-number ((t (:foreground "#0f2050")))) (compilation-mode-line-exit ((t (:foreground "#603a60" :weight bold)))) (compilation-mode-line-fail ((t (:foreground "#336c6c" :weight bold)))) (compilation-mode-line-run ((t (:foreground "#0f2050" :weight bold)))) (diff-added ((t (:background "#d0b0d0" :foreground "#603a60")))) (diff-changed ((t (:background "#aaaaee" :foreground "#1f3060")))) (diff-removed ((t (:background "#93cccc" :foreground "#235c5c")))) (diff-refine-added ((t (:background "#c0a0c0" :foreground "#502750")))) (diff-refine-changed ((t (:background "#7777ee" :foreground "#0f2050")))) (diff-refine-removed ((t (:background "#83bcbc" :foreground "#134c4c")))) (diff-header ((((class color) (min-colors 89)) (:background "#a0a0a0")) (t (:background "#232333" :foreground "#c0c0c0")))) (diff-file-header ((((class color) (min-colors 89)) (:background "#a0a0a0" :foreground "#232333" :weight bold)) (t (:background "#232333" :foreground "#c0c0c0" :weight bold)))) (flycheck-error ((((supports :underline (:style wave))) (:underline (:style wave :color "#437c7c") :inherit qunspecified)) (t (:foreground "#437c7c" :weight bold :underline t)))) (flycheck-warning ((((supports :underline (:style wave))) (:underline (:style wave :color "#0f2050") :inherit unspecified)) (t (:foreground "#0f2050" :weight bold :underline t)))) (flycheck-info ((((supports :underline (:style wave))) (:underline (:style wave :color "#6c1f1c") :inherit unspecified)) (t (:foreground "#6c1f1c" :weight bold :underline t)))) (flymake-errline ((((supports :underline (:style wave))) (:underline (:style wave :color "#336c6c") :inherit unspecified :foreground unspecified :background unspecified)) (t (:foreground "#437c7c" :weight bold :underline t)))) (flymake-warnline ((((supports :underline (:style wave))) (:underline (:style wave :color "#205070") :inherit unspecified :foreground unspecified :background unspecified)) (t (:foreground "#205070" :weight bold :underline t)))) (flymake-infoline ((((supports :underline (:style wave))) (:underline (:style wave :color "#806080") :inherit unspecified :foreground unspecified :background unspecified)) (t (:foreground "#a080a0" :weight bold :underline t)))) (flyspell-duplicate ((((supports :underline (:style wave))) (:underline (:style wave :color "#205070") :inherit unspecified)) (t (:foreground "#205070" :weight bold :underline t)))) (flyspell-incorrect ((((supports :underline (:style wave))) (:underline (:style wave :color "#336c6c") :inherit unspecified)) (t (:foreground "#437c7c" :weight bold :underline t)))) (term-color-red ((t (:foreground "#538c8c" :background "#73acac")))) (term-color-green ((t (:foreground "#806080" :background "#603a60")))) (term-color-blue ((t (:foreground "#834744" :background "#b38f8c")))) (term-default-fg-color ((t (:inherit term-color-white)))) (term-default-bg-color ((t (:inherit term-color-black)))))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_external_package_specs_cover_inheritance_and_literal_colors() {
    let elisp_form = r##"(let ((settings
                    (get
                     'anti-zenburn
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((setting
                   (seq-find
                    (lambda (candidate)
                      (eq
                       (cadr candidate)
                       face))
                    settings)))
              (list
               face
               (copy-tree
                (nth 3 setting)))))
          '(anzu-replace-to
            aw-leading-char-face
            cfw:face-holiday
            company-tooltip
            company-tooltip-selection
            company-preview-common
            ediff-current-diff-A
            ediff-fine-diff-B
            helm-grep-match
            helm-match
            magit-diff-added
            magit-diff-added-highlight
            magit-section-highlight
            org-document-title
            org-document-info
            org-level-1
            org-level-8
            powerline-active1
            powerline-inactive2
            web-mode-server-comment-face
            web-mode-server-string-face
            web-mode-warning-face)))"##;
    let expect = expect![[
        r##"OK ((anzu-replace-to ((t (:inherit anzu-replace-highlight :foreground "#0f2050")))) (aw-leading-char-face ((t (:inherit aw-mode-line-face)))) (cfw:face-holiday ((t (:inherit cfw:face-sunday)))) (company-tooltip ((t (:foreground "#232333" :background "#b0b0b0")))) (company-tooltip-selection ((t (:foreground "#232333" :background "#d4d4d4")))) (company-preview-common ((t (:foreground "#603a60" :background "#d4d4d4")))) (ediff-current-diff-A ((t (:inherit diff-removed)))) (ediff-fine-diff-B ((t (:inherit diff-refine-added :weight bold)))) (helm-grep-match ((t (:foreground nil :background nil :inherit helm-match)))) (helm-match ((t (:foreground "#205070" :background "#d4d4d4" :weight bold)))) (magit-diff-added ((t (:inherit diff-added)))) (magit-diff-added-highlight ((t (:inherit diff-refine-added)))) (magit-section-highlight ((t (:background "#b6b6b6")))) (org-document-title ((t (:foreground "#732f2c")))) (org-document-info ((t (:foreground "#732f2c")))) (org-level-1 ((t (:foreground "#205070")))) (org-level-8 ((t (:foreground "#b38f8c")))) (powerline-active1 ((t (:background "#c7c7c7" :inherit mode-line)))) (powerline-inactive2 ((t (:background "#909090" :inherit mode-line-inactive)))) (web-mode-server-comment-face ((t (:inherit web-mode-comment-face)))) (web-mode-server-string-face ((t (:inherit web-mode-string-face)))) (web-mode-warning-face ((t (:inherit font-lock-warning-face)))))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_preserves_legacy_malformed_specs_as_observable_data() {
    let elisp_form = r##"(let ((settings
                    (get
                     'anti-zenburn
                     'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((setting
                   (seq-find
                    (lambda (candidate)
                      (eq
                       (cadr candidate)
                       face))
                    settings)))
              (list
               face
               (copy-tree
                (nth 3 setting)))))
          '(diff-hl-insert
            cperl-array-face
            cperl-hash-face
            ledger-font-pending-face
            realgud-backtrace-number
            flycheck-error
            ruler-mode-column-number
            ruler-mode-fill-column
            ruler-mode-goal-column
            ruler-mode-comment-column
            ruler-mode-tab-stop)))"##;
    let expect = expect![[
        r##"OK ((diff-hl-insert ((((class color) (min-colors 89)) :foreground "#704d70" :background "#a080a0"))) (cperl-array-face ((t (:foreground "#0f2050" :backgorund "#c0c0c0")))) (cperl-hash-face ((t (:foreground "#1f3060" :background "#c0c0c0")))) (ledger-font-pending-face ((t (:foreground "#205070" weight: normal)))) (realgud-backtrace-number ((t (:foreground "#0f2050" :weight bold)))) (flycheck-error ((((supports :underline (:style wave))) (:underline (:style wave :color "#437c7c") :inherit qunspecified)) (t (:foreground "#437c7c" :weight bold :underline t)))) (ruler-mode-column-number ((t (:inherit 'ruler-mode-default :foreground "#232333")))) (ruler-mode-fill-column ((t (:inherit 'ruler-mode-default :foreground "#0f2050")))) (ruler-mode-goal-column ((t (:inherit 'ruler-mode-fill-column)))) (ruler-mode-comment-column ((t (:inherit 'ruler-mode-fill-column)))) (ruler-mode-tab-stop ((t (:inherit 'ruler-mode-fill-column)))))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_enabled_specs_apply_to_optional_faces_defined_late() {
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'anti-zenburn
            t)
           (eval
            '(defface
                 anzu-replace-highlight
               '((t
                  (:foreground "parent-fallback"
                   :background "parent-background")))
               "Parity parent face."))
           (eval
            '(defface
                 anzu-replace-to
               '((t
                  (:foreground "child-fallback")))
               "Parity child face."))
           (eval
            '(defface
                 company-tooltip
               '((t
                  (:foreground "fallback"
                   :background "fallback")))
               "Parity Company face."))
           (eval
            '(defface
                 web-mode-comment-face
               '((t
                  (:foreground "web-parent")))
               "Parity web parent face."))
           (eval
            '(defface
                 web-mode-server-comment-face
               '((t
                  (:foreground "web-child")))
               "Parity web child face."))
           (mapcar
            (lambda (face)
              (list
               face
               (face-attribute
                face :inherit nil nil)
               (face-attribute
                face :foreground nil t)
               (face-attribute
                face :background nil t)
               (face-attribute
                face :weight nil t)))
            '(anzu-replace-highlight
              anzu-replace-to
              company-tooltip
              web-mode-comment-face
              web-mode-server-comment-face)))
       (when
           (custom-theme-enabled-p
            'anti-zenburn)
         (disable-theme
          'anti-zenburn)))"##;
    let expect = expect![[
        r##"OK ((anzu-replace-highlight unspecified "parent-fallback" "parent-background" unspecified) (anzu-replace-to anzu-replace-highlight "#0f2050" "parent-background" unspecified) (company-tooltip unspecified "#232333" "#b0b0b0" unspecified) (web-mode-comment-face font-lock-comment-face "#806080" unspecified unspecified) (web-mode-server-comment-face web-mode-comment-face "#806080" unspecified unspecified))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_inheritance_resolves_across_real_optional_face_chains() {
    let prelude = r##"(defface anzu-replace-highlight
  '((t (:foreground "parent-fallback"
        :background "parent-background")))
  "Parity parent face.")
(defface anzu-replace-to
  '((t (:foreground "child-fallback")))
  "Parity child face.")
(defface web-mode-comment-face
  '((t (:foreground "web-parent"
        :slant italic)))
  "Parity web parent face.")
(defface web-mode-server-comment-face
  '((t (:foreground "web-child")))
  "Parity web child face.")
(defface diff-added
  '((t (:foreground "diff-parent")))
  "Parity diff parent face.")
(defface magit-diff-added
  '((t (:foreground "magit-child")))
  "Parity Magit child face.")"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'anti-zenburn
            t)
           (mapcar
            (lambda (face)
              (list
               face
               (face-attribute
                face :inherit nil nil)
               (face-attribute
                face :foreground nil nil)
               (face-attribute
                face :foreground nil t)
               (face-attribute
                face :background nil t)
               (face-attribute
                face :slant nil t)))
            '(anzu-replace-to
              web-mode-server-comment-face
              magit-diff-added)))
       (when
           (custom-theme-enabled-p
            'anti-zenburn)
         (disable-theme
          'anti-zenburn)))"##;
    let expect = expect![[
        r##"OK ((anzu-replace-to anzu-replace-highlight "#0f2050" "#0f2050" "parent-background" unspecified) (web-mode-server-comment-face web-mode-comment-face unspecified "#806080" unspecified unspecified) (magit-diff-added diff-added unspecified "#603a60" "#d0b0d0" unspecified))"##
    ]];

    assert_anti_zenburn_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_legacy_face_specs_have_exact_runtime_application_behavior() {
    let prelude = r##"(defface diff-hl-insert
  '((t (:foreground "diff-fallback")))
  "Parity Diff-HL face.")
(defface cperl-array-face
  '((t (:foreground "array-fallback"
        :background "array-background")))
  "Parity CPerl face.")
(defface ledger-font-pending-face
  '((t (:foreground "ledger-fallback"
        :weight light)))
  "Parity Ledger face.")
(defface ruler-mode-default
  '((t (:foreground "ruler-parent")))
  "Parity ruler parent face.")
(defface ruler-mode-fill-column
  '((t (:foreground "ruler-fallback")))
  "Parity ruler child face.")
(defface flycheck-error
  '((t (:foreground "flycheck-fallback")))
  "Parity Flycheck face.")"##;
    let elisp_form = r##"(unwind-protect
         (condition-case error-data
             (progn
               (load-theme
                'anti-zenburn
                t)
               (mapcar
                (lambda (face)
                  (list
                   face
                   (face-attribute
                    face :inherit nil nil)
                   (face-attribute
                    face :foreground nil t)
                   (face-attribute
                    face :background nil t)
                   (face-attribute
                    face :weight nil t)))
                '(diff-hl-insert
                  cperl-array-face
                  ledger-font-pending-face
                  ruler-mode-fill-column
                  flycheck-error)))
           (error
            (list
             'signal
             (car error-data)
             (error-message-string
              error-data))))
       (when
           (custom-theme-enabled-p
            'anti-zenburn)
         (disable-theme
          'anti-zenburn)))"##;
    let expect = expect![[
        r##"OK ((diff-hl-insert unspecified "#704d70" "#a080a0" unspecified) (cperl-array-face unspecified "#0f2050" unspecified unspecified) (ledger-font-pending-face unspecified "#205070" unspecified unspecified) (ruler-mode-fill-column 'ruler-mode-default "#0f2050" unspecified unspecified) (flycheck-error unspecified "#437c7c" unspecified bold))"##
    ]];

    assert_anti_zenburn_theme_with_prelude_parity(prelude, elisp_form, expect);
}
