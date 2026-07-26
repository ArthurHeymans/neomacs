use expect_test::expect;

use super::{assert_acme_theme_parity, assert_acme_theme_with_prelude_parity};

#[test]
fn acme_theme_core_compilation_search_and_selection_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      settings
                      (-
                       (length settings)
                       43))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((button ((t (:underline t)))) (link ((t (:foreground "#0066cc" :weight normal)))) (highlight ((t (:inherit link :underline t)))) (link-visited ((t (:foreground "#555599" :underline t :weight normal)))) (default ((t (:foreground "#444444" :background "#FFFFE8")))) (cursor ((t (:foreground "#FFFFE8" :background "#444444")))) (escape-glyph ((t (:foreground "#A8EFEB" :bold nil)))) (fringe ((t (:foreground "#444444" :background "#FFFFE8")))) (line-number ((t (:foreground "#444444" :background "#EFEFD8")))) (line-number-current-line ((t (:foreground "#444444" :background "#EFEFD8")))) (header-line ((t (:foreground "#444444" :background "#E1FAFF" :box t)))) (success ((t (:foreground "#005500" :weight normal)))) (warning ((t (:foreground "#880000" :weight normal)))) (error ((t (:foreground "#880000" :bold t)))) (compilation-column-face ((t (:foreground "#888838" :background "#F8FCE8")))) (compilation-column-number ((t (:foreground "#888838" :background "#F8FCE8")))) (compilation-error-face ((t (:foreground "#880000" :weight normal :underline t)))) (compilation-face ((t (:foreground "#444444")))) (compilation-info-face ((t (:foreground "#1054AF")))) (compilation-info ((t (:foreground "#1054AF" :underline t)))) (compilation-line-face ((t (:foreground "#555599")))) (compilation-line-number ((t (:foreground "#888838" :background "#F8FCE8")))) (compilation-message-face ((t (:foreground "#1054AF")))) (compilation-warning-face ((t (:foreground "#888838" :weight normal :underline t)))) (compilation-mode-line-exit ((t (:foreground "#007777" :weight normal)))) (compilation-mode-line-fail ((t (:foreground "#880000" :weight normal)))) (compilation-mode-line-run ((t (:foreground "#555599" :weight normal)))) (grep-context-face ((t (:foreground "#B8B09A")))) (grep-error-face ((t (:foreground "#880000" :weight normal :underline t)))) (grep-hit-face ((t (:foreground "#555599" :weight normal)))) (grep-match-face ((t (:foreground "#007777" :weight normal)))) (match ((t (:background "#007777" :foreground "#A8EFEB")))) (ag-hit-face ((t (:foreground "#005500" :weight normal)))) (ag-match-face ((t (:foreground "#007777" :background "#A8EFEB" :weight normal)))) (isearch ((t (:foreground "#444444" :weight normal :background "#A8EFEB")))) (isearch-fail ((t (:foreground "#444444" :weight normal :background "#880000")))) (lazy-highlight ((t (:foreground "#444444" :weight normal :background "#E1FAFF")))) (menu ((t (:foreground "#FFFFE8" :background "#444444")))) (minibuffer-prompt ((t (:foreground "#444444" :weight normal)))) (region ((((class color) (min-colors 89)) (:foreground "#444444" :background "#E8EB98" :extend nil)))) (secondary-selection ((t (:background "#E8FCE8")))) (trailing-whitespace ((t (:background "#F8E8E8")))) (vertical-border ((t (:foreground "#007777")))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_font_lock_ledger_clojure_and_diff_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      (nthcdr
                       43
                       settings)
                      (-
                       (length settings)
                       83))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((font-lock-builtin-face ((t (:foreground "#444444" :weight normal)))) (font-lock-function-name-face ((t (:foreground "#444444" :weight normal)))) (font-lock-string-face ((t (:foreground "#880000")))) (font-lock-keyword-face ((t (:foreground "#1054AF" :weight bold)))) (font-lock-type-face ((t (:foreground "#444444" :weight bold)))) (font-lock-constant-face ((t (:foreground "#444444" :weight bold)))) (font-lock-variable-name-face ((t (:foreground "#444444" :weight normal)))) (font-lock-comment-face ((t (:foreground "#005500" :italic nil)))) (font-lock-comment-delimiter-face ((t (:foreground "#005500" :italic nil)))) (font-lock-doc-face ((t (:foreground "#888838" :italic nil)))) (font-lock-negation-char-face ((t (:foreground "#880000" :weight normal)))) (font-lock-preprocessor-face ((t (:foreground "#880000" :weight normal)))) (font-lock-regexp-grouping-construct ((t (:foreground "#555599" :weight normal)))) (font-lock-regexp-grouping-backslash ((t (:foreground "#555599" :weight normal)))) (font-lock-warning-face ((t (:foreground "#880000" :weight normal)))) (table-cell ((t (:background "#EFEFD8")))) (ledger-font-directive-face ((t (:foreground "#007777")))) (ledger-font-periodic-xact-face ((t (:inherit ledger-font-directive-face)))) (ledger-font-posting-account-face ((t (:foreground "#1054AF")))) (ledger-font-posting-amount-face ((t (:foreground "#880000")))) (ledger-font-posting-date-face ((t (:foreground "#880000" :weight normal)))) (ledger-font-payee-uncleared-face ((t (:foreground "#555599")))) (ledger-font-payee-cleared-face ((t (:foreground "#444444")))) (ledger-font-payee-pending-face ((t (:foreground "#888838")))) (ledger-font-xact-highlight-face ((t (:background "#EFEFD8")))) (anzu-mode-line ((t (:foreground "#888838" :background "#F8FCE8" :weight normal)))) (clojure-interop-method-face ((t (:inherit font-lock-function-name-face)))) (clojure-test-failure-face ((t (:foreground "#880000" :weight normal :underline t)))) (clojure-test-error-face ((t (:foreground "#880000" :weight normal :underline t)))) (clojure-test-success-face ((t (:foreground "#005500" :weight normal :underline t)))) (diff-added ((((class color) (min-colors 89)) (:foreground "#444444" :background "#E8FCE8")) (t (:foreground "#444444" :background "#E8FCE8")))) (diff-changed ((t (:foreground "#888838")))) (diff-context ((t (:foreground "#444444")))) (diff-removed ((((class color) (min-colors 89)) (:foreground "#444444" :background "#F8E8E8")) (t (:foreground "#444444" :background "#F8E8E8")))) (diff-refine-added ((t :inherit diff-added :background "#E8FCE8" :weight bold :underline t))) (diff-refine-change ((t :inherit diff-changed :weight normal))) (diff-refine-removed ((t :inherit diff-removed :background "#F8E8E8" :weight bold :underline t))) (diff-header ((((class color) (min-colors 89)) (:foreground "#444444" :weight normal)) (t (:foreground "#FFEAFF" :weight normal)))) (diff-file-header ((((class color) (min-colors 89)) (:foreground "#444444" :background "#A8EFEB" :weight normal)) (t (:foreground "#444444" :background "#A8EFEB" :weight normal)))) (diff-hunk-header ((((class color) (min-colors 89)) (:foreground "#005500" :weight normal)) (t (:foreground "#005500" :weight normal)))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_dired_and_dired_subtree_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      (nthcdr
                       83
                       settings)
                      (-
                       (length settings)
                       112))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((dired-directory ((t (:foreground "#1054AF" :weight bold)))) (diredp-display-msg ((t (:foreground "#1054AF")))) (diredp-compressed-file-suffix ((t (:foreground "#555599")))) (diredp-date-time ((t (:foreground "#005500")))) (diredp-deletion ((t (:foreground "#880000")))) (diredp-deletion-file-name ((t (:foreground "#880000")))) (diredp-dir-heading ((t (:foreground "#1054AF" :background "#E1FAFF" :weight bold)))) (diredp-dir-priv ((t (:foreground "#1054AF")))) (diredp-exec-priv ((t (:foreground "#888838")))) (diredp-executable-tag ((t (:foreground "#888838")))) (diredp-file-name ((t (:foreground "#444444")))) (diredp-file-suffix ((t (:foreground "#888838")))) (diredp-flag-mark ((t (:foreground "#007777")))) (diredp-flag-mark-line ((t (:foreground "#007777")))) (diredp-ignored-file-name ((t (:foreground "#CCCCB7")))) (diredp-link-priv ((t (:foreground "#555599")))) (diredp-mode-line-flagged ((t (:foreground "#888838")))) (diredp-mode-line-marked ((t (:foreground "#888838")))) (diredp-no-priv ((t (:foreground "#444444")))) (diredp-number ((t (:foreground "#1054AF")))) (diredp-other-priv ((t (:foreground "#444444")))) (diredp-rare-priv ((t (:foreground "#444444")))) (diredp-read-priv ((t (:foreground "#444444")))) (diredp-symlink ((t (:foreground "#444444" :background "#E1FAFF")))) (diredp-write-priv ((t (:foreground "#444444")))) (diredp-dir-name ((t (:foreground "#1054AF" :weight bold)))) (dired-subtree-depth-1-face ((t (:background "#FFFFE8")))) (dired-subtree-depth-2-face ((t (:background "#FFFFE8")))) (dired-subtree-depth-3-face ((t (:background "#FFFFE8")))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_elfeed_and_erc_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      (nthcdr
                       112
                       settings)
                      (-
                       (length settings)
                       139))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((elfeed-search-date-face ((t (:foreground "#1054AF")))) (elfeed-search-title-face ((t (:foreground "#444444")))) (elfeed-search-unread-title-face ((t (:foreground "#444444")))) (elfeed-search-feed-face ((t (:foreground "#005500")))) (elfeed-search-tag-face ((t (:foreground "#880000")))) (elfeed-search-unread-count-face ((t (:foreground "#444444")))) (erc-default-face ((t (:foreground "#444444")))) (erc-header-line ((t (:inherit header-line)))) (erc-action-face ((t (:inherit erc-default-face)))) (erc-bold-face ((t (:inherit erc-default-face :weight normal)))) (erc-underline-face ((t (:underline t)))) (erc-error-face ((t (:inherit font-lock-warning-face)))) (erc-prompt-face ((t (:foreground "#005500" :background "#E8FCE8" :weight normal)))) (erc-timestamp-face ((t (:foreground "#005500" :background "#E8FCE8")))) (erc-direct-msg-face ((t (:inherit erc-default)))) (erc-notice-face ((t (:foreground "#CCCCB7")))) (erc-highlight-face ((t (:background "#E8EB98")))) (erc-input-face ((t (:foreground "#444444" :background "#EFEFD8")))) (erc-current-nick-face ((t (:foreground "#444444" :background "#A8EFEB" :weight normal :box (:line-width 1 :style released-button))))) (erc-nick-default-face ((t (:weight normal :background "#EFEFD8")))) (erc-my-nick-face ((t (:foreground "#444444" :background "#A8EFEB" :weight normal :box (:line-width 1 :style released-button))))) (erc-nick-msg-face ((t (:inherit erc-default)))) (erc-fool-face ((t (:inherit erc-default)))) (erc-pal-face ((t (:foreground "#555599" :weight normal)))) (erc-dangerous-host-face ((t (:inherit font-lock-warning-face)))) (erc-keyword-face ((t (:foreground "#888838" :weight normal)))) (evil-search-highlight-persist-highlight-face ((t (:inherit lazy-highlight)))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_completion_highlight_js2_and_lsp_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      (nthcdr
                       139
                       settings)
                      (-
                       (length settings)
                       177))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((flx-highlight-face ((t (:foreground "#888838" :background "#E8FCE8" :weight normal :underline t)))) (company-tooltip ((t (:background "#E1FAFF")))) (company-tooltip-selection ((t (:background "#A8EFEB")))) (company-tooltip-common ((t (:foreground "#1054AF" :bold t)))) (company-tooltip-annotation ((t (:foreground "#888838" :italic t)))) (company-scrollbar-fg ((t (:background "#007777")))) (company-scrollbar-bg ((t (:background "#A8EFEB")))) (company-preview-common ((t (:foreground "#444444" :background "#A8EFEB")))) (highlight-symbol-face ((t (:background "#EFEFD8")))) (highlight-numbers-number ((t (:foreground "#1054AF")))) (highlight-operators-face ((t (:foreground "#444444")))) (hl-todo ((t (:inverse-video t)))) (hl-line ((((class color) (min-colors 89)) (:background "#EFEFD8")))) (hl-sexp-face ((((class color) (min-colors 89)) (:background "#EFEFD8")))) (ido-first-match ((t (:foreground "#444444" :weight normal)))) (ido-only-match ((t (:foreground "#444444" :weight normal)))) (ido-subdir ((t (:foreground "#1054AF")))) (ido-indicator ((t (:foreground "#888838")))) (ido-vertical-first-match-face ((t (:foreground "#444444" :background "#A8EFEB" :weight normal)))) (ido-vertical-only-match-face ((t (:foreground "#880000" :background "#F8E8E8" :weight normal)))) (ido-vertical-match-face ((t (:foreground "#444444" :background "#E8FCE8" :weight normal :underline t)))) (indent-guide-face ((t (:foreground "#E8EB98")))) (ivy-current-match ((t (:background "#E1FAFF" :underline t :extend t)))) (ivy-minibuffer-match-face-1 ((t (:background "#EFEFD8")))) (ivy-minibuffer-match-face-2 ((t (:background "#A8EFEB")))) (ivy-minibuffer-match-face-3 ((t (:background "#FFEAFF")))) (ivy-minibuffer-match-face-3 ((t (:background "#E1FAFF")))) (js2-warning ((t (:underline "#888838")))) (js2-error ((t (:foreground "#880000" :weight normal)))) (js2-jsdoc-tag ((t (:foreground "#555599")))) (js2-jsdoc-type ((t (:foreground "#1054AF")))) (js2-jsdoc-value ((t (:foreground "#007777")))) (js2-function-param ((t (:foreground "#444444")))) (js2-external-variable ((t (:foreground "#007777")))) (linum ((t (:foreground "#CCCCB7")))) (lsp-face-highlight-textual ((t (:background "#E5E5D0")))) (lsp-face-highlight-read ((t (:background "#FFEAFF")))) (lsp-face-highlight-write ((t (:background "#E8FCE8")))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_magit_navigation_and_mode_line_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      (nthcdr
                       177
                       settings)
                      (-
                       (length settings)
                       230))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((magit-section-heading ((t (:foreground "#007777" :background "#E1FAFF" :weight normal :underline t)))) (magit-section-highlight ((t (:background "#EFEFD8")))) (magit-section-heading-selection ((t (:background "#E8EB98")))) (magit-filename ((t (:foreground "#444444")))) (magit-hash ((t (:foreground "#888838" :weight normal)))) (magit-tag ((t (:foreground "#555599" :weight normal)))) (magit-refname ((t (:foreground "#555599" :weight normal)))) (magit-head ((t (:foreground "#005500" :weight normal)))) (magit-branch-local ((t (:foreground "#1054AF" :background "#E1FAFF" :weight normal)))) (magit-branch-remote ((t (:foreground "#005500" :background "#E8FCE8" :weight normal)))) (magit-branch-current ((t (:foreground "#007777" :background "#A8EFEB" :weight normal :box (:line-width 1 :color "#007777"))))) (magit-diff-file-heading ((t (:foreground "#444444" :weight normal)))) (magit-diff-file-heading-highlight ((t (:background "#EFEFD8")))) (magit-diff-file-heading-selection ((t (:foreground "#880000" :background "#E8EB98")))) (magit-diff-hunk-heading ((t (:foreground "#1054AF" :background "#E1FAFF" :weight normal :underline t)))) (magit-diff-hunk-heading-highlight ((t (:background "#A8EFEB")))) (magit-diff-added ((t (:foreground "#005500" :background "#E8FCE8")))) (magit-diff-removed ((t (:foreground "#880000" :background "#F8E8E8")))) (magit-diff-context ((t (:foreground "#988D6D" :background nil)))) (magit-diff-added-highlight ((t (:foreground "#005500" :background "#E8FCE8")))) (magit-diff-removed-highlight ((t (:foreground "#880000" :background "#F8E8E8")))) (magit-diff-context-highlight ((t (:foreground "#988D6D" :background "#EFEFD8")))) (magit-diffstat-added ((t (:foreground "#005500" :background "#E8FCE8" :weight normal)))) (magit-diffstat-removed ((t (:foreground "#880000" :background "#F8E8E8" :weight normal)))) (magit-log-author ((t (:foreground "#1054AF" :weight normal)))) (magit-log-date ((t (:foreground "#555599" :weight normal)))) (magit-log-graph ((t (:foreground "#880000" :weight normal)))) (magit-blame-heading ((t (:foreground "#988D6D" :background "#EFEFD8")))) (parenthesis ((t (:foreground "#CCCCB7")))) (pe/file-face ((t (:foreground "#444444")))) (pe/directory-face ((t (:foreground "#1054AF" :weight normal)))) (rainbow-delimiters-depth-1-face ((t (:foreground "#005500")))) (rainbow-delimiters-depth-2-face ((t (:foreground "#1054AF")))) (rainbow-delimiters-depth-3-face ((t (:foreground "#880000")))) (show-paren-mismatch ((t (:foreground "#888838" :background "#880000" :weight normal)))) (show-paren-match ((t (:foreground "#444444" :background "#A8EFEB" :weight normal)))) (mode-line ((((class color) (min-colors 89)) (:foreground "#444444" :background "#E1FAFF" :box t)))) (mode-line-inactive ((t (:foreground "#444444" :background "#E5E5D0" :box t)))) (mode-line-buffer-id ((t (:foreground "#444444" :weight bold)))) (sml/global ((t (:foreground "#444444")))) (sml/modes ((t (:foreground "#005500" :background "#E8FCE8")))) (sml/filename ((t (:foreground "#880000")))) (sml/folder ((t (:foreground "#444444")))) (sml/prefix ((t (:foreground "#444444")))) (sml/read-only ((t (:foreground "#444444")))) (sml/modified ((t (:foreground "#880000" :weight normal)))) (sml/outside-modified ((t (:background "#880000" :foreground "#F8E8E8" :weight normal)))) (sml/line-number ((t (:foreground "#444444" :weight normal)))) (sml/col-number ((t (:foreground "#444444" :weight normal)))) (sml/vc ((t (:foreground "#444444" :weight normal)))) (sml/vc-edited ((t (:foreground "#880000" :weight normal)))) (sml/git ((t (:foreground "#444444" :weight normal)))) (sh-heredoc-face ((t (:foreground "#555599")))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_web_mode_and_which_func_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      (nthcdr
                       230
                       settings)
                      (-
                       (length settings)
                       255))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((web-mode-builtin-face ((t (:inherit font-lock-builtin-face)))) (web-mode-comment-face ((t (:inherit font-lock-comment-face)))) (web-mode-constant-face ((t (:inherit font-lock-constant-face)))) (web-mode-doctype-face ((t (:inherit font-lock-comment-face)))) (web-mode-folded-face ((t (:underline t)))) (web-mode-function-name-face ((t (:foreground "#444444" :weight normal)))) (web-mode-html-attr-name-face ((t (:foreground "#444444")))) (web-mode-html-attr-value-face ((t (:inherit font-lock-string-face)))) (web-mode-html-tag-face ((t (:foreground "#1054AF")))) (web-mode-keyword-face ((t (:inherit font-lock-keyword-face)))) (web-mode-preprocessor-face ((t (:inherit font-lock-preprocessor-face)))) (web-mode-string-face ((t (:inherit font-lock-string-face)))) (web-mode-type-face ((t (:inherit font-lock-type-face)))) (web-mode-variable-name-face ((t (:inherit font-lock-variable-name-face)))) (web-mode-server-background-face ((t (:background "#E8FCE8")))) (web-mode-server-comment-face ((t (:inherit web-mode-comment-face)))) (web-mode-server-string-face ((t (:foreground "#880000")))) (web-mode-symbol-face ((t (:inherit font-lock-constant-face)))) (web-mode-warning-face ((t (:inherit font-lock-warning-face)))) (web-mode-whitespaces-face ((t (:background "#F8E8E8")))) (web-mode-block-face ((t (:background "#E8FCE8")))) (web-mode-current-element-highlight-face ((t (:foreground "#444444" :background "#E1FAFF")))) (web-mode-json-key-face ((((class color) (min-colors 89)) (:inherit font-lock-string-face)))) (web-mode-json-context-face ((((class color) (min-colors 89)) (:inherit font-lock-string-face :bold t)))) (which-func ((t (:foreground "#555599" :background "#FFEAFF")))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_yascroll_org_and_origami_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (butlast
                      (nthcdr
                       255
                       settings)
                      (-
                       (length settings)
                       284))))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((yascroll:thumb-text-area ((t (:background "#E8EB98")))) (yascroll:thumb-fringe ((t (:background "#FFFFE8" :foreground "#FFFFE8" :box (:line-width 1 :style released-button))))) (org-level-1 ((t (:background "#E1FAFF" :foreground "#1054AF" :weight bold :overline t)))) (org-level-2 ((t (:background "#E1FAFF" :foreground "#007777" :weight bold :overline t)))) (org-level-3 ((t (:background "#E1FAFF" :foreground "#1054AF" :weight bold :overline t)))) (org-level-4 ((t (:background "#E1FAFF" :foreground "#007777")))) (org-level-5 ((t (:background "#E1FAFF" :foreground "#1054AF")))) (org-level-6 ((t (:background "#E1FAFF" :foreground "#007777")))) (org-level-7 ((t (:background "#E1FAFF" :foreground "#1054AF")))) (org-level-8 ((t (:background "#E1FAFF" :foreground "#007777")))) (org-document-title ((t (:height 1.2 :foreground "#1054AF" :weight bold :underline t)))) (org-meta-line ((t (:foreground "#005500")))) (org-document-info ((t (:foreground "#007777" :weight normal)))) (org-document-info-keyword ((t (:foreground "#007777")))) (org-todo ((t (:foreground "#888838" :background "#EFEFD8" :weight normal :box (:line-width 1 :style released-button))))) (org-done ((t (:foreground "#005500" :background "#E8FCE8" :weight normal :box (:style released-button))))) (org-date ((t (:foreground "#555599")))) (org-table ((t (:foreground "#555599")))) (org-formula ((t (:foreground "#1054AF" :background "#EFEFD8")))) (org-code ((t (:foreground "#880000" :background "#EFEFD8")))) (org-verbatim ((t (:foreground "#444444" :background "#EFEFD8" :underline t)))) (org-special-keyword ((t (:foreground "#007777")))) (org-agenda-date ((t (:foreground "#007777")))) (org-agenda-structure ((t (:foreground "#555599")))) (org-block ((t (:foreground "#444444" :background "#EFEFD8" :extend t)))) (org-block-background ((t (:background "#EFEFD8" :extend t)))) (org-block-begin-line ((t (:foreground "#B8B09A" :background "#E5E5D0" :italic t :extend t)))) (org-block-end-line ((t (:foreground "#B8B09A" :background "#E5E5D0" :italic t :extend t)))) (origami-fold-replacement-face ((t (:foreground "#880000" :background "#F8E8E8" :box (:line-width -1))))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_git_mail_terminal_and_fill_column_face_specs_match_exactly() {
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'acme
                        'theme-settings))))
                    (slice
                     (nthcdr
                      284
                      settings)))
         (mapcar
          (lambda (setting)
            (list
             (cadr setting)
             (copy-tree
              (nth
               3
               setting))))
          slice))"##;
    let expect = expect![[
        r##"OK ((git-gutter:added ((t (:background "#006600" :foreground "#006600" :weight normal)))) (git-gutter:deleted ((t (:background "#880000" :foreground "#880000" :weight normal)))) (git-gutter:modified ((t (:background "#888838" :foreground "#888838" :weight normal)))) (git-gutter-fr:added ((t (:background "#006600" :foreground "#006600" :weight normal)))) (git-gutter-fr:deleted ((t (:background "#880000" :foreground "#880000" :weight normal)))) (git-gutter-fr:modified ((t (:background "#888838" :foreground "#888838" :weight normal)))) (diff-hl-insert ((t (:background "#006600" :foreground "#006600")))) (diff-hl-delete ((t (:background "#880000" :foreground "#880000")))) (diff-hl-change ((t (:background "#888838" :foreground "#888838")))) (mu4e-header-highlight-face ((t (:background "#E8EB98")))) (mu4e-unread-face ((t (:foreground "#1054AF" :weight normal)))) (mu4e-flagged-face ((t (:foreground "#880000" :background "#F8E8E8" :weight normal)))) (mu4e-compose-separator-face ((t (:foreground "#005500")))) (mu4e-header-value-face ((t (:foreground "#444444")))) (message-header-name ((t (:foreground "#555599" :weight normal)))) (message-header-to ((t (:foreground "#1054AF")))) (message-header-subject ((t (:foreground "#1054AF")))) (message-header-other ((t (:foreground "#1054AF")))) (message-cited-text ((t (:inherit font-lock-comment-face)))) (term ((((class color) (min-colors 89)) (:foreground "#444444" :background "#FFFFE8")))) (term-color-black ((((class color) (min-colors 89)) (:foreground "#444444" :background "#444444")))) (term-color-blue ((((class color) (min-colors 89)) (:foreground "#1054AF" :background "#1054AF")))) (term-color-red ((((class color) (min-colors 89)) (:foreground "#880000" :background "#880000")))) (term-color-green ((((class color) (min-colors 89)) (:foreground "#005500" :background "#005500")))) (term-color-yellow ((((class color) (min-colors 89)) (:foreground "#888838" :background "#888838")))) (term-color-magenta ((((class color) (min-colors 89)) (:foreground "#555599" :background "#555599")))) (term-color-cyan ((((class color) (min-colors 89)) (:foreground "#007777" :background "#007777")))) (term-color-white ((((class color) (min-colors 89)) (:foreground "#444444" :background "#444444")))) (fci-rule-color ((t (:foreground "#E8EBC8")))) (fill-column-indicator ((t (:foreground "#E8EBC8")))))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_palette_usage_has_exact_counts_across_all_face_specs() {
    let elisp_form = r##"(let ((counts
                (mapcar
                 (lambda (color)
                   (cons color 0))
                 '("#FFFFE8"
                   "#EFEFD8"
                   "#E5E5D0"
                   "#444444"
                   "#B8B09A"
                   "#988D6D"
                   "#CCCCB7"
                   "#E8EB98"
                   "#E8EBC8"
                   "#007777"
                   "#A8EFEB"
                   "#880000"
                   "#F8E8E8"
                   "#888838"
                   "#F8FCE8"
                   "#005500"
                   "#006600"
                   "#E8FCE8"
                   "#1054AF"
                   "#E1FAFF"
                   "#555599"
                   "#FFEAFF"
                   "#0066cc"))))
         (cl-labels
             ((visit
               (value)
               (cond
                ((stringp value)
                 (let ((entry
                        (assoc
                         value
                         counts)))
                   (when entry
                     (setcdr
                      entry
                      (1+
                       (cdr entry))))))
                ((consp value)
                 (visit
                  (car value))
                 (visit
                  (cdr value)))
                ((vectorp value)
                 (mapc
                  #'visit
                  value)))))
           (dolist
               (setting
                (get
                 'acme
                 'theme-settings))
             (visit
              (nth
               3
               setting))))
         counts)"##;
    let expect = expect![[
        r##"OK (("#FFFFE8" . 10) ("#EFEFD8" . 20) ("#E5E5D0" . 4) ("#444444" . 74) ("#B8B09A" . 3) ("#988D6D" . 3) ("#CCCCB7" . 4) ("#E8EB98" . 7) ("#E8EBC8" . 2) ("#007777" . 24) ("#A8EFEB" . 16) ("#880000" . 42) ("#F8E8E8" . 12) ("#888838" . 29) ("#F8FCE8" . 4) ("#005500" . 23) ("#006600" . 6) ("#E8FCE8" . 17) ("#1054AF" . 34) ("#E1FAFF" . 20) ("#555599" . 22) ("#FFEAFF" . 4) ("#0066cc" . 1))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_black_foreground_option_rewrites_every_dark_grey_palette_use() {
    let prelude = r##"(setq acme-theme-black-fg t)"##;
    let elisp_form = r##"(let ((settings
                (reverse
                 (copy-sequence
                  (get
                   'acme
                   'theme-settings))))
               black
               grey)
         (dolist
             (setting settings)
           (let ((printed
                  (prin1-to-string
                   (nth
                    3
                    setting))))
             (when
                 (string-match-p
                  (regexp-quote
                   "#000000")
                  printed)
               (push
                (list
                 (cadr setting)
                 (copy-tree
                  (nth
                   3
                   setting)))
                black))
             (when
                 (string-match-p
                  (regexp-quote
                   "#444444")
                  printed)
               (push
                (cadr setting)
                grey))))
         (list
          (nreverse black)
          (nreverse grey)))"##;
    let expect = expect![[
        r##"OK (((default ((t (:foreground "#000000" :background "#FFFFE8")))) (cursor ((t (:foreground "#FFFFE8" :background "#000000")))) (fringe ((t (:foreground "#000000" :background "#FFFFE8")))) (line-number ((t (:foreground "#000000" :background "#EFEFD8")))) (line-number-current-line ((t (:foreground "#000000" :background "#EFEFD8")))) (header-line ((t (:foreground "#000000" :background "#E1FAFF" :box t)))) (compilation-face ((t (:foreground "#000000")))) (isearch ((t (:foreground "#000000" :weight normal :background "#A8EFEB")))) (isearch-fail ((t (:foreground "#000000" :weight normal :background "#880000")))) (lazy-highlight ((t (:foreground "#000000" :weight normal :background "#E1FAFF")))) (menu ((t (:foreground "#FFFFE8" :background "#000000")))) (minibuffer-prompt ((t (:foreground "#000000" :weight normal)))) (region ((((class color) (min-colors 89)) (:foreground "#000000" :background "#E8EB98" :extend nil)))) (font-lock-builtin-face ((t (:foreground "#000000" :weight normal)))) (font-lock-function-name-face ((t (:foreground "#000000" :weight normal)))) (font-lock-type-face ((t (:foreground "#000000" :weight bold)))) (font-lock-constant-face ((t (:foreground "#000000" :weight bold)))) (font-lock-variable-name-face ((t (:foreground "#000000" :weight normal)))) (ledger-font-payee-cleared-face ((t (:foreground "#000000")))) (diff-added ((((class color) (min-colors 89)) (:foreground "#000000" :background "#E8FCE8")) (t (:foreground "#000000" :background "#E8FCE8")))) (diff-context ((t (:foreground "#000000")))) (diff-removed ((((class color) (min-colors 89)) (:foreground "#000000" :background "#F8E8E8")) (t (:foreground "#000000" :background "#F8E8E8")))) (diff-header ((((class color) (min-colors 89)) (:foreground "#000000" :weight normal)) (t (:foreground "#FFEAFF" :weight normal)))) (diff-file-header ((((class color) (min-colors 89)) (:foreground "#000000" :background "#A8EFEB" :weight normal)) (t (:foreground "#000000" :background "#A8EFEB" :weight normal)))) (diredp-file-name ((t (:foreground "#000000")))) (diredp-no-priv ((t (:foreground "#000000")))) (diredp-other-priv ((t (:foreground "#000000")))) (diredp-rare-priv ((t (:foreground "#000000")))) (diredp-read-priv ((t (:foreground "#000000")))) (diredp-symlink ((t (:foreground "#000000" :background "#E1FAFF")))) (diredp-write-priv ((t (:foreground "#000000")))) (elfeed-search-title-face ((t (:foreground "#000000")))) (elfeed-search-unread-title-face ((t (:foreground "#000000")))) (elfeed-search-unread-count-face ((t (:foreground "#000000")))) (erc-default-face ((t (:foreground "#000000")))) (erc-input-face ((t (:foreground "#000000" :background "#EFEFD8")))) (erc-current-nick-face ((t (:foreground "#000000" :background "#A8EFEB" :weight normal :box (:line-width 1 :style released-button))))) (erc-my-nick-face ((t (:foreground "#000000" :background "#A8EFEB" :weight normal :box (:line-width 1 :style released-button))))) (company-preview-common ((t (:foreground "#000000" :background "#A8EFEB")))) (highlight-operators-face ((t (:foreground "#000000")))) (ido-first-match ((t (:foreground "#000000" :weight normal)))) (ido-only-match ((t (:foreground "#000000" :weight normal)))) (ido-vertical-first-match-face ((t (:foreground "#000000" :background "#A8EFEB" :weight normal)))) (ido-vertical-match-face ((t (:foreground "#000000" :background "#E8FCE8" :weight normal :underline t)))) (js2-function-param ((t (:foreground "#000000")))) (magit-filename ((t (:foreground "#000000")))) (magit-diff-file-heading ((t (:foreground "#000000" :weight normal)))) (pe/file-face ((t (:foreground "#000000")))) (show-paren-match ((t (:foreground "#000000" :background "#A8EFEB" :weight normal)))) (mode-line ((((class color) (min-colors 89)) (:foreground "#000000" :background "#E1FAFF" :box t)))) (mode-line-inactive ((t (:foreground "#000000" :background "#E5E5D0" :box t)))) (mode-line-buffer-id ((t (:foreground "#000000" :weight bold)))) (sml/global ((t (:foreground "#000000")))) (sml/folder ((t (:foreground "#000000")))) (sml/prefix ((t (:foreground "#000000")))) (sml/read-only ((t (:foreground "#000000")))) (sml/line-number ((t (:foreground "#000000" :weight normal)))) (sml/col-number ((t (:foreground "#000000" :weight normal)))) (sml/vc ((t (:foreground "#000000" :weight normal)))) (sml/git ((t (:foreground "#000000" :weight normal)))) (web-mode-function-name-face ((t (:foreground "#000000" :weight normal)))) (web-mode-html-attr-name-face ((t (:foreground "#000000")))) (web-mode-current-element-highlight-face ((t (:foreground "#000000" :background "#E1FAFF")))) (org-verbatim ((t (:foreground "#000000" :background "#EFEFD8" :underline t)))) (org-block ((t (:foreground "#000000" :background "#EFEFD8" :extend t)))) (mu4e-header-value-face ((t (:foreground "#000000")))) (term ((((class color) (min-colors 89)) (:foreground "#000000" :background "#FFFFE8")))) (term-color-black ((((class color) (min-colors 89)) (:foreground "#000000" :background "#000000")))) (term-color-white ((((class color) (min-colors 89)) (:foreground "#000000" :background "#000000"))))) nil)"##
    ]];
    assert_acme_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn acme_theme_duplicate_ivy_face_specs_preserve_both_and_first_source_definition_wins() {
    let elisp_form = r##"(let ((definitions
                (mapcar
                 (lambda (setting)
                   (copy-tree
                    (nth
                     3
                     setting)))
                 (cl-remove-if-not
                  (lambda (setting)
                    (eq
                     (cadr setting)
                     'ivy-minibuffer-match-face-3))
                  (reverse
                   (copy-sequence
                    (get
                     'acme
                     'theme-settings))))))
               applied)
         (unwind-protect
             (progn
               (load-theme
                'acme
                t)
               (eval
                '(defface
                     ivy-minibuffer-match-face-3
                   '((t
                      (:background
                       "fallback")))
                   "Parity fixture."))
               (setq applied
                     (face-attribute
                      'ivy-minibuffer-match-face-3
                      :background
                      nil
                      t)))
           (disable-theme
            'acme))
         (list
          definitions
          applied))"##;
    let expect = expect![[
        r##"OK ((((t (:background "#FFEAFF"))) ((t (:background "#E1FAFF")))) "#FFEAFF")"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}
