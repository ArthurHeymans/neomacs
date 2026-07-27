use expect_test::expect;

use super::assert_almost_mono_themes_parity;

#[test]
fn white_theme_styles_real_emacs_lisp_font_lock_tokens() {
    let elisp_form = r##"(let ((theme 'almost-mono-white))
  (unwind-protect
      (progn
        (load-theme theme t)
        (with-temp-buffer
          (emacs-lisp-mode)
          (insert
           ";;; Render a report\n(defun render-report (count &optional label)\n  \"Build LABEL for COUNT.\"\n  (let ((message \"ready\"))\n    ;; Highlight a practical result.\n    (list count label message)))\n")
          (font-lock-ensure)
          (mapcar
           (lambda (token)
             (goto-char (point-min))
             (search-forward token)
             (let* ((position (- (point) (length token)))
                    (face (get-text-property position 'face)))
               (list
                token face
                (and face
                     (face-attribute
                      face :foreground nil 'default))
                (and face
                     (face-attribute
                      face :weight nil 'default))
                (and face
                     (face-attribute
                      face :slant nil 'default)))))
           '("Render a report" "defun" "render-report"
             "count" "Build LABEL" "\"ready\""
             "Highlight a practical result"))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (("Render a report" font-lock-comment-face "#888888" normal italic) ("defun" font-lock-keyword-face "#000000" bold normal) ("render-report" font-lock-function-name-face "#000000" bold normal) ("count" nil nil nil nil) ("Build LABEL" font-lock-doc-face "#888888" normal italic) ("\"ready\"" font-lock-string-face "#3c5e2b" normal normal) ("Highlight a practical result" font-lock-comment-face "#888888" normal italic))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn black_theme_styles_same_real_buffer_with_dark_palette_and_semantic_faces() {
    let elisp_form = r##"(let ((theme 'almost-mono-black))
  (unwind-protect
      (progn
        (load-theme theme t)
        (with-temp-buffer
          (emacs-lisp-mode)
          (insert
           "(defconst report-limit 42)\n(defun build-report (name)\n  \"Build a report.\"\n  ;; Keep this explanation readable.\n  (message \"ready: %s\" name))\n")
          (font-lock-ensure)
          (list
           (face-attribute 'default :background nil 'default)
           (face-attribute 'default :foreground nil 'default)
           (mapcar
            (lambda (token)
              (goto-char (point-min))
              (search-forward token)
              (let* ((position (- (point) (length token)))
                     (face (get-text-property position 'face)))
                (list
                 token face
                 (and face
                      (face-attribute
                       face :foreground nil 'default))
                 (and face
                      (face-attribute
                       face :weight nil 'default))
                 (and face
                      (face-attribute
                       face :slant nil 'default)))))
            '("defconst" "report-limit" "42"
              "defun" "build-report" "Build a report"
              "Keep this explanation" "\"ready: %s\"")))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK ("#000000" "#ffffff" (("defconst" font-lock-keyword-face "#ffffff" bold normal) ("report-limit" font-lock-variable-name-face "#ffffff" normal normal) ("42" nil nil nil nil) ("defun" font-lock-keyword-face "#ffffff" bold normal) ("build-report" font-lock-function-name-face "#ffffff" bold normal) ("Build a report" font-lock-doc-face "#aaaaaa" normal italic) ("Keep this explanation" font-lock-comment-face "#aaaaaa" normal italic) ("\"ready: %s\"" font-lock-string-face "#a7bca4" normal normal)))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn cream_theme_styles_real_org_title_tasks_properties_and_table() {
    let elisp_form = r##"(let ((theme 'almost-mono-cream))
  (unwind-protect
      (progn
        (require 'org)
        (load-theme theme t)
        (with-temp-buffer
          (org-mode)
          (insert
           "#+title: Release dashboard\n* TODO Ship release\n:PROPERTIES:\n:Owner: Ada\n:END:\n* DONE Archive notes\n| Item | State |\n| API  | Ready |\n")
          (font-lock-ensure)
          (mapcar
           (lambda (token)
             (goto-char (point-min))
             (search-forward token)
             (let* ((position (- (point) (length token)))
                    (face (get-text-property position 'face))
                    (primary
                     (if (listp face) (car face) face)))
               (list
                token face
                (and primary
                     (face-attribute
                      primary :foreground nil 'default))
                (and primary
                     (face-attribute
                      primary :weight nil 'default))
                (and primary
                     (face-attribute
                      primary :slant nil 'default)))))
           '("#+title:" "Release dashboard" "TODO"
             ":PROPERTIES:" ":Owner:" "Ada" "DONE"
             "Archive notes" "| Item | State |"))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (("#+title:" org-document-info-keyword "#000000" normal normal) ("Release dashboard" org-document-title "#000000" normal normal) ("TODO" (org-todo org-level-1) "#fda50f" bold normal) (":PROPERTIES:" org-drawer "#7d7165" normal normal) (":Owner:" org-special-keyword "#7d7165" bold normal) ("Ada" org-property-value "#7d7165" normal italic) ("DONE" (org-done org-level-1) "#00ff00" bold normal) ("Archive notes" (org-headline-done org-level-1) "#000000" bold normal) ("| Item | State |" org-table "#7d7165" normal normal))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn theme_applies_expected_visual_contracts_to_available_ecosystem_faces() {
    let elisp_form = r##"(let ((theme 'almost-mono-gray)
      (faces
       '(company-tooltip company-tooltip-selection
         company-tooltip-annotation
         git-gutter:modified git-gutter:added
         git-gutter:deleted
         diff-hl-change diff-hl-insert diff-hl-delete
         vertico-current completions-common-part
         orderless-match-face-0
         orderless-match-face-2)))
  (unwind-protect
      (progn
        (mapc
         (lambda (face)
           (unless (facep face)
             (make-face face)))
         faces)
        (load-theme theme t)
        (mapcar
         (lambda (face)
           (list
            face
            (facep face)
            (face-attribute
             face :background nil 'default)
            (face-attribute
             face :foreground nil 'default)
            (face-attribute
             face :weight nil 'default)
            (face-attribute
             face :slant nil 'default)
            (face-attribute
             face :underline nil 'default)
            (face-attribute
             face :inherit nil 'default)))
         faces))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK ((company-tooltip [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#222222" "#ffffff" normal normal nil nil) (company-tooltip-selection [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#666666" "#ffffff" normal normal nil nil) (company-tooltip-annotation [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#222222" "#aaaaaa" normal italic nil nil) (git-gutter:modified [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#fda50f" "#fda50f" normal normal nil nil) (git-gutter:added [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#00ff00" "#00ff00" normal normal nil nil) (git-gutter:deleted [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#ff0000" "#ff0000" normal normal nil nil) (diff-hl-change [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#fda50f" "#fda50f" normal normal nil nil) (diff-hl-insert [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#00ff00" "#00ff00" normal normal nil nil) (diff-hl-delete [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#ff0000" "#ff0000" normal normal nil nil) (vertico-current [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#fda50f" "#ffffff" bold normal nil nil) (completions-common-part [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#2b2b2b" "#ffffff" bold normal t nil) (orderless-match-face-0 [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#2b2b2b" "#ffffff" bold normal t nil) (orderless-match-face-2 [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#2b2b2b" "#ffffff" bold normal t nil))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn inherited_documentation_line_and_shell_faces_resolve_through_parent_faces() {
    let elisp_form = r##"(let ((theme 'almost-mono-black))
  (unwind-protect
      (progn
        (mapc
         (lambda (face)
           (unless (facep face)
             (make-face face)))
         '(linum highlight-current-line-face
           eshell-ls-unreadable eshell-ls-archive
           eshell-ls-symlink))
        (load-theme theme t)
        (mapcar
         (lambda (face)
           (list
            face
            (face-attribute
             face :inherit nil 'default)
            (face-attribute
             face :background nil 'default)
            (face-attribute
             face :foreground nil 'default)
            (face-attribute
             face :weight nil 'default)
            (face-attribute
             face :slant nil 'default)))
         '(font-lock-doc-face
           linum
           highlight-current-line-face
           eshell-ls-archive
           eshell-ls-symlink)))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK ((font-lock-doc-face font-lock-comment-face "#000000" "#aaaaaa" normal italic) (linum line-number "#000000" "#666666" normal normal) (highlight-current-line-face hl-line "#000000" "#ffffff" normal normal) (eshell-ls-archive eshell-ls-unreadable "#000000" "#ffffff" normal normal) (eshell-ls-symlink eshell-ls-unreadable "#000000" "#ffffff" normal normal))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn selection_search_parenthesis_and_diagnostic_faces_keep_distinct_status_colors() {
    let elisp_form = r##"(let ((themes
       '(almost-mono-white almost-mono-black
         almost-mono-gray almost-mono-cream))
      (faces
       '(region isearch lazy-highlight
         show-paren-match show-paren-mismatch
         font-lock-warning-face
         org-todo org-done
         git-gutter:modified git-gutter:added
         git-gutter:deleted)))
  (mapcar
   (lambda (theme)
     (unwind-protect
         (progn
           (require 'org)
           (mapc
            (lambda (face)
              (unless (facep face)
                (make-face face)))
            '(git-gutter:modified git-gutter:added
              git-gutter:deleted))
           (load-theme theme t)
           (list
            theme
            (mapcar
             (lambda (face)
               (list
                face
                (face-attribute
                 face :background nil 'default)
                (face-attribute
                 face :foreground nil 'default)
                (face-attribute
                 face :weight nil 'default)
                (copy-tree
                 (face-attribute
                  face :underline nil 'default))))
             faces)))
       (when (memq theme custom-enabled-themes)
         (disable-theme theme))))
   themes))"##;
    let expect = expect![[
        r##"OK ((almost-mono-white ((region "#fda50f" "#000000" normal nil) (isearch "#888888" "#000000" bold nil) (lazy-highlight "#dddddd" "#000000" normal nil) (show-paren-match "#ffffff" "#00ff00" bold nil) (show-paren-mismatch "#ffffff" "#ff0000" bold nil) (font-lock-warning-face "#ffffff" "#000000" normal (:color "#ff0000" :style wave)) (org-todo "#ffffff" "#fda50f" bold nil) (org-done "#ffffff" "#00ff00" bold nil) (git-gutter:modified "#fda50f" "#fda50f" normal nil) (git-gutter:added "#00ff00" "#00ff00" normal nil) (git-gutter:deleted "#ff0000" "#ff0000" normal nil))) (almost-mono-black ((region "#fda50f" "#ffffff" normal nil) (isearch "#aaaaaa" "#ffffff" bold nil) (lazy-highlight "#666666" "#ffffff" normal nil) (show-paren-match "#000000" "#00ff00" bold nil) (show-paren-mismatch "#000000" "#ff0000" bold nil) (font-lock-warning-face "#000000" "#ffffff" normal (:color "#ff0000" :style wave)) (org-todo "#000000" "#fda50f" bold nil) (org-done "#000000" "#00ff00" bold nil) (git-gutter:modified "#fda50f" "#fda50f" normal nil) (git-gutter:added "#00ff00" "#00ff00" normal nil) (git-gutter:deleted "#ff0000" "#ff0000" normal nil))) (almost-mono-gray ((region "#fda50f" "#ffffff" normal nil) (isearch "#aaaaaa" "#ffffff" bold nil) (lazy-highlight "#666666" "#ffffff" normal nil) (show-paren-match "#2b2b2b" "#00ff00" bold nil) (show-paren-mismatch "#2b2b2b" "#ff0000" bold nil) (font-lock-warning-face "#2b2b2b" "#ffffff" normal (:color "#ff0000" :style wave)) (org-todo "#2b2b2b" "#fda50f" bold nil) (org-done "#2b2b2b" "#00ff00" bold nil) (git-gutter:modified "#fda50f" "#fda50f" normal nil) (git-gutter:added "#00ff00" "#00ff00" normal nil) (git-gutter:deleted "#ff0000" "#ff0000" normal nil))) (almost-mono-cream ((region "#fda50f" "#000000" normal nil) (isearch "#7d7165" "#000000" bold nil) (lazy-highlight "#c4baaf" "#000000" normal nil) (show-paren-match "#f0e5da" "#00ff00" bold nil) (show-paren-mismatch "#f0e5da" "#ff0000" bold nil) (font-lock-warning-face "#f0e5da" "#000000" normal (:color "#ff0000" :style wave)) (org-todo "#f0e5da" "#fda50f" bold nil) (org-done "#f0e5da" "#00ff00" bold nil) (git-gutter:modified "#fda50f" "#fda50f" normal nil) (git-gutter:added "#00ff00" "#00ff00" normal nil) (git-gutter:deleted "#ff0000" "#ff0000" normal nil))))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn mode_line_boxes_and_borders_preserve_subtle_monochrome_separation() {
    let elisp_form = r##"(let ((themes
       '(almost-mono-white almost-mono-black
         almost-mono-gray almost-mono-cream)))
  (mapcar
   (lambda (theme)
     (unwind-protect
         (progn
           (load-theme theme t)
           (list
            theme
            (mapcar
             (lambda (face)
               (list
                face
                (face-attribute
                 face :background nil 'default)
                (face-attribute
                 face :foreground nil 'default)
                (face-attribute
                 face :box nil 'default)))
             '(mode-line mode-line-inactive))
            (face-attribute
             'vertical-border :foreground nil 'default)
            (face-attribute
             'line-number :foreground nil 'default)))
       (when (memq theme custom-enabled-themes)
         (disable-theme theme))))
   themes))"##;
    let expect = expect![[
        r##"OK ((almost-mono-white ((mode-line "#efefef" "#000000" (:line-width -1 :color "#dddddd")) (mode-line-inactive "#ffffff" "#dddddd" (:line-width -1 :color "#dddddd"))) "#dddddd" "#dddddd") (almost-mono-black ((mode-line "#222222" "#ffffff" (:line-width -1 :color "#666666")) (mode-line-inactive "#000000" "#666666" (:line-width -1 :color "#666666"))) "#666666" "#666666") (almost-mono-gray ((mode-line "#222222" "#ffffff" (:line-width -1 :color "#666666")) (mode-line-inactive "#2b2b2b" "#666666" (:line-width -1 :color "#666666"))) "#666666" "#666666") (almost-mono-cream ((mode-line "#dbd0c5" "#000000" (:line-width -1 :color "#c4baaf")) (mode-line-inactive "#f0e5da" "#c4baaf" (:line-width -1 :color "#c4baaf"))) "#c4baaf" "#c4baaf"))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}
