use expect_test::expect;

use super::assert_almost_mono_themes_parity;

#[test]
fn cream_theme_styles_a_real_org_dashboard_with_tasks_properties_table_and_indent() {
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
          (let ((describe
                 (lambda (token)
                   (goto-char (point-min))
                   (search-forward token)
                   (let* ((position
                           (- (point) (length token)))
                          (face
                           (get-text-property
                            position 'face))
                          (primary
                           (if (listp face)
                               (car face)
                             face)))
                     (list
                      token
                      face
                      (and
                       primary
                       (face-attribute
                        primary :foreground
                        nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :weight
                        nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :slant
                        nil 'default)))))))
            (list
             major-mode
             (copy-sequence custom-enabled-themes)
             (face-attribute
              'default :background nil 'default)
             (face-attribute
              'org-hide :foreground nil 'default)
             (mapcar
              describe
              '("#+title:" "Release dashboard" "TODO"
                ":PROPERTIES:" ":Owner:" "Ada" "DONE"
                "Archive notes" "| Item | State |"))))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (org-mode (almost-mono-cream) "#f0e5da" "#f0e5da" (("#+title:" org-document-info-keyword "#000000" normal normal) ("Release dashboard" org-document-title "#000000" normal normal) ("TODO" (org-todo org-level-1) "#fda50f" bold normal) (":PROPERTIES:" org-drawer "#7d7165" normal normal) (":Owner:" org-special-keyword "#7d7165" bold normal) ("Ada" org-property-value "#7d7165" normal italic) ("DONE" (org-done org-level-1) "#00ff00" bold normal) ("Archive notes" (org-headline-done org-level-1) "#000000" bold normal) ("| Item | State |" org-table "#7d7165" normal normal)))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn gray_theme_styles_a_real_unified_diff_and_change_status_indicators() {
    let elisp_form = r##"(let ((theme 'almost-mono-gray)
      (indicator-faces
       '(diff-hl-change diff-hl-insert diff-hl-delete)))
  (unwind-protect
      (progn
        (dolist (face indicator-faces)
          (unless (facep face)
            (make-face face)))
        (load-theme theme t)
        (with-temp-buffer
          (diff-mode)
          (insert
           "diff --git a/src/lib.rs b/src/lib.rs\n"
           "index 56a6051..f47c63d 100644\n"
           "--- a/src/lib.rs\n"
           "+++ b/src/lib.rs\n"
           "@@ -1,3 +1,3 @@\n"
           " fn value() -> i32 {\n"
           "-    1\n"
           "+    2\n"
           " }\n")
          (font-lock-ensure)
          (goto-char (point-max))
          (dolist
              (entry
               '(("~ modified" diff-hl-change)
                 ("+ added" diff-hl-insert)
                 ("- deleted" diff-hl-delete)))
            (let ((start (point)))
              (insert (car entry) "\n")
              (put-text-property
               start (1- (point)) 'face (cadr entry))))
          (let ((describe
                 (lambda (token)
                   (goto-char (point-min))
                   (search-forward token)
                   (let* ((position
                           (- (point) (length token)))
                          (face
                           (get-text-property
                            position 'face))
                          (primary
                           (if (listp face)
                               (car face)
                             face)))
                     (list
                      token
                      face
                      (and
                       primary
                       (face-attribute
                        primary :background
                        nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :foreground
                        nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :weight
                        nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :slant
                        nil 'default)))))))
            (list
             major-mode
             (copy-sequence custom-enabled-themes)
             (face-attribute
              'default :background nil 'default)
             (mapcar
              describe
              '("diff --git a/src/lib.rs b/src/lib.rs"
                "--- a/src/lib.rs"
                "@@ -1,3 +1,3 @@"
                "-    1"
                "+    2"
                "~ modified"
                "+ added"
                "- deleted"))))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (diff-mode (almost-mono-gray) "#2b2b2b" (("diff --git a/src/lib.rs b/src/lib.rs" diff-header "#2b2b2b" "#ffffff" bold normal) ("--- a/src/lib.rs" diff-header "#2b2b2b" "#ffffff" bold normal) ("@@ -1,3 +1,3 @@" diff-hunk-header "#2b2b2b" "#ffffff" bold normal) ("-    1" diff-indicator-removed "#2b2b2b" "#ffffff" normal normal) ("+    2" diff-indicator-added "#2b2b2b" "#ffffff" normal normal) ("~ modified" diff-hl-change "#fda50f" "#fda50f" normal normal) ("+ added" diff-hl-insert "#00ff00" "#00ff00" normal normal) ("- deleted" diff-hl-delete "#ff0000" "#ff0000" normal normal)))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn white_theme_resolves_completion_search_selection_and_navigation_ui_faces() {
    let elisp_form = r##"(let ((theme 'almost-mono-white)
      (extension-faces
       '(vertico-current
         completions-common-part
         orderless-match-face-0
         orderless-match-face-2)))
  (unwind-protect
      (progn
        (dolist (face extension-faces)
          (unless (facep face)
            (make-face face)))
        (load-theme theme t)
        (with-temp-buffer
          (completion-list-mode)
          (let ((inhibit-read-only t))
            (dolist
                (entry
                 '(("Choose file: " minibuffer-prompt)
                   ("src/lib.rs" vertico-current)
                   ("common-prefix" completions-common-part)
                   ("orderless-zero" orderless-match-face-0)
                   ("orderless-two" orderless-match-face-2)
                   ("selected text" region)
                   ("active search" isearch)
                   ("lazy match" lazy-highlight)
                   ("matching pair" show-paren-match)
                   ("broken pair" show-paren-mismatch)
                   ("documentation" link)))
              (insert
               (propertize
                (car entry) 'face (cadr entry))
               "\n")))
          (let ((describe
                 (lambda (token)
                   (goto-char (point-min))
                   (search-forward token)
                   (let* ((position
                           (- (point) (length token)))
                          (face
                           (get-text-property position 'face))
                          (primary
                           (if (listp face)
                               (car face)
                             face)))
                     (list
                      token
                      face
                      (and
                       primary
                       (face-attribute
                        primary :background nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :foreground nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :weight nil 'default))
                      (and
                       primary
                       (face-attribute
                        primary :underline nil 'default)))))))
            (list
             major-mode
             (copy-sequence custom-enabled-themes)
             (mapcar
              describe
              '("Choose file: "
                "src/lib.rs"
                "common-prefix"
                "orderless-zero"
                "orderless-two"
                "selected text"
                "active search"
                "lazy match"
                "matching pair"
                "broken pair"
                "documentation"))
             (list
              (face-attribute
               'mode-line :background nil 'default)
              (face-attribute
               'mode-line :foreground nil 'default)
              (copy-tree
               (face-attribute
                'mode-line :box nil 'default)))
             (list
              (face-attribute
               'mode-line-inactive
               :background nil 'default)
              (face-attribute
               'mode-line-inactive
               :foreground nil 'default)
              (copy-tree
               (face-attribute
                'mode-line-inactive
                :box nil 'default)))
             (face-attribute
              'vertical-border
              :foreground nil 'default)
             (face-attribute
              'line-number
              :foreground nil 'default)))))
      (when (memq theme custom-enabled-themes)
        (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (completion-list-mode (almost-mono-white) (("Choose file: " minibuffer-prompt "#ffffff" "#000000" bold nil) ("src/lib.rs" vertico-current "#fda50f" "#000000" bold nil) ("common-prefix" completions-common-part "#ffffff" "#000000" bold t) ("orderless-zero" orderless-match-face-0 "#ffffff" "#000000" bold t) ("orderless-two" orderless-match-face-2 "#ffffff" "#000000" bold t) ("selected text" region "#fda50f" "#000000" normal nil) ("active search" isearch "#888888" "#000000" bold nil) ("lazy match" lazy-highlight "#dddddd" "#000000" normal nil) ("matching pair" show-paren-match "#ffffff" "#00ff00" bold nil) ("broken pair" show-paren-mismatch "#ffffff" "#ff0000" bold nil) ("documentation" link "#ffffff" "#000000" normal t)) ("#efefef" "#000000" (:line-width -1 :color "#dddddd")) ("#ffffff" "#dddddd" (:line-width -1 :color "#dddddd")) "#dddddd" "#dddddd")"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}
