use expect_test::expect;

use super::assert_ancient_theme_parity;

#[test]
fn ancient_theme_settings_capture_every_face_exactly() {
    let elisp_form = r##"(let* ((settings
                          (get 'ancient
                               'theme-settings))
               (faces
                (mapcar #'cadr settings)))
         (list
          (length settings)
          (cl-count 'theme-face settings
                    :key #'car)
          (= (length faces)
             (length
              (delete-dups
               (copy-sequence faces))))
          (secure-hash
           'sha256
           (prin1-to-string settings))
          (secure-hash
           'sha256
           (prin1-to-string
            (sort
             (copy-sequence faces)
             (lambda (left right)
               (string<
                (symbol-name left)
                (symbol-name right))))))))"##;
    let expect = expect![[
        r#"OK (236 236 t "12efc48e191a89413d085774abaceccf2b0d2e726bb9649c276569cebd200617" "323b71ce354aacb72ee1e5da6f9130e36e0b75d371e5b1b1d4222454a7ffc5cb")"#
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_core_ui_and_search_specs_are_exact() {
    let elisp_form = r##"(let ((table
                        (mapcar
                         (lambda (setting)
                           (cons
                            (cadr setting)
                            (cddr setting)))
                         (get 'ancient
                              'theme-settings))))
         (mapcar
          (lambda (face)
            (assq face table))
          '(default cursor region highlight
            fringe vertical-border
            minibuffer-prompt link error
            mode-line mode-line-inactive
            isearch isearch-fail
            lazy-highlight match query-replace)))"##;
    let expect = expect![[
        r##"OK ((default ancient ((t (:background "#1a1710" :foreground "#e8dcc8")))) (cursor ancient ((t (:background "#3d8a6e")))) (region ancient ((t (:background "#4a4234")))) (highlight ancient ((t (:background "#2d2820")))) (fringe ancient ((t (:background "#1a1710" :foreground "#665a48")))) (vertical-border ancient ((t (:foreground "#4a4234")))) (minibuffer-prompt ancient ((t (:foreground "#3d8a6e" :weight normal)))) (link ancient ((t (:foreground "#7ecfb4" :underline t)))) (error ancient ((t (:foreground "#e08c68")))) (mode-line ancient ((t (:background "#2d2820" :foreground "#8a7a64" :box (:line-width 1 :color "#4a4234"))))) (mode-line-inactive ancient ((t (:background "#1a1710" :foreground "#665a48" :box (:line-width 1 :color "#2d2820"))))) (isearch ancient ((t (:background "#2d6652" :foreground "#f0e8d4")))) (isearch-fail ancient ((t (:background "#4c1c10" :foreground "#e08c68")))) (lazy-highlight ancient ((t (:background "#4a4234" :foreground "#8a7a64")))) (match ancient ((t (:background "#2d6652" :foreground "#f0e8d4")))) (query-replace ancient ((t (:background "#5a4422" :foreground "#e8cc90")))))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_font_lock_specs_cover_all_modern_emacs_categories() {
    let elisp_form = r##"(let ((settings
                        (get 'ancient
                             'theme-settings)))
         (mapcar
          (lambda (face)
            (assq
             face
             (mapcar
              (lambda (setting)
                (cons
                 (cadr setting)
                 (cddr setting)))
              settings)))
          '(font-lock-comment-face
            font-lock-doc-face
            font-lock-string-face
            font-lock-keyword-face
            font-lock-builtin-face
            font-lock-function-name-face
            font-lock-function-call-face
            font-lock-variable-name-face
            font-lock-variable-use-face
            font-lock-type-face
            font-lock-constant-face
            font-lock-preprocessor-face
            font-lock-warning-face
            font-lock-number-face
            font-lock-operator-face
            font-lock-property-name-face
            font-lock-property-use-face
            font-lock-delimiter-face
            font-lock-bracket-face
            font-lock-escape-face
            font-lock-misc-punctuation-face)))"##;
    let expect = expect![[
        r##"OK ((font-lock-comment-face ancient ((t (:foreground "#665a48" :slant italic)))) (font-lock-doc-face ancient ((t (:foreground "#5a4422" :slant italic)))) (font-lock-string-face ancient ((t (:foreground "#c8a05a")))) (font-lock-keyword-face ancient ((t (:foreground "#3d8a6e")))) (font-lock-builtin-face ancient ((t (:foreground "#7ecfb4")))) (font-lock-function-name-face ancient ((t (:foreground "#f0e8d4" :weight normal)))) (font-lock-function-call-face ancient ((t (:foreground "#e8dcc8")))) (font-lock-variable-name-face ancient ((t (:foreground "#e8dcc8")))) (font-lock-variable-use-face ancient ((t (:foreground "#c8b89a")))) (font-lock-type-face ancient ((t (:foreground "#e8cc90")))) (font-lock-constant-face ancient ((t (:foreground "#e08c68")))) (font-lock-preprocessor-face ancient ((t (:foreground "#a84428")))) (font-lock-warning-face ancient ((t (:foreground "#a84428" :weight normal)))) (font-lock-number-face ancient ((t (:foreground "#7aacc0")))) (font-lock-operator-face ancient ((t (:foreground "#8a7a64")))) (font-lock-property-name-face ancient ((t (:foreground "#e8dcc8")))) (font-lock-property-use-face ancient ((t (:foreground "#c8b89a")))) (font-lock-delimiter-face ancient ((t (:foreground "#8a7a64")))) (font-lock-bracket-face ancient ((t (:foreground "#8a7a64")))) (font-lock-escape-face ancient ((t (:foreground "#e08c68")))) (font-lock-misc-punctuation-face ancient ((t (:foreground "#8a7a64")))))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_ecosystem_face_groups_are_complete_and_exact() {
    let elisp_form = r##"(let* ((settings
                          (get 'ancient
                               'theme-settings))
               (groups
                '((completion
                   company-tooltip company-tooltip-selection
                   corfu-default corfu-current vertico-current)
                  (org
                   org-level-1 org-level-8 org-todo org-done
                   org-code org-block org-document-title
                   org-scheduled org-warning org-table)
                  (magit
                   magit-section-heading magit-diff-added
                   magit-diff-removed magit-branch-current
                   magit-signature-good magit-blame-heading)
                  (files
                   dired-directory dired-symlink
                   dired-broken-symlink dired-flagged
                   treemacs-root-face treemacs-git-untracked-face)
                  (diagnostics
                   flycheck-error flycheck-warning flycheck-info
                   flymake-error flymake-warning flymake-note
                   eglot-diagnostic-tag-deprecated-face)
                  (navigation
                   which-key-key-face which-key-group-description-face
                   consult-preview-cursor consult-highlight-match
                   orderless-match-face-0 orderless-match-face-3
                   rainbow-delimiters-depth-1-face
                   rainbow-delimiters-unmatched-face))))
         (mapcar
          (lambda (group)
            (list
             (car group)
             (mapcar
              (lambda (face)
                (seq-find
                 (lambda (setting)
                   (eq (cadr setting)
                       face))
                 settings))
              (cdr group))))
          groups))"##;
    let expect = expect![[
        r##"OK ((completion ((theme-face company-tooltip ancient ((t (:background "#2d2820" :foreground "#e8dcc8")))) (theme-face company-tooltip-selection ancient ((t (:background "#4a4234" :foreground "#f0e8d4")))) (theme-face corfu-default ancient ((t (:background "#2d2820" :foreground "#e8dcc8")))) (theme-face corfu-current ancient ((t (:background "#4a4234" :foreground "#f0e8d4")))) (theme-face vertico-current ancient ((t (:background "#4a4234" :foreground "#f0e8d4")))))) (org ((theme-face org-level-1 ancient ((t (:foreground "#e8cc90" :weight normal :height 1.15)))) (theme-face org-level-8 ancient ((t (:foreground "#c8b89a")))) (theme-face org-todo ancient ((t (:foreground "#a84428" :weight normal)))) (theme-face org-done ancient ((t (:foreground "#665a48")))) (theme-face org-code ancient ((t (:foreground "#7ecfb4" :background "#2d2820")))) (theme-face org-block ancient ((t (:background "#0e0c09" :foreground "#c8b89a")))) (theme-face org-document-title ancient ((t (:foreground "#e8cc90" :weight normal :height 1.3)))) (theme-face org-scheduled ancient ((t (:foreground "#3d8a6e")))) (theme-face org-warning ancient ((t (:foreground "#e08c68")))) (theme-face org-table ancient ((t (:foreground "#c8b89a")))))) (magit ((theme-face magit-section-heading ancient ((t (:foreground "#c8a05a" :weight normal)))) (theme-face magit-diff-added ancient ((t (:background "#122a20" :foreground "#7ecfb4")))) (theme-face magit-diff-removed ancient ((t (:background "#4c1c10" :foreground "#e08c68")))) (theme-face magit-branch-current ancient ((t (:foreground "#7ecfb4" :box (:line-width -1 :color "#3d8a6e"))))) (theme-face magit-signature-good ancient ((t (:foreground "#7ecfb4")))) (theme-face magit-blame-heading ancient ((t (:background "#2d2820" :foreground "#665a48")))))) (files ((theme-face dired-directory ancient ((t (:foreground "#3d8a6e")))) (theme-face dired-symlink ancient ((t (:foreground "#7aacc0")))) (theme-face dired-broken-symlink ancient ((t (:foreground "#e08c68" :strike-through t)))) (theme-face dired-flagged ancient ((t (:foreground "#e08c68" :strike-through t)))) (theme-face treemacs-root-face ancient ((t (:foreground "#e8cc90" :weight normal)))) (theme-face treemacs-git-untracked-face ancient ((t (:foreground "#7aacc0")))))) (diagnostics ((theme-face flycheck-error ancient ((t (:underline (:style wave :color "#e08c68"))))) (theme-face flycheck-warning ancient ((t (:underline (:style wave :color "#c8a05a"))))) (theme-face flycheck-info ancient ((t (:underline (:style wave :color "#7aacc0"))))) (theme-face flymake-error ancient ((t (:underline (:style wave :color "#e08c68"))))) (theme-face flymake-warning ancient ((t (:underline (:style wave :color "#c8a05a"))))) (theme-face flymake-note ancient ((t (:underline (:style wave :color "#7aacc0"))))) (theme-face eglot-diagnostic-tag-deprecated-face ancient ((t (:foreground "#665a48" :strike-through t)))))) (navigation ((theme-face which-key-key-face ancient ((t (:foreground "#7ecfb4")))) (theme-face which-key-group-description-face ancient ((t (:foreground "#c8a05a")))) (theme-face consult-preview-cursor ancient ((t (:background "#2d6652")))) (theme-face consult-highlight-match ancient ((t (:background "#2d6652" :foreground "#f0e8d4")))) (theme-face orderless-match-face-0 ancient ((t (:foreground "#7ecfb4")))) (theme-face orderless-match-face-3 ancient ((t (:foreground "#c09080")))) (theme-face rainbow-delimiters-depth-1-face ancient ((t (:foreground "#8a7a64")))) (theme-face rainbow-delimiters-unmatched-face ancient ((t (:foreground "#e08c68" :background "#4c1c10")))))))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_palette_has_exact_reuse_and_contrast_relationships() {
    let elisp_form = r##"(let ((spec
                        (lambda (face)
                          (cadddr
                           (seq-find
                            (lambda (setting)
                              (eq (cadr setting)
                                  face))
                            (get 'ancient
                                 'theme-settings))))))
         (list
          (mapcar
           (lambda (face)
             (list face
                   (funcall spec face)))
           '(diff-added diff-removed diff-changed
             diff-refine-added diff-refine-removed
             diff-refine-changed))
          (equal
           (funcall spec 'diff-added)
           (funcall spec 'magit-diff-added))
          (equal
           (funcall spec 'diff-refine-added)
           (funcall spec 'magit-diff-added-highlight))
          (equal
           (funcall spec 'diff-removed)
           (funcall spec 'magit-diff-removed))
          (equal
           (funcall spec 'diff-refine-removed)
           (funcall spec 'magit-diff-removed-highlight))
          (equal
           (funcall spec 'isearch)
           (funcall spec 'match))
          (equal
           (funcall spec 'hl-line)
           (funcall spec 'consult-preview-line))))"##;
    let expect = expect![[
        r##"OK (((diff-added ((t (:background "#122a20" :foreground "#7ecfb4")))) (diff-removed ((t (:background "#4c1c10" :foreground "#e08c68")))) (diff-changed ((t (:background "#2a2410" :foreground "#e8cc90")))) (diff-refine-added ((t (:background "#1e4434" :foreground "#7ecfb4")))) (diff-refine-removed ((t (:background "#7a2e18" :foreground "#e08c68")))) (diff-refine-changed ((t (:background "#4a3c10" :foreground "#e8cc90"))))) t t t t t t)"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}
