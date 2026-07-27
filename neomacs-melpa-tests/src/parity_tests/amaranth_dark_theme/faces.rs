use expect_test::expect;

use super::assert_amaranth_dark_theme_parity;

#[test]
fn core_calendar_compilation_completion_custom_diff_dired_and_ebrowse_specs_match_exactly() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(border
                  cursor
                  default
                  fringe
                  vertical-border
                  link
                  link-visited
                  match
                  shadow
                  minibuffer-prompt
                  region
                  secondary-selection
                  trailing-whitespace
                  tooltip
                  holiday-face
                  compilation-info
                  compilation-warning
                  compilation-error
                  compilation-mode-line-fail
                  compilation-mode-line-exit
                  completions-annotations
                  custom-state
                  diff-removed
                  diff-added
                  dired-directory
                  dired-ignored
                  ebrowse-root-class
                  ebrowse-progress)))"##;
    let expect = expect![[
        r##"OK ((border ((t (:background "#080808" :foreground "#302d2d")))) (cursor ((t (:background "#ffd966")))) (default ((t (:foreground "#e4e4ef" :background "#000000")))) (fringe ((t (:background nil :foreground "#302d2d")))) (vertical-border ((t (:foreground "#302d2d")))) (link ((t (:foreground "#97a1b5" :underline t)))) (link-visited ((t (:foreground "#a64d79" :underline t)))) (match ((t (:background "#7b7171")))) (shadow ((t (:foreground "#7b7171")))) (minibuffer-prompt ((t (:foreground "#97a1b5")))) (region ((t (:background "#4f4949" :foreground nil)))) (secondary-selection ((t (:background "#4f4949" :foreground nil)))) (trailing-whitespace ((t (:foreground "#000000" :background "#a02e2e")))) (tooltip ((t (:background "#7b7171" :foreground "#ffffff")))) (holiday-face ((t (:foreground "#a02e2e")))) (compilation-info ((t (:foreground "#598b43" :inherit unspecified)))) (compilation-warning ((t (:foreground "#7b7171" :bold t :inherit unspecified)))) (compilation-error ((t (:foreground "#c81a1a")))) (compilation-mode-line-fail ((t (:foreground "#a02e2e" :weight bold :inherit unspecified)))) (compilation-mode-line-exit ((t (:foreground "#598b43" :weight bold :inherit unspecified)))) (completions-annotations ((t (:inherit 'shadow)))) (custom-state ((t (:foreground "#598b43")))) (diff-removed ((t (:foreground "#c81a1a" :background nil)))) (diff-added ((t (:foreground "#598b43" :background nil)))) (dired-directory ((t (:foreground "#97a1b5" :weight bold)))) (dired-ignored ((t (:foreground "#959da3" :inherit unspecified)))) (ebrowse-root-class ((t (:foreground "#97a1b5" :weight bold)))) (ebrowse-progress ((t (:background "#97a1b5")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn every_font_lock_face_spec_matches_the_high_contrast_syntax_palette() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(font-lock-builtin-face
                  font-lock-comment-face
                  font-lock-comment-delimiter-face
                  font-lock-constant-face
                  font-lock-doc-face
                  font-lock-doc-string-face
                  font-lock-function-name-face
                  font-lock-keyword-face
                  font-lock-preprocessor-face
                  font-lock-reference-face
                  font-lock-string-face
                  font-lock-type-face
                  font-lock-variable-name-face
                  font-lock-warning-face)))"##;
    let expect = expect![[
        r##"OK ((font-lock-builtin-face ((t (:foreground "#ffd966")))) (font-lock-comment-face ((t (:foreground "#7b7171")))) (font-lock-comment-delimiter-face ((t (:foreground "#7b7171")))) (font-lock-constant-face ((t (:foreground "#959da3")))) (font-lock-doc-face ((t (:foreground "#598b43")))) (font-lock-doc-string-face ((t (:foreground "#598b43")))) (font-lock-function-name-face ((t (:foreground "#97a1b5")))) (font-lock-keyword-face ((t (:foreground "#ffd966" :bold t)))) (font-lock-preprocessor-face ((t (:foreground "#959da3")))) (font-lock-reference-face ((t (:foreground "#959da3")))) (font-lock-string-face ((t (:foreground "#598b43")))) (font-lock-type-face ((t (:foreground "#959da3")))) (font-lock-variable-name-face ((t (:foreground "#f4f4ff")))) (font-lock-warning-face ((t (:foreground "#a02e2e")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn flymake_and_flyspell_specs_preserve_both_wave_underline_and_terminal_fallback_branches() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list
                     face
                     (length (nth 3 setting))
                     (copy-tree (nth 3 setting)))))
                '(flymake-errline
                  flymake-warnline
                  flymake-infoline
                  flyspell-incorrect
                  flyspell-duplicate)))"##;
    let expect = expect![[
        r##"OK ((flymake-errline 2 ((((supports :underline (:style wave))) (:underline (:style wave :color "#a02e2e") :foreground unspecified :background unspecified :inherit unspecified)) (t (:foreground "#a02e2e" :weight bold :underline t)))) (flymake-warnline 2 ((((supports :underline (:style wave))) (:underline (:style wave :color "#ffd966") :foreground unspecified :background unspecified :inherit unspecified)) (t (:foreground "#ffd966" :weight bold :underline t)))) (flymake-infoline 2 ((((supports :underline (:style wave))) (:underline (:style wave :color "#598b43") :foreground unspecified :background unspecified :inherit unspecified)) (t (:foreground "#598b43" :weight bold :underline t)))) (flyspell-incorrect 2 ((((supports :underline (:style wave))) (:underline (:style wave :color "#a02e2e") :inherit unspecified)) (t (:foreground "#a02e2e" :weight bold :underline t)))) (flyspell-duplicate 2 ((((supports :underline (:style wave))) (:underline (:style wave :color "#ffd966") :inherit unspecified)) (t (:foreground "#ffd966" :weight bold :underline t)))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn ido_info_highlight_and_line_number_specs_match_exactly() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(ido-first-match
                  ido-only-match
                  ido-subdir
                  info-xref
                  info-visited
                  highlight
                  highlight-current-line-face
                  line-number
                  line-number-current-line)))"##;
    let expect = expect![[
        r##"OK ((ido-first-match ((t (:foreground "#ffd966" :bold nil)))) (ido-only-match ((t (:foreground "#7b7171" :weight bold)))) (ido-subdir ((t (:foreground "#97a1b5" :weight bold)))) (info-xref ((t (:foreground "#97a1b5")))) (info-visited ((t (:foreground "#a64d79")))) (highlight ((t (:background "#101010" :foreground nil)))) (highlight-current-line-face ((t (:background "#101010" :foreground nil)))) (line-number ((t (:inherit default :foreground "#7b7171")))) (line-number-current-line ((t (:inherit line-number :foreground "#ffd966")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn magit_message_mode_line_and_neotree_specs_match_exactly() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(magit-branch
                  magit-diff-hunk-header
                  magit-diff-file-header
                  magit-log-sha1
                  magit-log-author
                  magit-log-head-label-remote
                  magit-log-head-label-local
                  magit-log-head-label-tags
                  magit-log-head-label-head
                  magit-item-highlight
                  magit-tag
                  magit-blame-heading
                  message-header-name
                  mode-line
                  mode-line-buffer-id
                  mode-line-inactive
                  neo-dir-link-face)))"##;
    let expect = expect![[
        r##"OK ((magit-branch ((t (:foreground "#97a1b5")))) (magit-diff-hunk-header ((t (:background "#302d2d")))) (magit-diff-file-header ((t (:background "#7b7171")))) (magit-log-sha1 ((t (:foreground "#c81a1a")))) (magit-log-author ((t (:foreground "#7b7171")))) (magit-log-head-label-remote ((t (:foreground "#598b43" :background "#101010")))) (magit-log-head-label-local ((t (:foreground "#97a1b5" :background "#101010")))) (magit-log-head-label-tags ((t (:foreground "#ffd966" :background "#101010")))) (magit-log-head-label-head ((t (:foreground "#e4e4ef" :background "#101010")))) (magit-item-highlight ((t (:background "#101010")))) (magit-tag ((t (:foreground "#ffd966" :background "#000000")))) (magit-blame-heading ((t (:background "#101010" :foreground "#e4e4ef")))) (message-header-name ((t (:foreground "#598b43")))) (mode-line ((t (:background "#101010" :foreground "#ffffff")))) (mode-line-buffer-id ((t (:background "#101010" :foreground "#ffffff")))) (mode-line-inactive ((t (:background "#101010" :foreground "#959da3")))) (neo-dir-link-face ((t (:foreground "#97a1b5")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn org_search_shell_paren_speedbar_and_which_function_specs_match_exactly() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(org-agenda-structure
                  org-column
                  org-column-title
                  org-done
                  org-todo
                  org-upcoming-deadline
                  isearch
                  isearch-fail
                  isearch-lazy-highlight-face
                  sh-quoted-exec
                  show-paren-match-face
                  show-paren-mismatch-face
                  speedbar-directory-face
                  speedbar-file-face
                  speedbar-highlight-face
                  speedbar-selected-face
                  speedbar-tag-face
                  which-func)))"##;
    let expect = expect![[
        r##"OK ((org-agenda-structure ((t (:foreground "#97a1b5")))) (org-column ((t (:background "#080808")))) (org-column-title ((t (:background "#080808" :underline t :weight bold)))) (org-done ((t (:foreground "#598b43")))) (org-todo ((t (:foreground "#c73c3f")))) (org-upcoming-deadline ((t (:foreground "#ffd966")))) (isearch ((t (:foreground "#000000" :background "#f5f5f5")))) (isearch-fail ((t (:foreground "#000000" :background "#a02e2e")))) (isearch-lazy-highlight-face ((t (:foreground "#f4f4ff" :background "#616775")))) (sh-quoted-exec ((t (:foreground "#c81a1a")))) (show-paren-match-face ((t (:background "#7b7171")))) (show-paren-mismatch-face ((t (:background "#c73c3f")))) (speedbar-directory-face ((t (:foreground "#97a1b5" :weight bold)))) (speedbar-file-face ((t (:foreground "#e4e4ef")))) (speedbar-highlight-face ((t (:background "#101010")))) (speedbar-selected-face ((t (:foreground "#a02e2e")))) (speedbar-tag-face ((t (:foreground "#ffd966")))) (which-func ((t (:foreground "#a64d79")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn whitespace_and_tab_bar_specs_match_every_background_foreground_and_inheritance_choice() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(whitespace-space
                  whitespace-tab
                  whitespace-hspace
                  whitespace-line
                  whitespace-newline
                  whitespace-trailing
                  whitespace-empty
                  whitespace-indentation
                  whitespace-space-after-tab
                  whitespace-space-before-tab
                  tab-bar
                  tab-bar-tab
                  tab-bar-tab-inactive)))"##;
    let expect = expect![[
        r##"OK ((whitespace-space ((t (:background "#000000" :foreground "#101010")))) (whitespace-tab ((t (:background "#000000" :foreground "#101010")))) (whitespace-hspace ((t (:background "#000000" :foreground "#302d2d")))) (whitespace-line ((t (:background "#302d2d" :foreground "#c81a1a")))) (whitespace-newline ((t (:background "#000000" :foreground "#302d2d")))) (whitespace-trailing ((t (:background "#a02e2e" :foreground "#a02e2e")))) (whitespace-empty ((t (:background "#ffd966" :foreground "#ffd966")))) (whitespace-indentation ((t (:background "#ffd966" :foreground "#a02e2e")))) (whitespace-space-after-tab ((t (:background "#ffd966" :foreground "#ffd966")))) (whitespace-space-before-tab ((t (:background "#7b7171" :foreground "#7b7171")))) (tab-bar ((t (:background "#101010" :foreground "#7b7171")))) (tab-bar-tab ((t (:background nil :foreground "#ffd966" :weight bold)))) (tab-bar-tab-inactive ((t (:background nil)))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn terminal_ansi_color_specs_match_foreground_and_background_pairs_exactly() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(term-color-black
                  term-color-red
                  term-color-green
                  term-color-blue
                  term-color-yellow
                  term-color-magenta
                  term-color-cyan
                  term-color-white)))"##;
    let expect = expect![[
        r##"OK ((term-color-black ((t (:foreground "#4f4949" :background "#7b7171")))) (term-color-red ((t (:foreground "#c73c3f" :background "#c73c3f")))) (term-color-green ((t (:foreground "#598b43" :background "#598b43")))) (term-color-blue ((t (:foreground "#97a1b5" :background "#97a1b5")))) (term-color-yellow ((t (:foreground "#ffd966" :background "#ffd966")))) (term-color-magenta ((t (:foreground "#a64d79" :background "#a64d79")))) (term-color-cyan ((t (:foreground "#959da3" :background "#959da3")))) (term-color-white ((t (:foreground "#e4e4ef" :background "#ffffff")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn company_completion_specs_match_normal_selected_common_preview_and_scrollbar_states() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(company-tooltip
                  company-tooltip-annotation
                  company-tooltip-annotation-selection
                  company-tooltip-selection
                  company-tooltip-mouse
                  company-tooltip-common
                  company-tooltip-common-selection
                  company-scrollbar-fg
                  company-scrollbar-bg
                  company-preview
                  company-preview-common)))"##;
    let expect = expect![[
        r##"OK ((company-tooltip ((t (:foreground "#e4e4ef" :background "#101010")))) (company-tooltip-annotation ((t (:foreground "#7b7171" :background "#101010")))) (company-tooltip-annotation-selection ((t (:foreground "#7b7171" :background "#080808")))) (company-tooltip-selection ((t (:foreground "#e4e4ef" :background "#080808")))) (company-tooltip-mouse ((t (:background "#080808")))) (company-tooltip-common ((t (:foreground "#598b43")))) (company-tooltip-common-selection ((t (:foreground "#598b43")))) (company-scrollbar-fg ((t (:background "#080808")))) (company-scrollbar-bg ((t (:background "#302d2d")))) (company-preview ((t (:background "#598b43")))) (company-preview-common ((t (:foreground "#598b43" :background "#080808")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn proof_general_and_orderless_specs_match_locked_and_ranked_match_colors() {
    let elisp_form = r##"(let ((settings
                    (get 'amaranth-dark 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((setting
                         (catch 'found
                           (dolist (entry settings)
                             (when
                                 (and
                                  (eq (car entry) 'theme-face)
                                  (eq (cadr entry) face))
                               (throw 'found entry))))))
                    (list face (copy-tree (nth 3 setting)))))
                '(proof-locked-face
                  orderless-match-face-0
                  orderless-match-face-1
                  orderless-match-face-2
                  orderless-match-face-3)))"##;
    let expect = expect![[
        r##"OK ((proof-locked-face ((t (:background "#303540")))) (orderless-match-face-0 ((t (:foreground "#ffd966")))) (orderless-match-face-1 ((t (:foreground "#598b43")))) (orderless-match-face-2 ((t (:foreground "#7b7171")))) (orderless-match-face-3 ((t (:foreground "#959da3")))))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}
