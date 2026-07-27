use expect_test::expect;

use super::assert_alect_themes_parity;

#[test]
fn real_emacs_lisp_font_lock_exercises_all_six_theme_palettes_and_inversions() {
    let elisp_form = r##"
(progn
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t))
    (unwind-protect
        (mapcar
         (lambda (theme)
           (mapc #'disable-theme custom-enabled-themes)
           (load-theme theme t)
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              "(defun settle-invoice (invoice)\n\
  \"Return payment status for INVOICE.\"\n\
  ;; Preserve an auditable decision.\n\
  (let ((status :paid))\n\
    (when invoice\n\
      (message \"settled\"))\n\
    status))\n")
             (font-lock-ensure)
             (cons
              theme
              (mapcar
               (lambda (token)
                 (goto-char (point-min))
                 (search-forward token)
                 (let* ((start
                         (- (point) (length token)))
                        (face
                         (get-text-property start 'face))
                        (primary
                         (if (consp face) (car face) face)))
                   (list
                    token face
                    (and
                     (facep primary)
                     (face-attribute
                      primary :foreground nil 'default))
                    (and
                     (facep primary)
                     (face-attribute
                      primary :background nil 'default))
                    (and
                     (facep primary)
                     (face-attribute
                      primary :weight nil 'default))
                    (and
                     (facep primary)
                     (face-attribute
                      primary :slant nil 'default)))))
               '("defun" "settle-invoice"
                 "Return payment status" "Preserve"
                 "let" ":paid" "when" "message"
                 "\"settled\"")))))
         '(alect-light alect-light-alt
           alect-dark alect-dark-alt
           alect-black alect-black-alt))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK ((alect-light ("defun" font-lock-keyword-face "#2020cc" "#ded6c5" bold normal) ("settle-invoice" font-lock-function-name-face "#2c53ca" "#ded6c5" normal normal) ("Return payment status" font-lock-doc-face "#505050" "#ded6c5" normal italic) ("Preserve" font-lock-comment-face "#008b45" "#ded6c5" normal normal) ("let" font-lock-keyword-face "#2020cc" "#ded6c5" bold normal) (":paid" font-lock-builtin-face "#ba55d3" "#ded6c5" normal normal) ("when" font-lock-keyword-face "#2020cc" "#ded6c5" bold normal) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#e43838" "#ded6c5" normal normal)) (alect-light-alt ("defun" font-lock-keyword-face "#2c53ca" "#ded6c5" bold normal) ("settle-invoice" font-lock-function-name-face "#2020cc" "#ded6c5" normal normal) ("Return payment status" font-lock-doc-face "#505050" "#ded6c5" normal italic) ("Preserve" font-lock-comment-face "#1c9e28" "#ded6c5" normal normal) ("let" font-lock-keyword-face "#2c53ca" "#ded6c5" bold normal) (":paid" font-lock-builtin-face "#9400d3" "#ded6c5" normal normal) ("when" font-lock-keyword-face "#2c53ca" "#ded6c5" bold normal) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#d81212" "#ded6c5" normal normal)) (alect-dark ("defun" font-lock-keyword-face "#30a5f5" "#3f3f3f" bold normal) ("settle-invoice" font-lock-function-name-face "#94bff3" "#3f3f3f" normal normal) ("Return payment status" font-lock-doc-face "#d0bf8f" "#3f3f3f" normal italic) ("Preserve" font-lock-comment-face "#3cb370" "#3f3f3f" normal normal) ("let" font-lock-keyword-face "#30a5f5" "#3f3f3f" bold normal) (":paid" font-lock-builtin-face "#dc8cc3" "#3f3f3f" normal normal) ("when" font-lock-keyword-face "#30a5f5" "#3f3f3f" bold normal) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#fa5151" "#3f3f3f" normal normal)) (alect-dark-alt ("defun" font-lock-keyword-face "#94bff3" "#3f3f3f" bold normal) ("settle-invoice" font-lock-function-name-face "#30a5f5" "#3f3f3f" normal normal) ("Return payment status" font-lock-doc-face "#d0bf8f" "#3f3f3f" normal italic) ("Preserve" font-lock-comment-face "#32cd32" "#3f3f3f" normal normal) ("let" font-lock-keyword-face "#94bff3" "#3f3f3f" bold normal) (":paid" font-lock-builtin-face "#e81eda" "#3f3f3f" normal normal) ("when" font-lock-keyword-face "#94bff3" "#3f3f3f" bold normal) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#db4334" "#3f3f3f" normal normal)) (alect-black ("defun" font-lock-keyword-face "#1e7bda" "#000000" bold normal) ("settle-invoice" font-lock-function-name-face "#58b1f3" "#000000" normal normal) ("Return payment status" font-lock-doc-face "#ab9861" "#000000" normal italic) ("Preserve" font-lock-comment-face "#319448" "#000000" normal normal) ("let" font-lock-keyword-face "#1e7bda" "#000000" bold normal) (":paid" font-lock-builtin-face "#e353b9" "#000000" normal normal) ("when" font-lock-keyword-face "#1e7bda" "#000000" bold normal) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#ea4141" "#000000" normal normal)) (alect-black-alt ("defun" font-lock-keyword-face "#58b1f3" "#000000" bold normal) ("settle-invoice" font-lock-function-name-face "#1e7bda" "#000000" normal normal) ("Return payment status" font-lock-doc-face "#ab9861" "#000000" normal italic) ("Preserve" font-lock-comment-face "#29b029" "#000000" normal normal) ("let" font-lock-keyword-face "#58b1f3" "#000000" bold normal) (":paid" font-lock-builtin-face "#c251df" "#000000" normal normal) ("when" font-lock-keyword-face "#58b1f3" "#000000" bold normal) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#c83029" "#000000" normal normal)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn real_org_runbook_resolves_titles_todos_links_blocks_dates_and_metadata() {
    let elisp_form = r##"
(progn
  (require 'org)
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t))
    (unwind-protect
        (mapcar
         (lambda (theme)
           (mapc #'disable-theme custom-enabled-themes)
           (load-theme theme t)
           (with-temp-buffer
             (org-mode)
             (insert
              "#+title: Settlement Runbook\n\
* TODO Validate transaction\n\
DEADLINE: <2026-07-31 Fri>\n\
See [[https://example.invalid][ledger documentation]].\n\
#+begin_src emacs-lisp\n\
(message \"validate\")\n\
#+end_src\n")
             (font-lock-ensure)
             (cons
              theme
              (mapcar
               (lambda (token)
                 (goto-char (point-min))
                 (search-forward token)
                 (let* ((start
                         (- (point) (length token)))
                        (face
                         (get-text-property start 'face))
                        (primary
                         (if (consp face) (car face) face)))
                   (list
                    token face
                    (and
                     (facep primary)
                     (face-attribute
                      primary :foreground nil 'default))
                    (and
                     (facep primary)
                     (face-attribute
                      primary :background nil 'default))
                    (and
                     (facep primary)
                     (face-attribute
                      primary :inherit nil 'default)))))
               '("#+title:" "Settlement Runbook"
                 "TODO" "Validate transaction"
                 "2026-07-31"
                 "https://example.invalid"
                 "ledger documentation"
                 "#+begin_src" "message"
                 "#+end_src")))))
         '(alect-light alect-dark alect-black-alt))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK ((alect-light ("#+title:" org-document-info-keyword "#958323" "#ded6c5" nil) ("Settlement Runbook" org-document-title "#077707" "#ded6c5" alect-title) ("TODO" (org-todo org-level-1) "#f71010" "#ded6c5" nil) ("Validate transaction" org-level-1 "#2020cc" "#ded6c5" alect-title-1) ("2026-07-31" (org-date) "#0eaeae" "#ded6c5" alect-time) ("https://example.invalid" org-link "#2c53ca" "#ded6c5" link) ("ledger documentation" org-link "#2c53ca" "#ded6c5" link) ("#+begin_src" org-block-begin-line "#008b45" "#dcd2bd" org-meta-line) ("message" (org-block) "#262626" "#dcd2bd" alect-block) ("#+end_src" org-block-end-line "#008b45" "#dcd2bd" org-meta-line)) (alect-dark ("#+title:" org-document-info-keyword "#e5c900" "#3f3f3f" nil) ("Settlement Runbook" org-document-title "#099709" "#3f3f3f" alect-title) ("TODO" (org-todo org-level-1) "#ea3838" "#3f3f3f" nil) ("Validate transaction" org-level-1 "#30a5f5" "#3f3f3f" alect-title-1) ("2026-07-31" (org-date) "#8cf1f1" "#3f3f3f" alect-time) ("https://example.invalid" org-link "#94bff3" "#3f3f3f" link) ("ledger documentation" org-link "#94bff3" "#3f3f3f" link) ("#+begin_src" org-block-begin-line "#3cb370" "#464646" org-meta-line) ("message" (org-block) "#d5d2be" "#464646" alect-block) ("#+end_src" org-block-end-line "#3cb370" "#464646" org-meta-line)) (alect-black-alt ("#+title:" org-document-info-keyword "#c9d617" "#000000" nil) ("Settlement Runbook" org-document-title "#47cd57" "#000000" alect-title) ("TODO" (org-todo org-level-1) "#db4334" "#000000" nil) ("Validate transaction" org-level-1 "#58b1f3" "#000000" alect-title-1) ("2026-07-31" (org-date) "#0a7874" "#000000" alect-time) ("https://example.invalid" org-link "#1e7bda" "#000000" link) ("ledger documentation" org-link "#1e7bda" "#000000" link) ("#+begin_src" org-block-begin-line "#29b029" "#101010" org-meta-line) ("message" (org-block) "#b2af95" "#101010" alect-block) ("#+end_src" org-block-end-line "#29b029" "#101010" org-meta-line)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn real_diff_workflow_applies_file_hunk_context_change_and_refinement_faces() {
    let elisp_form = r##"
(progn
  (require 'diff-mode)
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t))
    (unwind-protect
        (mapcar
         (lambda (theme)
           (mapc #'disable-theme custom-enabled-themes)
           (load-theme theme t)
           (with-temp-buffer
             (insert
              "diff --git a/payment.el b/payment.el\n\
--- a/payment.el\n\
+++ b/payment.el\n\
@@ -1,3 +1,3 @@\n\
-(setq amount 10)\n\
+(setq amount 20)\n\
 context line\n")
             (diff-mode)
             (font-lock-ensure)
             (cons
              theme
              (mapcar
               (lambda (token)
                 (goto-char (point-min))
                 (search-forward token)
                 (let* ((start
                         (- (point) (length token)))
                        (face
                         (get-text-property start 'face))
                        (primary
                         (if (consp face) (car face) face)))
                   (list
                    token face
                    (and
                     (facep primary)
                     (face-attribute
                      primary :foreground nil 'default))
                    (and
                     (facep primary)
                     (face-attribute
                      primary :background nil 'default))
                    (and
                     (facep primary)
                     (face-attribute
                      primary :inherit nil 'default)))))
               '("diff --git" "--- a/payment.el"
                 "+++ b/payment.el" "@@ -1,3"
                 "-(setq amount" "+(setq amount"
                 "context line")))))
         '(alect-light alect-dark-alt alect-black))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK ((alect-light ("diff --git" diff-header "#0092ff" "#ded6c5" nil) ("--- a/payment.el" diff-header "#0092ff" "#ded6c5" nil) ("+++ b/payment.el" diff-header "#0092ff" "#ded6c5" nil) ("@@ -1,3" diff-hunk-header "#077707" "#ded6c5" diff-header) ("-(setq amount" diff-indicator-removed "#e43838" "#ded6c5" diff-removed) ("+(setq amount" diff-indicator-added "#1c9e28" "#ded6c5" diff-added) ("context line" diff-context "#505050" "#ded6c5" nil)) (alect-dark-alt ("diff --git" diff-header "#3390dc" "#3f3f3f" nil) ("--- a/payment.el" diff-header "#3390dc" "#3f3f3f" nil) ("+++ b/payment.el" diff-header "#3390dc" "#3f3f3f" nil) ("@@ -1,3" diff-hunk-header "#8ce096" "#3f3f3f" diff-header) ("-(setq amount" diff-indicator-removed "#db4334" "#3f3f3f" diff-removed) ("+(setq amount" diff-indicator-added "#3cb370" "#3f3f3f" diff-added) ("context line" diff-context "#d0bf8f" "#3f3f3f" nil)) (alect-black ("diff --git" diff-header "#8cb7ff" "#000000" nil) ("--- a/payment.el" diff-header "#8cb7ff" "#000000" nil) ("+++ b/payment.el" diff-header "#8cb7ff" "#000000" nil) ("@@ -1,3" diff-hunk-header "#078607" "#000000" diff-header) ("-(setq amount" diff-indicator-removed "#ea4141" "#000000" diff-removed) ("+(setq amount" diff-indicator-added "#29b029" "#000000" diff-added) ("context line" diff-context "#ab9861" "#000000" nil)))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn real_compilation_buffer_preserves_diagnostic_faces_and_location_metadata() {
    let elisp_form = r##"
(progn
  (require 'compile)
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t))
    (unwind-protect
        (progn
          (load-theme 'alect-dark t)
          (with-temp-buffer
            (compilation-mode)
            (let ((inhibit-read-only t))
              (insert
               "Checking settlement\n\
src/payment.rs:3:5: error: invalid datum\n\
src/payment.rs:8:2: warning: unused receipt\n"))
            (font-lock-ensure)
            (mapcar
             (lambda (token)
               (goto-char (point-min))
               (search-forward token)
               (let* ((start
                       (- (point) (length token)))
                      (face
                       (get-text-property start 'face))
                      (message
                       (get-text-property
                        start 'compilation-message))
                      (primary
                       (if (consp face) (car face) face)))
                 (list
                  token face
                  (and message t)
                  (and
                   (facep primary)
                   (face-attribute
                    primary :foreground nil 'default))
                  (and
                   (facep primary)
                   (face-attribute
                    primary :inherit nil 'default))
                  (and
                   (facep primary)
                   (face-attribute
                    primary :weight nil 'default)))))
             '("src/payment.rs:3:5" "error"
               "src/payment.rs:8:2" "warning"))))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK (("src/payment.rs:3:5" font-lock-function-name-face t "#94bff3" nil normal) ("error" nil t nil nil nil) ("src/payment.rs:8:2" font-lock-function-name-face t "#94bff3" nil normal) ("warning" nil t nil nil nil))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn real_dired_listing_applies_directory_symlink_mark_and_permission_faces() {
    let elisp_form = r##"
(progn
  (require 'dired)
  (mapc #'disable-theme custom-enabled-themes)
  (let* ((alect-display-class t)
         (root (make-temp-file "alect-dired-" t))
         (directory
          (expand-file-name "contracts" root))
         (file
          (expand-file-name "payment.el" root))
         (link
          (expand-file-name "payment-link.el" root))
         buffer)
    (unwind-protect
        (progn
          (make-directory directory)
          (with-temp-file file
            (insert "(message \"settled\")\n"))
          (make-symbolic-link file link)
          (load-theme 'alect-light t)
          (setq buffer
                (dired-noselect
                 (file-name-as-directory root)
                 "-al"))
          (with-current-buffer buffer
            (font-lock-ensure)
            (goto-char (point-min))
            (dired-goto-file file)
            (dired-mark 1)
            (font-lock-flush)
            (font-lock-ensure)
            (mapcar
             (lambda (token)
               (goto-char (point-min))
               (search-forward token)
               (let* ((start
                       (- (point) (length token)))
                      (face
                       (get-text-property start 'face))
                      (primary
                       (if (consp face) (car face) face)))
                 (list
                  token face
                  (and
                   (facep primary)
                   (face-attribute
                    primary :foreground nil 'default))
                  (and
                   (facep primary)
                   (face-attribute
                    primary :background nil 'default))
                  (and
                   (facep primary)
                   (face-attribute
                    primary :inherit nil 'default)))))
             '("contracts" "payment.el"
               "payment-link.el"))))
      (when (buffer-live-p buffer)
        (kill-buffer buffer))
      (mapc #'disable-theme custom-enabled-themes)
      (delete-directory root t))))
"##;
    let expect = expect![[
        r##"OK (("contracts" dired-directory "#2c53ca" "#ded6c5" font-lock-function-name-face) ("payment.el" default "#262626" "#ded6c5" nil) ("payment-link.el" dired-symlink "#259ea2" "#ded6c5" font-lock-constant-face))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}

#[test]
fn themed_ansi_vector_drives_real_terminal_escape_rendering_and_text_properties() {
    let elisp_form = r##"
(progn
  (require 'ansi-color)
  (mapc #'disable-theme custom-enabled-themes)
  (let ((alect-display-class t))
    (unwind-protect
        (mapcar
         (lambda (theme)
           (mapc #'disable-theme custom-enabled-themes)
           (load-theme theme t)
           (let ((rendered
                  (ansi-color-apply
                   "\e[31mred\e[0m \
\e[32mgreen\e[0m \
\e[34mblue\e[0m \
\e[35mmagenta\e[0m")))
             (list
              theme
              (append ansi-color-names-vector nil)
              (substring-no-properties rendered)
              (mapcar
               (lambda (index)
                 (list
                  index
                  (aref rendered index)
                  (text-properties-at index rendered)))
               '(0 2 3 4 8 9 10 13 14 15 20 21)))))
         '(alect-light alect-light-alt
           alect-dark alect-dark-alt
           alect-black alect-black-alt))
      (mapc #'disable-theme custom-enabled-themes))))
"##;
    let expect = expect![[
        r##"OK ((alect-light ("#ded6c5" "#f71010" "#028902" "#da7710" "#1111ff" "#a020f0" "#358d8d" "#262626") "red green blue magenta" ((0 114 #1=(font-lock-face (:foreground "red3"))) (2 100 #1#) (3 32 nil) (4 103 #2=(font-lock-face (:foreground "green3"))) (8 110 #2#) (9 32 nil) (10 98 #3=(font-lock-face (:foreground "blue2"))) (13 101 #3#) (14 32 nil) (15 109 #4=(font-lock-face (:foreground "magenta3"))) (20 116 #4#) (21 97 #4#))) (alect-light-alt ("#ded6c5" "#f71010" "#028902" "#da7710" "#1111ff" "#a020f0" "#358d8d" "#262626") "red green blue magenta" ((0 114 #5=(font-lock-face (:foreground "red3"))) (2 100 #5#) (3 32 nil) (4 103 #6=(font-lock-face (:foreground "green3"))) (8 110 #6#) (9 32 nil) (10 98 #7=(font-lock-face (:foreground "blue2"))) (13 101 #7#) (14 32 nil) (15 109 #8=(font-lock-face (:foreground "magenta3"))) (20 116 #8#) (21 97 #8#))) (alect-dark ("#3f3f3f" "#ea3838" "#7fb07f" "#fe8b04" "#62b6ea" "#e353b9" "#1fb3b3" "#d5d2be") "red green blue magenta" ((0 114 #9=(font-lock-face (:foreground "red3"))) (2 100 #9#) (3 32 nil) (4 103 #10=(font-lock-face (:foreground "green3"))) (8 110 #10#) (9 32 nil) (10 98 #11=(font-lock-face (:foreground "blue2"))) (13 101 #11#) (14 32 nil) (15 109 #12=(font-lock-face (:foreground "magenta3"))) (20 116 #12#) (21 97 #12#))) (alect-dark-alt ("#3f3f3f" "#ea3838" "#7fb07f" "#fe8b04" "#62b6ea" "#e353b9" "#1fb3b3" "#d5d2be") "red green blue magenta" ((0 114 #13=(font-lock-face (:foreground "red3"))) (2 100 #13#) (3 32 nil) (4 103 #14=(font-lock-face (:foreground "green3"))) (8 110 #14#) (9 32 nil) (10 98 #15=(font-lock-face (:foreground "blue2"))) (13 101 #15#) (14 32 nil) (15 109 #16=(font-lock-face (:foreground "magenta3"))) (20 116 #16#) (21 97 #16#))) (alect-black ("#000000" "#db4334" "#60a060" "#dc7700" "#00a2f5" "#da26ce" "#1ba1a1" "#b2af95") "red green blue magenta" ((0 114 #17=(font-lock-face (:foreground "red3"))) (2 100 #17#) (3 32 nil) (4 103 #18=(font-lock-face (:foreground "green3"))) (8 110 #18#) (9 32 nil) (10 98 #19=(font-lock-face (:foreground "blue2"))) (13 101 #19#) (14 32 nil) (15 109 #20=(font-lock-face (:foreground "magenta3"))) (20 116 #20#) (21 97 #20#))) (alect-black-alt ("#000000" "#db4334" "#60a060" "#dc7700" "#00a2f5" "#da26ce" "#1ba1a1" "#b2af95") "red green blue magenta" ((0 114 #21=(font-lock-face (:foreground "red3"))) (2 100 #21#) (3 32 nil) (4 103 #22=(font-lock-face (:foreground "green3"))) (8 110 #22#) (9 32 nil) (10 98 #23=(font-lock-face (:foreground "blue2"))) (13 101 #23#) (14 32 nil) (15 109 #24=(font-lock-face (:foreground "magenta3"))) (20 116 #24#) (21 97 #24#))))"##
    ]];
    assert_alect_themes_parity(elisp_form, expect);
}
