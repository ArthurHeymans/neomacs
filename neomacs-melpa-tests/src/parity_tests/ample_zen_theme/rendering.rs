use expect_test::expect;

use super::assert_ample_zen_theme_parity;

#[test]
fn theme_styles_real_emacs_lisp_tokens_with_muted_semantic_palette() {
    let elisp_form = r##"(let ((theme 'ample-zen))
  (unwind-protect
      (progn
        (load-theme theme t)
        (with-temp-buffer
          (emacs-lisp-mode)
          (insert
           ";;; Build a release report\n"
           "(defconst release-limit 24)\n"
           "(defun build-release (channel &optional dry-run)\n"
           "  \"Build CHANNEL unless DRY-RUN.\"\n"
           "  ;; Keep the operator informed.\n"
           "  (let ((message \"ready\"))\n"
           "    (list channel dry-run message release-limit)))\n")
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
           '("Build a release report" "defconst"
             "release-limit" "24" "defun" "build-release"
             "channel" "Build CHANNEL" "Keep the operator"
             "\"ready\"" "list"))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (("Build a release report" font-lock-comment-face "#6aaf50" normal normal) ("defconst" font-lock-keyword-face "#7d7c61" bold normal) ("release-limit" font-lock-variable-name-face "#fb8512" normal normal) ("24" nil nil nil nil) ("defun" font-lock-keyword-face "#7d7c61" bold normal) ("build-release" font-lock-function-name-face "#9b55c3" normal normal) ("channel" nil nil nil nil) ("Build CHANNEL" font-lock-doc-face "#6a9550" normal normal) ("Keep the operator" font-lock-comment-face "#6aaf50" normal normal) ("\"ready\"" font-lock-string-face "#CC5542" normal normal) ("list" nil nil nil nil))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn theme_styles_real_ruby_class_methods_constants_symbols_and_comments() {
    let elisp_form = r##"(let ((theme 'ample-zen))
  (unwind-protect
      (progn
        (load-theme theme t)
        (with-temp-buffer
          (ruby-mode)
          (insert
           "# Build a release artifact\n"
           "class ReleaseBuilder\n"
           "  DEFAULT_CHANNEL = \"stable\"\n"
           "  def build(channel: DEFAULT_CHANNEL)\n"
           "    puts :ready if channel\n"
           "  end\n"
           "end\n")
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
                      primary :weight nil 'default)))))
           '("Build a release artifact" "class"
             "ReleaseBuilder" "DEFAULT_CHANNEL"
             "\"stable\"" "def" "build" ":ready" "if" "end"))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (("Build a release artifact" font-lock-comment-face "#6aaf50" normal) ("class" font-lock-keyword-face "#7d7c61" bold) ("ReleaseBuilder" font-lock-type-face "#528fd1" normal) ("DEFAULT_CHANNEL" font-lock-type-face "#528fd1" normal) ("\"stable\"" font-lock-string-face "#CC5542" normal) ("def" font-lock-type-face "#528fd1" normal) ("build" font-lock-comment-face "#6aaf50" normal) (":ready" font-lock-constant-face "#6a7550" normal) ("if" font-lock-comment-face "#6aaf50" normal) ("end" font-lock-keyword-face "#7d7c61" bold))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn theme_styles_real_shell_commands_variables_strings_and_control_flow() {
    let elisp_form = r##"(let ((theme 'ample-zen))
  (unwind-protect
      (progn
        (load-theme theme t)
        (with-temp-buffer
          (sh-mode)
          (insert
           "#!/usr/bin/env bash\n"
           "# Build and publish the release\n"
           "channel=\"stable\"\n"
           "if [[ -n \"$channel\" ]]; then\n"
           "  printf 'publishing %s\\n' \"$channel\"\n"
           "else\n"
           "  exit 1\n"
           "fi\n")
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
                      primary :weight nil 'default)))))
           '("#!/usr/bin/env bash" "Build and publish"
             "channel" "\"stable\"" "if" "-n" "then"
             "printf" "'publishing %s\\n'" "else" "exit" "fi"))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (("#!/usr/bin/env bash" font-lock-comment-delimiter-face "#6abd50" normal) ("Build and publish" font-lock-comment-face "#6aaf50" normal) ("channel" font-lock-variable-name-face "#fb8512" normal) ("\"stable\"" font-lock-string-face "#CC5542" normal) ("if" font-lock-keyword-face "#7d7c61" bold) ("-n" nil nil nil) ("then" font-lock-keyword-face "#7d7c61" bold) ("printf" font-lock-builtin-face "#bdbdb3" bold) ("'publishing %s\\n'" font-lock-string-face "#CC5542" normal) ("else" font-lock-keyword-face "#7d7c61" bold) ("exit" font-lock-keyword-face "#7d7c61" bold) ("fi" font-lock-keyword-face "#7d7c61" bold))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn theme_styles_real_org_planning_headings_tasks_dates_properties_and_tables() {
    let elisp_form = r##"(let ((theme 'ample-zen))
  (unwind-protect
      (progn
        (require 'org)
        (load-theme theme t)
        (with-temp-buffer
          (org-mode)
          (insert
           "#+title: Release dashboard\n"
           "* TODO Publish runtime :release:\n"
           "SCHEDULED: <2026-07-28 Tue>\n"
           ":PROPERTIES:\n"
           ":Owner: Ada\n"
           ":END:\n"
           "** DONE Archive artifacts\n"
           "| Artifact | State |\n"
           "| runtime  | ready |\n")
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
                      primary :background nil 'default))
                (and primary
                     (face-attribute
                      primary :weight nil 'default)))))
           '("#+title:" "Release dashboard" "TODO"
             "Publish runtime" ":release:" "SCHEDULED:"
             "2026-07-28 Tue" ":PROPERTIES:" ":Owner:"
             "Ada" "DONE" "Archive artifacts"
             "| Artifact | State |"))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (("#+title:" org-document-info-keyword "#bdbdb3" "#212121" normal) ("Release dashboard" org-document-title "#bdbdb3" "#212121" bold) ("TODO" (org-todo org-level-1) "#CC5542" "#212121" bold) ("Publish runtime" org-level-1 "#fb8512" "#212121" normal) (":release:" (org-tag org-level-1) "#bdbdb3" "#212121" bold) ("SCHEDULED:" org-special-keyword "#c9c9c9" "#212121" normal) ("2026-07-28 Tue" (org-date) "#5180b3" "#212121" normal) (":PROPERTIES:" org-drawer "#bdbdb3" "#212121" bold) (":Owner:" org-special-keyword "#c9c9c9" "#212121" normal) ("Ada" org-property-value "#bdbdb3" "#212121" normal) ("DONE" (org-done org-level-2) "#6a8550" "#212121" bold) ("Archive artifacts" (org-headline-done org-level-2) "#6a8550" "#212121" normal) ("| Artifact | State |" org-table "#6a9550" "#212121" normal))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn theme_styles_real_unified_diff_headers_hunks_additions_removals_and_context() {
    let elisp_form = r##"(let ((theme 'ample-zen))
  (unwind-protect
      (progn
        (require 'diff-mode)
        (load-theme theme t)
        (with-temp-buffer
          (diff-mode)
          (insert
           "diff --git a/config.el b/config.el\n"
           "index 1111111..2222222 100644\n"
           "--- a/config.el\n"
           "+++ b/config.el\n"
           "@@ -1,3 +1,3 @@\n"
           " (setq channel \"stable\")\n"
           "-(setq workers 12)\n"
           "+(setq workers 24)\n")
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
                      primary :background nil 'default))
                (and primary
                     (face-attribute
                      primary :weight nil 'default)))))
           '("diff --git" "index 1111111"
             "--- a/config.el" "+++ b/config.el"
             "@@ -1,3 +1,3 @@" "channel"
             "-(setq workers 12)" "+(setq workers 24)"))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (("diff --git" diff-header "#212121" "#bdbdb3" normal) ("index 1111111" diff-header "#212121" "#bdbdb3" normal) ("--- a/config.el" diff-header "#212121" "#bdbdb3" normal) ("+++ b/config.el" diff-header "#212121" "#bdbdb3" normal) ("@@ -1,3 +1,3 @@" diff-hunk-header "#212121" "#bdbdb3" normal) ("channel" diff-context "#bdbdb3" "#212121" normal) ("-(setq workers 12)" diff-indicator-removed "#ff5542" "#212121" normal) ("+(setq workers 24)" diff-indicator-added "#6abd50" "#212121" normal))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn optional_ecosystem_faces_receive_effective_completion_vcs_mail_and_web_attributes() {
    let elisp_form = r##"(let ((theme 'ample-zen)
      (faces
       '(ac-candidate-face ac-selection-face popup-tip-face
         git-gutter:added git-gutter:deleted git-gutter:modified
         magit-section-title magit-branch magit-item-highlight
         powerline-active1 powerline-inactive1
         mu4e-cited-1-face mu4e-trashed-face
         web-mode-html-tag-face web-mode-html-attr-name-face
         web-mode-server-background-face
         rainbow-delimiters-depth-1-face
         rainbow-delimiters-depth-6-face
         rainbow-delimiters-depth-12-face)))
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
             face :strike-through nil 'default)
            (face-attribute
             face :inherit nil 'default)))
         faces))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK ((ac-candidate-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#000000" "#4c4c4c" normal normal nil nil) (ac-selection-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#4C7073" "#bdbdb3" normal normal nil nil) (popup-tip-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#cc8512" "#000000" normal normal nil nil) (git-gutter:added [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#6aaf50" bold normal nil nil) (git-gutter:deleted [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#CC5542" bold normal nil nil) (git-gutter:modified [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#baba36" bold normal nil nil) (magit-section-title [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#7d7c61" bold normal nil nil) (magit-branch [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#fb8512" bold normal nil nil) (magit-item-highlight [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#141414" "#bdbdb3" normal normal nil nil) (powerline-active1 [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#2e2e2e" "#bdbdb3" normal normal nil mode-line) (powerline-inactive1 [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#141414" "#9b9b9b" light normal nil mode-line-inactive) (mu4e-cited-1-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#5180b3" normal italic nil nil) (mu4e-trashed-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#000000" normal normal t nil) (web-mode-html-tag-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#9b55c3" normal normal nil nil) (web-mode-html-attr-name-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#fb8512" normal normal nil nil) (web-mode-server-background-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#bdbdb3" normal normal nil nil) (rainbow-delimiters-depth-1-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#bdbdb3" normal normal nil nil) (rainbow-delimiters-depth-6-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#6380b3" normal normal nil nil) (rainbow-delimiters-depth-12-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "#212121" "#366060" normal normal nil nil))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn available_diagnostic_faces_resolve_current_display_fallbacks_and_status_colors() {
    let elisp_form = r##"(let ((theme 'ample-zen)
      (faces
       '(flycheck-error flycheck-warning
         flycheck-fringe-error flycheck-fringe-warning
         flymake-errline flymake-warnline flymake-infoline
         flyspell-duplicate flyspell-incorrect)))
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
            (face-attribute
             face :background nil 'default)
            (face-attribute
             face :foreground nil 'default)
            (face-attribute
             face :weight nil 'default)
            (copy-tree
             (face-attribute
              face :underline nil 'default))
            (face-attribute
             face :inherit nil 'default)))
         faces))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK ((flycheck-error "#212121" "#dd5542" bold t nil) (flycheck-warning "#212121" "#fb8512" bold t nil) (flycheck-fringe-error "#212121" "#dd5542" bold nil nil) (flycheck-fringe-warning "#212121" "#fb8512" bold nil nil) (flymake-errline "#212121" "#dd5542" bold t nil) (flymake-warnline "#212121" "#fb8512" bold t nil) (flymake-infoline "#212121" "#6abd50" bold t nil) (flyspell-duplicate "#212121" "#fb8512" bold t nil) (flyspell-incorrect "#212121" "#dd5542" bold t nil))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}
