use expect_test::expect;

use super::assert_ancient_one_dark_theme_parity;

#[test]
fn ancient_one_dark_theme_core_editor_faces_preserve_every_literal_attribute() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (copy-tree
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq
                   (cadr setting)
                   face))
                settings)))))
          '(default
            font-lock-builtin-face
            font-lock-comment-face
            font-lock-negation-char-face
            font-lock-reference-face
            font-lock-constant-face
            font-lock-doc-face
            font-lock-function-name-face
            font-lock-keyword-face
            font-lock-string-face
            font-lock-type-face
            font-lock-variable-name-face
            font-lock-warning-face
            region
            highlight
            hl-line
            fringe
            cursor
            show-paren-match-face
            isearch)))"##;
    let expect = expect![[
        r##"OK ((default ((((class color) (min-colors 89)) (:background "#312843" :foreground "#d1cad5")))) (font-lock-builtin-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (font-lock-comment-face ((((class color) (min-colors 89)) (:foreground "#736a8c")))) (font-lock-negation-char-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (font-lock-reference-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (font-lock-constant-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (font-lock-doc-face ((((class color) (min-colors 89)) (:foreground "#736a8c")))) (font-lock-function-name-face ((((class color) (min-colors 89)) (:foreground "#8e7ed9")))) (font-lock-keyword-face ((((class color) (min-colors 89)) (:bold ((class color) (min-colors 89)) :foreground "#8b76bc")))) (font-lock-string-face ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (font-lock-type-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (font-lock-variable-name-face ((((class color) (min-colors 89)) (:foreground "#d1cad5")))) (font-lock-warning-face ((((class color) (min-colors 89)) (:foreground "#fad13d" :background "#413952")))) (region ((((class color) (min-colors 89)) (:background "#d1cad5" :foreground "#312843")))) (highlight ((((class color) (min-colors 89)) (:foreground "#b0aab3" :background "#524a61")))) (hl-line ((((class color) (min-colors 89)) (:background "#413952")))) (fringe ((((class color) (min-colors 89)) (:background "#312843" :foreground "#767278")))) (cursor ((((class color) (min-colors 89)) (:background "#524a61")))) (show-paren-match-face ((((class color) (min-colors 89)) (:background "#fad13d")))) (isearch ((((class color) (min-colors 89)) (:bold t :foreground "#fad13d" :background "#524a61")))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_mode_line_navigation_and_tab_faces_are_exact() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (mapcar
              (lambda (setting)
                (copy-tree
                 (nth 3 setting)))
              (seq-filter
               (lambda (setting)
                 (eq
                  (cadr setting)
                  face))
               settings))))
          '(centaur-tabs-selected
            centaur-tabs-unselected
            mode-line
            mode-line-inactive
            mode-line-buffer-id
            mode-line-highlight
            mode-line-emphasis
            vertical-border
            minibuffer-prompt
            default-italic
            link
            ido-only-match
            ido-first-match
            ivy-current-match
            line-number
            line-number-current-line
            tab-line
            tab-line-tab
            tab-line-tab-inactive
            tab-line-tab-current
            tab-line-highlight)))"##;
    let expect = expect![[
        r##"OK ((centaur-tabs-selected (((((class color) (min-colors 89)) (:background "#312843"))))) (centaur-tabs-unselected (((((class color) (min-colors 89)) (:background "#413952"))))) (mode-line (((((class color) (min-colors 89)) (:box (:line-width 3 :color "#413952") :bold t :foreground "#c0bac4" :background "#413952"))))) (mode-line-inactive (((((class color) (min-colors 89)) (:box (:line-width 1 :color "#413952" :style pressed-button) :foreground "#d1cad5" :background "#312843" :weight normal))))) (mode-line-buffer-id (((((class color) (min-colors 89)) (:bold t :foreground "#8e7ed9" :background nil))))) (mode-line-highlight (((((class color) (min-colors 89)) (:foreground "#8b76bc" :box nil :weight bold))))) (mode-line-emphasis (((((class color) (min-colors 89)) (:foreground "#767278"))))) (vertical-border (((((class color) (min-colors 89)) (:foreground "#767278"))))) (minibuffer-prompt (((((class color) (min-colors 89)) (:bold t :foreground "#8b76bc"))))) (default-italic (((((class color) (min-colors 89)) (:italic t))))) (link (((((class color) (min-colors 89)) (:foreground "#b273b1" :underline t))))) (ido-only-match (((((class color) (min-colors 89)) (:foreground "#fad13d"))))) (ido-first-match (((((class color) (min-colors 89)) (:foreground "#8b76bc" :bold t))))) (ivy-current-match (((((class color) (min-colors 89)) (:foreground "#b0aab3" :inherit highlight :underline t))))) (line-number (((t (:background "#413952" :foreground "#767278"))) ((t (:inherit fringe))))) (line-number-current-line (((t (:background "#413952" :foreground "#d1cad5"))) ((t (:inherit fringe :foreground "white" :weight bold))))) (tab-line (((((class color) (min-colors 89)) (:inherit fringe :box (:line-width 5 :color "#312843")))))) (tab-line-tab (((((class color) (min-colors 89)) (:inherit tab-line))))) (tab-line-tab-inactive (((((class color) (min-colors 89)) (:inherit tab-line :foreground "#736a8c"))))) (tab-line-tab-current (((((class color) (min-colors 89)) (:background "#625c70" :foreground "#d1cad5" :box (:line-width 4 :color "#625c70")))))) (tab-line-highlight (((((class color) (min-colors 89)) (:background "#312843" :foreground "#c0bac4" :box (:line-width 4 :color "#312843")))))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_org_and_latex_faces_cover_real_document_semantics() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (copy-tree
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq
                   (cadr setting)
                   face))
                settings)))))
          '(org-code
            org-hide
            org-level-1
            org-level-2
            org-level-3
            org-level-4
            org-footnote
            org-link
            org-special-keyword
            org-quote
            org-verse
            org-todo
            org-done
            org-block
            org-date
            org-warning
            org-agenda-structure
            org-agenda-date
            org-agenda-date-weekend
            org-agenda-date-today
            org-agenda-done
            org-scheduled
            org-scheduled-today
            org-ellipsis
            org-verbatim
            org-document-info-keyword
            org-sexp-date
            font-latex-bold-face
            font-latex-italic-face
            font-latex-string-face
            font-latex-match-reference-keywords
            font-latex-match-variable-keywords)))"##;
    let expect = expect![[
        r##"OK ((org-code ((((class color) (min-colors 89)) (:foreground "#c0bac4")))) (org-hide ((((class color) (min-colors 89)) (:foreground "#767278")))) (org-level-1 ((((class color) (min-colors 89)) (:bold t :foreground "#c0bac4" :height 1.1)))) (org-level-2 ((((class color) (min-colors 89)) (:bold nil :foreground "#b0aab3")))) (org-level-3 ((((class color) (min-colors 89)) (:bold t :foreground "#767278")))) (org-level-4 ((((class color) (min-colors 89)) (:bold nil :foreground "#625c70")))) (org-footnote ((((class color) (min-colors 89)) (:underline t :foreground "#767278")))) (org-link ((((class color) (min-colors 89)) (:underline t :foreground "#b273b1")))) (org-special-keyword ((((class color) (min-colors 89)) (:foreground "#8e7ed9")))) (org-quote ((((class color) (min-colors 89)) (:inherit org-block :slant italic)))) (org-verse ((((class color) (min-colors 89)) (:inherit org-block :slant italic)))) (org-todo ((((class color) (min-colors 89)) (:box (:line-width 1 :color "#312843") :foreground "#8b76bc" :bold t)))) (org-done ((((class color) (min-colors 89)) (:box (:line-width 1 :color "#312843") :bold t :foreground "#625c70")))) (org-block ((((class color) (min-colors 89)) (:foreground "#b0aab3")))) (org-date ((((class color) (min-colors 89)) (:underline t :foreground "#d1cad5")))) (org-warning ((((class color) (min-colors 89)) (:underline t :foreground "#fad13d")))) (org-agenda-structure ((((class color) (min-colors 89)) (:weight bold :foreground "#b0aab3" :box (:color "#767278") :background "#524a61")))) (org-agenda-date ((((class color) (min-colors 89)) (:foreground "#d1cad5" :height 1.1)))) (org-agenda-date-weekend ((((class color) (min-colors 89)) (:weight normal :foreground "#767278")))) (org-agenda-date-today ((((class color) (min-colors 89)) (:weight bold :foreground "#8b76bc" :height 1.4)))) (org-agenda-done ((((class color) (min-colors 89)) (:foreground "#625c70")))) (org-scheduled ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (org-scheduled-today ((((class color) (min-colors 89)) (:foreground "#8e7ed9" :weight bold :height 1.2)))) (org-ellipsis ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (org-verbatim ((((class color) (min-colors 89)) (:foreground "#767278")))) (org-document-info-keyword ((((class color) (min-colors 89)) (:foreground "#8e7ed9")))) (org-sexp-date ((((class color) (min-colors 89)) (:foreground "#767278")))) (font-latex-bold-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (font-latex-italic-face ((((class color) (min-colors 89)) (:foreground "#d1cad5" :italic t)))) (font-latex-string-face ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (font-latex-match-reference-keywords ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (font-latex-match-variable-keywords ((((class color) (min-colors 89)) (:foreground "#d1cad5")))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_gnus_mu4e_javascript_and_utility_faces_are_exact() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (copy-tree
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq
                   (cadr setting)
                   face))
                settings)))))
          '(gnus-header-content
            gnus-header-from
            gnus-header-name
            gnus-header-subject
            mu4e-view-url-number-face
            mu4e-cited-1-face
            mu4e-cited-7-face
            mu4e-header-marks-face
            ffap
            js2-private-function-call
            js2-jsdoc-html-tag-delimiter
            js2-jsdoc-html-tag-name
            js2-external-variable
            js2-function-param
            js2-jsdoc-value
            js2-private-member
            js3-warning-face
            js3-error-face
            js3-external-variable-face
            js3-function-param-face
            js3-jsdoc-tag-face
            js3-instance-member-face
            warning
            ac-completion-face
            info-quoted-name
            info-string
            icompletep-determined
            slime-repl-inputed-output-face
            trailing-whitespace)))"##;
    let expect = expect![[
        r##"OK ((gnus-header-content ((((class color) (min-colors 89)) (:foreground "#8b76bc")))) (gnus-header-from ((((class color) (min-colors 89)) (:foreground "#d1cad5")))) (gnus-header-name ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (gnus-header-subject ((((class color) (min-colors 89)) (:foreground "#8e7ed9" :bold t)))) (mu4e-view-url-number-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (mu4e-cited-1-face ((((class color) (min-colors 89)) (:foreground "#c0bac4")))) (mu4e-cited-7-face ((((class color) (min-colors 89)) (:foreground "#b0aab3")))) (mu4e-header-marks-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (ffap ((((class color) (min-colors 89)) (:foreground "#767278")))) (js2-private-function-call ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (js2-jsdoc-html-tag-delimiter ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (js2-jsdoc-html-tag-name ((((class color) (min-colors 89)) (:foreground "#d1cad5")))) (js2-external-variable ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (js2-function-param ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (js2-jsdoc-value ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (js2-private-member ((((class color) (min-colors 89)) (:foreground "#b0aab3")))) (js3-warning-face ((((class color) (min-colors 89)) (:underline "#8b76bc")))) (js3-error-face ((((class color) (min-colors 89)) (:underline "#fad13d")))) (js3-external-variable-face ((((class color) (min-colors 89)) (:foreground "#d1cad5")))) (js3-function-param-face ((((class color) (min-colors 89)) (:foreground "#c0bac4")))) (js3-jsdoc-tag-face ((((class color) (min-colors 89)) (:foreground "#8b76bc")))) (js3-instance-member-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (warning ((((class color) (min-colors 89)) (:foreground "#fad13d")))) (ac-completion-face ((((class color) (min-colors 89)) (:underline t :foreground "#8b76bc")))) (info-quoted-name ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (info-string ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (icompletep-determined ((((class color) (min-colors 89)) :foreground "#b273b1"))) (slime-repl-inputed-output-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (trailing-whitespace ((((class color) (min-colors 89)) :foreground nil :background "#fad13d"))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_undo_and_rainbow_faces_retain_legacy_spec_shapes() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (copy-tree
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq
                   (cadr setting)
                   face))
                settings)))))
          '(undo-tree-visualizer-current-face
            undo-tree-visualizer-default-face
            undo-tree-visualizer-unmodified-face
            undo-tree-visualizer-register-face
            rainbow-delimiters-depth-1-face
            rainbow-delimiters-depth-2-face
            rainbow-delimiters-depth-3-face
            rainbow-delimiters-depth-4-face
            rainbow-delimiters-depth-5-face
            rainbow-delimiters-depth-6-face
            rainbow-delimiters-depth-7-face
            rainbow-delimiters-depth-8-face
            rainbow-delimiters-unmatched-face)))"##;
    let expect = expect![[
        r##"OK ((undo-tree-visualizer-current-face ((((class color) (min-colors 89)) :foreground "#b273b1"))) (undo-tree-visualizer-default-face ((((class color) (min-colors 89)) :foreground "#c0bac4"))) (undo-tree-visualizer-unmodified-face ((((class color) (min-colors 89)) :foreground "#d1cad5"))) (undo-tree-visualizer-register-face ((((class color) (min-colors 89)) :foreground "#b273b1"))) (rainbow-delimiters-depth-1-face ((((class color) (min-colors 89)) :foreground "#d1cad5"))) (rainbow-delimiters-depth-2-face ((((class color) (min-colors 89)) :foreground "#b273b1"))) (rainbow-delimiters-depth-3-face ((((class color) (min-colors 89)) :foreground "#d1cad5"))) (rainbow-delimiters-depth-4-face ((((class color) (min-colors 89)) :foreground "#b273b1"))) (rainbow-delimiters-depth-5-face ((((class color) (min-colors 89)) :foreground "#8b76bc"))) (rainbow-delimiters-depth-6-face ((((class color) (min-colors 89)) :foreground "#d1cad5"))) (rainbow-delimiters-depth-7-face ((((class color) (min-colors 89)) :foreground "#b273b1"))) (rainbow-delimiters-depth-8-face ((((class color) (min-colors 89)) :foreground "#d1cad5"))) (rainbow-delimiters-unmatched-face ((((class color) (min-colors 89)) :foreground "#fad13d"))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_magit_and_terminal_faces_match_workflow_palette() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (mapcar
              (lambda (setting)
                (copy-tree
                 (nth 3 setting)))
              (seq-filter
               (lambda (setting)
                 (eq
                  (cadr setting)
                  face))
               settings))))
          '(magit-item-highlight
            magit-section-heading
            magit-hunk-heading
            magit-section-highlight
            magit-hunk-heading-highlight
            magit-diff-context-highlight
            magit-diffstat-added
            magit-diffstat-removed
            magit-process-ok
            magit-process-ng
            magit-branch
            magit-log-author
            magit-hash
            magit-diff-file-header
            lazy-highlight
            term
            term-color-black
            term-color-blue
            term-color-red
            term-color-green
            term-color-yellow
            term-color-magenta
            term-color-cyan
            term-color-white)))"##;
    let expect = expect![[
        r##"OK ((magit-item-highlight (((((class color) (min-colors 89)) :background "#524a61")))) (magit-section-heading (((((class color) (min-colors 89)) (:foreground "#8b76bc" :weight bold))))) (magit-hunk-heading (((((class color) (min-colors 89)) (:background "#524a61"))))) (magit-section-highlight (((((class color) (min-colors 89)) (:background "#413952"))))) (magit-hunk-heading-highlight (((((class color) (min-colors 89)) (:background "#524a61"))))) (magit-diff-context-highlight (((((class color) (min-colors 89)) (:background "#524a61" :foreground "#b0aab3"))))) (magit-diffstat-added (((((class color) (min-colors 89)) (:foreground "#b273b1"))))) (magit-diffstat-removed (((((class color) (min-colors 89)) (:foreground "#d1cad5"))))) (magit-process-ok (((((class color) (min-colors 89)) (:foreground "#8e7ed9" :weight bold))))) (magit-process-ng (((((class color) (min-colors 89)) (:foreground "#fad13d" :weight bold))))) (magit-branch (((((class color) (min-colors 89)) (:foreground "#b273b1" :weight bold))))) (magit-log-author (((((class color) (min-colors 89)) (:foreground "#b0aab3"))))) (magit-hash (((((class color) (min-colors 89)) (:foreground "#c0bac4"))))) (magit-diff-file-header (((((class color) (min-colors 89)) (:foreground "#c0bac4" :background "#524a61"))))) (lazy-highlight (((((class color) (min-colors 89)) (:foreground "#c0bac4" :background "#524a61"))))) (term (((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843"))))) (term-color-black (((((class color) (min-colors 89)) (:foreground "#524a61" :background "#524a61"))) ((((class color) (min-colors 89)) (:foreground "#c0bac4" :background nil))))) (term-color-blue (((((class color) (min-colors 89)) (:foreground "#8e7ed9" :background "#8e7ed9"))))) (term-color-red (((((class color) (min-colors 89)) (:foreground "#8b76bc" :background "#524a61"))))) (term-color-green (((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#524a61"))))) (term-color-yellow (((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#d1cad5"))))) (term-color-magenta (((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#b273b1"))))) (term-color-cyan (((((class color) (min-colors 89)) (:foreground "#f3cb89" :background "#f3cb89"))))) (term-color-white (((((class color) (min-colors 89)) (:foreground "#c0bac4" :background "#c0bac4"))))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_helm_and_company_faces_preserve_all_attributes_and_typos() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (copy-tree
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq
                   (cadr setting)
                   face))
                settings)))))
          '(helm-header
            helm-source-header
            helm-selection
            helm-selection-line
            helm-visible-mark
            helm-candidate-number
            helm-separator
            helm-time-zone-current
            helm-time-zone-home
            helm-buffer-not-saved
            helm-buffer-process
            helm-buffer-saved-out
            helm-buffer-size
            helm-ff-directory
            helm-ff-file
            helm-ff-executable
            helm-ff-invalid-symlink
            helm-ff-symlink
            helm-ff-prefix
            helm-grep-cmd-line
            helm-grep-file
            helm-grep-finish
            helm-grep-lineno
            helm-grep-match
            helm-grep-running
            helm-moccur-buffer
            helm-source-go-package-godoc-description
            helm-bookmark-w3m
            company-echo-common
            company-preview
            company-preview-common
            company-preview-search
            company-scrollbar-bg
            company-scrollbar-fg
            company-tooltip
            company-tooltop-annotation
            company-tooltip-common
            company-tooltip-common-selection
            company-tooltip-mouse
            company-tooltip-selection
            company-template-field)))"##;
    let expect = expect![[
        r##"OK ((helm-header ((((class color) (min-colors 89)) (:foreground "#c0bac4" :background "#312843" :underline nil :box nil)))) (helm-source-header ((((class color) (min-colors 89)) (:foreground "#8b76bc" :background "#312843" :underline nil :weight bold)))) (helm-selection ((((class color) (min-colors 89)) (:background "#413952" :underline nil)))) (helm-selection-line ((((class color) (min-colors 89)) (:background "#413952")))) (helm-visible-mark ((((class color) (min-colors 89)) (:foreground "#312843" :background "#524a61")))) (helm-candidate-number ((((class color) (min-colors 89)) (:foreground "#312843" :background "#d1cad5")))) (helm-separator ((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#312843")))) (helm-time-zone-current ((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#312843")))) (helm-time-zone-home ((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#312843")))) (helm-buffer-not-saved ((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#312843")))) (helm-buffer-process ((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#312843")))) (helm-buffer-saved-out ((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843")))) (helm-buffer-size ((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843")))) (helm-ff-directory ((((class color) (min-colors 89)) (:foreground "#8e7ed9" :background "#312843" :weight bold)))) (helm-ff-file ((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843" :weight normal)))) (helm-ff-executable ((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843" :weight normal)))) (helm-ff-invalid-symlink ((((class color) (min-colors 89)) (:foreground "#fad13d" :background "#312843" :weight bold)))) (helm-ff-symlink ((((class color) (min-colors 89)) (:foreground "#8b76bc" :background "#312843" :weight bold)))) (helm-ff-prefix ((((class color) (min-colors 89)) (:foreground "#312843" :background "#8b76bc" :weight normal)))) (helm-grep-cmd-line ((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843")))) (helm-grep-file ((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843")))) (helm-grep-finish ((((class color) (min-colors 89)) (:foreground "#c0bac4" :background "#312843")))) (helm-grep-lineno ((((class color) (min-colors 89)) (:foreground "#d1cad5" :background "#312843")))) (helm-grep-match ((((class color) (min-colors 89)) (:foreground nil :background nil :inherit helm-match)))) (helm-grep-running ((((class color) (min-colors 89)) (:foreground "#8e7ed9" :background "#312843")))) (helm-moccur-buffer ((((class color) (min-colors 89)) (:foreground "#8e7ed9" :background "#312843")))) (helm-source-go-package-godoc-description ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (helm-bookmark-w3m ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (company-echo-common ((((class color) (min-colors 89)) (:foreground "#312843" :background "#d1cad5")))) (company-preview ((((class color) (min-colors 89)) (:background "#312843" :foreground "#d1cad5")))) (company-preview-common ((((class color) (min-colors 89)) (:foreground "#413952" :foreground "#b0aab3")))) (company-preview-search ((((class color) (min-colors 89)) (:foreground "#b273b1" :background "#312843")))) (company-scrollbar-bg ((((class color) (min-colors 89)) (:background "#524a61")))) (company-scrollbar-fg ((((class color) (min-colors 89)) (:foreground "#8b76bc")))) (company-tooltip ((((class color) (min-colors 89)) (:foreground "#c0bac4" :background "#312843" :bold t)))) (company-tooltop-annotation ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (company-tooltip-common ((((class color) (min-colors 89)) (:foreground "#b0aab3")))) (company-tooltip-common-selection ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (company-tooltip-mouse ((((class color) (min-colors 89)) (:inherit highlight)))) (company-tooltip-selection ((((class color) (min-colors 89)) (:background "#524a61" :foreground "#b0aab3")))) (company-template-field ((((class color) (min-colors 89)) (:inherit region)))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_web_and_java_faces_preserve_inheritance_and_fallback_classes() {
    let elisp_form = r##"(let ((settings
                (get
                 'ancient-one-dark
                 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (copy-tree
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq
                   (cadr setting)
                   face))
                settings)))))
          '(web-mode-builtin-face
            web-mode-comment-face
            web-mode-constant-face
            web-mode-keyword-face
            web-mode-doctype-face
            web-mode-function-name-face
            web-mode-string-face
            web-mode-type-face
            web-mode-html-attr-name-face
            web-mode-html-attr-value-face
            web-mode-warning-face
            web-mode-html-tag-face
            jde-java-font-lock-package-face
            jde-java-font-lock-public-face
            jde-java-font-lock-private-face
            jde-java-font-lock-constant-face
            jde-java-font-lock-modifier-face
            jde-jave-font-lock-protected-face
            jde-java-font-lock-number-face)))"##;
    let expect = expect![[
        r##"OK ((web-mode-builtin-face ((((class color) (min-colors 89)) (:inherit font-lock-builtin-face)))) (web-mode-comment-face ((((class color) (min-colors 89)) (:inherit font-lock-comment-face)))) (web-mode-constant-face ((((class color) (min-colors 89)) (:inherit font-lock-constant-face)))) (web-mode-keyword-face ((((class color) (min-colors 89)) (:foreground "#8b76bc")))) (web-mode-doctype-face ((((class color) (min-colors 89)) (:inherit font-lock-comment-face)))) (web-mode-function-name-face ((((class color) (min-colors 89)) (:inherit font-lock-function-name-face)))) (web-mode-string-face ((((class color) (min-colors 89)) (:foreground "#f3cb89")))) (web-mode-type-face ((((class color) (min-colors 89)) (:inherit font-lock-type-face)))) (web-mode-html-attr-name-face ((((class color) (min-colors 89)) (:foreground "#8e7ed9")))) (web-mode-html-attr-value-face ((((class color) (min-colors 89)) (:foreground "#8b76bc")))) (web-mode-warning-face ((((class color) (min-colors 89)) (:inherit font-lock-warning-face)))) (web-mode-html-tag-face ((((class color) (min-colors 89)) (:foreground "#b273b1")))) (jde-java-font-lock-package-face ((t (:foreground "#d1cad5")))) (jde-java-font-lock-public-face ((t (:foreground "#8b76bc")))) (jde-java-font-lock-private-face ((t (:foreground "#8b76bc")))) (jde-java-font-lock-constant-face ((t (:foreground "#b273b1")))) (jde-java-font-lock-modifier-face ((t (:foreground "#c0bac4")))) (jde-jave-font-lock-protected-face ((t (:foreground "#8b76bc")))) (jde-java-font-lock-number-face ((t (:foreground "#d1cad5")))))"##
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_complete_setting_tree_has_stable_semantic_digest() {
    let elisp_form = r##"(let ((settings
                (reverse
                 (copy-sequence
                  (get
                   'ancient-one-dark
                   'theme-settings)))))
         (list
          (length settings)
          (secure-hash
           'sha256
           (prin1-to-string settings))
          (secure-hash
           'sha256
           (prin1-to-string
            (mapcar
             (lambda (setting)
               (secure-hash
                'sha256
                (prin1-to-string setting)))
             settings)))))"##;
    let expect = expect![[
        r#"OK (202 "7965757ee01d748b945fd3304a02c8c39781d92801c3ebd8b36bf6f41d44b896" "79e3884d89efbe84d58747b41b7363d7d4292f78b67c88a733b5fdfccedf3445")"#
    ]];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}
