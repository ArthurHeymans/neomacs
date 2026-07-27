use expect_test::expect;

use super::assert_ancient_theme_parity;

#[test]
fn ancient_theme_real_elisp_fontification_resolves_declared_colors() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ancient)
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              ";; weathered comment\n"
              "(defconst ancient-count 42)\n"
              "(defun ancient-build (name)\n"
              "  (if name (message \"ruin: %s\" name) nil))\n")
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char (point-min))
                (search-forward needle)
                (let* ((position
                        (match-beginning 0))
                       (face
                        (get-text-property
                         position 'face)))
                  (list
                   needle
                   face
                   (and face
                        (face-attribute
                         face :foreground nil t))
                   (and face
                        (face-attribute
                         face :background nil t))
                   (and face
                        (face-attribute
                         face :slant nil t)))))
              '("weathered comment" "defconst"
                "ancient-count" "42" "defun"
                "ancient-build" "if" "message"
                "\"ruin: %s\"" "nil"))))
       (disable-theme 'ancient))"##;
    let expect = expect![[
        r##"OK (("weathered comment" font-lock-comment-face "#665a48" unspecified italic) ("defconst" font-lock-keyword-face "#3d8a6e" unspecified unspecified) ("ancient-count" font-lock-variable-name-face "#e8dcc8" unspecified unspecified) ("42" nil nil nil nil) ("defun" font-lock-keyword-face "#3d8a6e" unspecified unspecified) ("ancient-build" font-lock-function-name-face "#f0e8d4" unspecified unspecified) ("if" font-lock-keyword-face "#3d8a6e" unspecified unspecified) ("message" nil nil nil nil) ("\"ruin: %s\"" font-lock-string-face "#c8a05a" unspecified unspecified) ("nil" nil nil nil nil))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_real_org_fontification_resolves_document_structure() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ancient)
           (with-temp-buffer
             (org-mode)
             (insert
              "#+title: Excavation Log\n"
              "* TODO Survey the ruins :field:\n"
              "SCHEDULED: <2026-03-22 Sun>\n"
              "Read [[https://example.invalid][the map]] and =glyph=.\n"
              "#+begin_src emacs-lisp\n"
              "(message \"found\")\n"
              "#+end_src\n")
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char (point-min))
                (search-forward needle)
                (let ((face
                       (get-text-property
                        (match-beginning 0)
                        'face)))
                  (list
                   needle
                   face
                   (if (listp face)
                       (mapcar
                        (lambda (item)
                          (and
                           (symbolp item)
                           (face-attribute
                            item :foreground nil t)))
                        face)
                     (and face
                          (face-attribute
                           face :foreground nil t))))))
              '("Excavation Log" "TODO" "Survey"
                ":field:" "SCHEDULED:" "2026-03-22"
                "the map" "=glyph=" "#+begin_src"
                "(message" "#+end_src"))))
       (disable-theme 'ancient))"##;
    let expect = expect![[
        r##"OK (("Excavation Log" org-document-title "#e8cc90") ("TODO" (org-todo org-level-1) ("#a84428" "#e8cc90")) ("Survey" org-level-1 "#e8cc90") (":field:" (org-tag org-level-1) ("#665a48" "#e8cc90")) ("SCHEDULED:" org-special-keyword "#665a48") ("2026-03-22" (org-date) ("#c8a05a")) ("the map" org-link "#7ecfb4") ("=glyph=" (org-verbatim) ("#e8cc90")) ("#+begin_src" org-block-begin-line "#665a48") ("(message" (org-block) ("#c8b89a")) ("#+end_src" org-block-end-line "#665a48"))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_real_diff_fontification_distinguishes_change_kinds() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ancient)
           (with-temp-buffer
             (diff-mode)
             (insert
              "diff --git a/map.txt b/map.txt\n"
              "index 1111111..2222222 100644\n"
              "--- a/map.txt\n"
              "+++ b/map.txt\n"
              "@@ -1,2 +1,2 @@\n"
              "-lost chamber\n"
              "+found chamber\n"
              " stable wall\n")
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char (point-min))
                (search-forward needle)
                (let ((face
                       (get-text-property
                        (match-beginning 0)
                        'face)))
                  (list
                   needle
                   face
                   (and face
                        (face-attribute
                         face :foreground nil t))
                   (and face
                        (face-attribute
                         face :background nil t)))))
              '("diff --git" "--- a/map.txt"
                "+++ b/map.txt" "@@ -1,2"
                "-lost chamber" "+found chamber"
                " stable wall"))))
       (disable-theme 'ancient))"##;
    let expect = expect![[
        r##"OK (("diff --git" diff-header "#8a7a64" "#2d2820") ("--- a/map.txt" diff-header "#8a7a64" "#2d2820") ("+++ b/map.txt" diff-header "#8a7a64" "#2d2820") ("@@ -1,2" diff-hunk-header "#7aacc0" "#2d2820") ("-lost chamber" diff-indicator-removed "#e08c68" "#4c1c10") ("+found chamber" diff-indicator-added "#7ecfb4" "#122a20") (" stable wall" diff-context "#665a48" unspecified))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_major_ecosystem_faces_resolve_practical_status_palette() {
    let elisp_form = r##"(let ((faces
                        '(success warning error
                          org-todo org-done org-scheduled
                          magit-diff-added magit-diff-removed
                          magit-branch-local magit-branch-remote
                          dired-directory dired-broken-symlink
                          flycheck-error flycheck-warning
                          flymake-error flymake-note
                          which-key-key-face
                          treemacs-git-untracked-face
                          orderless-match-face-0
                          rainbow-delimiters-unmatched-face)))
         (mapc
          (lambda (face)
            (unless (facep face)
              (make-face face)))
          faces)
         (unwind-protect
         (progn
           (enable-theme 'ancient)
           (mapcar
            (lambda (face)
              (list
               face
               (face-attribute
                face :foreground nil t)
               (face-attribute
                face :background nil t)
               (face-attribute
                face :weight nil t)
               (face-attribute
                face :slant nil t)
               (face-attribute
                face :underline nil t)))
            '(success warning error
              org-todo org-done org-scheduled
              magit-diff-added magit-diff-removed
              magit-branch-local magit-branch-remote
              dired-directory dired-broken-symlink
              flycheck-error flycheck-warning
              flymake-error flymake-note
              which-key-key-face
              treemacs-git-untracked-face
              orderless-match-face-0
              rainbow-delimiters-unmatched-face)))
           (disable-theme 'ancient)))"##;
    let expect = expect![[
        r##"OK ((success "#7ecfb4" unspecified unspecified unspecified unspecified) (warning "#c8a05a" unspecified unspecified unspecified unspecified) (error "#e08c68" unspecified unspecified unspecified unspecified) (org-todo "#a84428" unspecified normal unspecified unspecified) (org-done "#665a48" unspecified unspecified unspecified unspecified) (org-scheduled "#3d8a6e" unspecified unspecified unspecified unspecified) (magit-diff-added "#7ecfb4" "#122a20" unspecified unspecified unspecified) (magit-diff-removed "#e08c68" "#4c1c10" unspecified unspecified unspecified) (magit-branch-local "#3d8a6e" unspecified unspecified unspecified unspecified) (magit-branch-remote "#4a7a94" unspecified unspecified unspecified unspecified) (dired-directory "#3d8a6e" unspecified unspecified unspecified unspecified) (dired-broken-symlink "#e08c68" unspecified unspecified unspecified unspecified) (flycheck-error unspecified unspecified unspecified unspecified (:style wave :color "#e08c68")) (flycheck-warning unspecified unspecified unspecified unspecified (:style wave :color "#c8a05a")) (flymake-error unspecified unspecified unspecified unspecified (:style wave :color "#e08c68")) (flymake-note unspecified unspecified unspecified unspecified (:style wave :color "#7aacc0")) (which-key-key-face "#7ecfb4" unspecified unspecified unspecified unspecified) (treemacs-git-untracked-face "#7aacc0" unspecified unspecified unspecified unspecified) (orderless-match-face-0 "#7ecfb4" unspecified unspecified unspecified unspecified) (rainbow-delimiters-unmatched-face "#e08c68" "#4c1c10" unspecified unspecified unspecified))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_core_palette_has_exact_color_values_and_accessibility_pairs() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ancient)
           (mapcar
            (lambda (face)
              (let ((foreground
                     (face-attribute
                      face :foreground nil t))
                    (background
                     (face-attribute
                      face :background nil t)))
                (list
                 face
                 foreground background
                 (and
                  (stringp foreground)
                  (color-values foreground))
                 (and
                  (stringp background)
                  (color-values background)))))
            '(default cursor region highlight
              mode-line mode-line-inactive
              isearch isearch-fail
              lazy-highlight query-replace
              header-line tab-bar-tab)))
       (disable-theme 'ancient))"##;
    let expect = expect![[
        r##"OK ((default "#e8dcc8" "#1a1710" #2=(65535 65535 65535) #1=(65535 0 0)) (cursor unspecified "#3d8a6e" nil #3=(0 65535 0)) (region unspecified "#4a4234" nil #1#) (highlight unspecified "#2d2820" nil #1#) (mode-line "#8a7a64" "#2d2820" #1# #1#) (mode-line-inactive "#665a48" "#1a1710" #1# #1#) (isearch "#f0e8d4" "#2d6652" #2# #3#) (isearch-fail "#e08c68" "#4c1c10" #4=(65535 65535 0) #1#) (lazy-highlight "#8a7a64" "#4a4234" #1# #1#) (query-replace "#e8cc90" "#5a4422" #4# #1#) (header-line "#8a7a64" "#0e0c09" #1# #1#) (tab-bar-tab "#e8dcc8" "#2d2820" #2# #1#))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}
